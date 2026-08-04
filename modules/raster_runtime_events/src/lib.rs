// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(
    clippy::mutable_key_type,
    clippy::for_kv_map,
    clippy::new_without_default
)]
use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
};

use raster_runtime_utils::{
    error::ErrorExtensions, module::ModuleInfo, object::ObjectExt, result::ResultExt,
};
use rquickjs::{
    class::{JsClass, Trace, Tracer},
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt, Rest, This},
    CatchResultExt, Class, Ctx, Function, JsLifetime, Object, Result, String as JsString, Symbol,
    Value,
};
use tracing::trace;

use self::{custom_event::CustomEvent, event::Event, event_target::EventTarget};

pub mod custom_event;
pub mod event;
pub mod event_target;

#[derive(Clone, Debug)]
pub enum EventKey<'js> {
    Symbol(Symbol<'js>),
    String(Rc<str>),
}

impl<'js> EventKey<'js> {
    fn from_value(ctx: &Ctx, value: Value<'js>) -> Result<Self> {
        if value.is_string() {
            let key: String = value.get()?;
            Ok(EventKey::String(key.into()))
        } else {
            let sym = value.into_symbol().ok_or("Not a symbol").or_throw(ctx)?;
            Ok(EventKey::Symbol(sym))
        }
    }
}

impl Eq for EventKey<'_> {}

impl PartialEq for EventKey<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (EventKey::Symbol(symbol1), EventKey::Symbol(symbol2)) => symbol1 == symbol2,
            (EventKey::String(str1), EventKey::String(str2)) => str1 == str2,
            _ => false,
        }
    }
}

/// Internal listener identity for precise once/remove (not exposed to JS).
static NEXT_LISTENER_ID: AtomicU64 = AtomicU64::new(1);

pub struct EventItem<'js> {
    id: u64,
    callback: Function<'js>,
    once: bool,
    /// Shared with emit snapshots so recursive `emit` can mark a once listener
    /// as fired and outer snapshots skip a second call (Node onceWrapper).
    fired: Rc<Cell<bool>>,
}

pub type EventList<'js> = Vec<(EventKey<'js>, Vec<EventItem<'js>>)>;
pub type Events<'js> = Arc<RwLock<EventList<'js>>>;

fn is_remove_listener_key(key: &EventKey<'_>) -> bool {
    matches!(key, EventKey::String(s) if s.as_ref() == "removeListener")
}

fn event_key_to_value<'js>(ctx: &Ctx<'js>, key: &EventKey<'js>) -> Result<Value<'js>> {
    match key {
        EventKey::String(s) => Ok(JsString::from_str(ctx.clone(), s)?.into_value()),
        EventKey::Symbol(sym) => Ok(sym.clone().into_value()),
    }
}

/// Remove one listener by internal ID. Returns true if a listener was removed.
/// When `emit_meta` is true and removal succeeds, emits `removeListener`.
fn remove_listener_by_id<'js>(
    ctx: &Ctx<'js>,
    this: &This<Object<'js>>,
    key: &EventKey<'js>,
    id: u64,
    emit_meta: bool,
) -> Result<bool> {
    let events = resolve_events(ctx, &this.0)?;
    let mut events = events.write().or_throw(ctx)?;
    let Some(index) = events.iter_mut().position(|(k, _)| k == key) else {
        return Ok(false);
    };
    let items = &mut events[index].1;
    let Some(pos) = items.iter().position(|item| item.id == id) else {
        return Ok(false);
    };
    let removed = items.remove(pos);
    let now_empty = items.is_empty();
    if now_empty {
        events.remove(index);
    }
    drop(events);

    if emit_meta {
        let event_val = event_key_to_value(ctx, key)?;
        EventEmitter::do_emit(
            to_event(ctx, "removeListener")?,
            This(this.0.clone()),
            ctx,
            Rest(vec![event_val, removed.callback.into_value()]),
            false,
        )?;
    }

    if now_empty {
        if let Some(class) = Class::<EventEmitter>::from_object(&this.0) {
            class.borrow_mut().on_event_changed(key.clone(), false)?;
        } else if let Some(class) = Class::<EventTarget>::from_object(&this.0) {
            class.borrow_mut().on_event_changed(key.clone(), false)?;
        }
    }
    Ok(true)
}

/// Reverse-order remove-all for one event key (Node `removeAllListeners(event)`).
fn remove_all_for_key<'js>(
    ctx: &Ctx<'js>,
    this: &This<Object<'js>>,
    key: &EventKey<'js>,
) -> Result<()> {
    let events = resolve_events(ctx, &this.0)?;
    let events_read = events.read().or_throw(ctx)?;
    let ids: Vec<u64> = events_read
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, items)| items.iter().rev().map(|item| item.id).collect())
        .unwrap_or_default();
    drop(events_read);

    for id in ids {
        // emit_meta per actual removal; missing ID is a no-op (reentrancy).
        let _ = remove_listener_by_id(ctx, this, key, id, true)?;
    }
    Ok(())
}

/// Get the hidden symbol used to store the event list on JS objects.
fn events_symbol<'js>(ctx: &Ctx<'js>) -> Result<Symbol<'js>> {
    Symbol::new_global(ctx.clone(), "__ee")
}

/// Convert a Class into an Object for use with Emitter methods.
fn class_to_obj<'js, C: JsClass<'js>>(class: Class<'js, C>) -> Result<Object<'js>> {
    Object::from_value(class.into_value())
}

/// Resolve the event list from a JS object. For native Emitter classes,
/// reads from the native struct. For plain JS objects (e.g. stream.js Readable),
/// lazily creates and stores a native EventEmitter as a hidden property.
#[allow(clippy::arc_with_non_send_sync)]
pub fn resolve_events<'js>(ctx: &Ctx<'js>, obj: &Object<'js>) -> Result<Events<'js>> {
    // Try native EventEmitter / EventTarget first
    if let Some(class) = Class::<EventEmitter>::from_object(obj) {
        return Ok(class.borrow().events.clone());
    }
    if let Some(class) = Class::<EventTarget>::from_object(obj) {
        return Ok(class.borrow().events.clone());
    }
    let sym = events_symbol(ctx)?;
    // Check for hidden property
    if let Some(ee) = obj.get::<_, Option<Class<'js, EventEmitter<'js>>>>(sym.clone())? {
        return Ok(ee.borrow().events.clone());
    }
    // Create and store a new one
    let events: Events<'js> = Arc::new(RwLock::new(Vec::new()));
    let ee = Class::instance(
        ctx.clone(),
        EventEmitter {
            events: events.clone(),
        },
    )?;
    obj.set(sym, ee)?;
    Ok(events)
}

#[rquickjs::class]
#[derive(Clone)]
pub struct EventEmitter<'js> {
    pub events: Events<'js>,
}

unsafe impl<'js> JsLifetime<'js> for EventEmitter<'js> {
    type Changed<'to> = EventEmitter<'to>;
}

impl<'js> Emitter<'js> for EventEmitter<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.events.clone()
    }
}

impl<'js> Trace<'js> for EventEmitter<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.trace_event_emitter(tracer);
    }
}

#[rquickjs::methods]
impl<'js> EventEmitter<'js> {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            #[allow(clippy::arc_with_non_send_sync)]
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

pub trait EmitError<'js> {
    fn emit_error<C>(self, id: &'static str, ctx: &Ctx<'js>, this: Class<'js, C>) -> Result<bool>
    where
        C: Emitter<'js>;
}

impl<'js, T> EmitError<'js> for Result<T> {
    fn emit_error<C>(self, id: &'static str, ctx: &Ctx<'js>, this: Class<'js, C>) -> Result<bool>
    where
        C: Emitter<'js>,
    {
        if let Err(err) = self.catch(ctx) {
            trace!("Error caught in: {}", id);
            if this.borrow().has_listener_str("error") {
                let error_value = err.into_value(ctx)?;
                C::emit_str(this, ctx, "error", vec![error_value], false)?;
                return Ok(true);
            }
            return Err(err.throw(ctx));
        }
        Ok(false)
    }
}

pub trait Emitter<'js>
where
    Self: JsClass<'js> + Sized + 'js,
{
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>>;

    fn on_event_changed(&mut self, _event: EventKey<'js>, _added: bool) -> Result<()> {
        Ok(())
    }

    /// Resolve the event list from a `this` object. For native classes,
    /// extracts from the class data. For plain JS objects, uses the hidden property.
    fn resolve_events_from(ctx: &Ctx<'js>, this: &Object<'js>) -> Result<Events<'js>> {
        if let Some(class) = Class::<Self>::from_object(this) {
            return Ok(class.borrow().get_event_list());
        }
        resolve_events(ctx, this)
    }

    fn add_event_emitter_prototype(ctx: &Ctx<'js>) -> Result<Object<'js>> {
        let proto = Class::<Self>::prototype(ctx)?
            .or_throw_msg(ctx, "Prototype for EventEmitter not found")?;

        let on = Function::new(ctx.clone(), Self::on)?;
        let off = Function::new(ctx.clone(), Self::remove_event_listener)?;

        proto.set("once", Func::from(Self::once))?;
        proto.set("on", on.clone())?;
        proto.set("emit", Func::from(Self::emit))?;
        proto.set("prependListener", Func::from(Self::prepend_listener))?;
        proto.set(
            "prependOnceListener",
            Func::from(Self::prepend_once_listener),
        )?;
        proto.set("off", off.clone())?;
        proto.set("eventNames", Func::from(Self::event_names))?;
        proto.set("addListener", on)?;
        proto.set("removeListener", off)?;
        proto.set("listenerCount", Func::from(Self::listener_count))?;
        proto.set("listeners", Func::from(Self::listeners))?;
        proto.set("removeAllListeners", Func::from(Self::remove_all_listeners))?;

        Ok(proto)
    }

    fn add_event_target_prototype(ctx: &Ctx<'js>) -> Result<Object<'js>> {
        let proto = Class::<Self>::prototype(ctx)?
            .or_throw_msg(ctx, "Prototype for EventTarget not found")?;

        let on = Function::new(ctx.clone(), Self::evt_add_event_listener)?;
        let off = Function::new(ctx.clone(), Self::remove_event_listener)?;

        proto.set("dispatchEvent", Func::from(Self::evt_dispatch_event))?;
        proto.set("addEventListener", on)?;
        proto.set("removeEventListener", off)?;

        Ok(proto)
    }

    fn trace_event_emitter<'a>(&self, tracer: Tracer<'a, 'js>) {
        let events = self.get_event_list();
        let events = events.read().unwrap();
        for (key, items) in events.iter() {
            if let EventKey::Symbol(sym) = &key {
                tracer.mark(sym);
            }

            for item in items {
                tracer.mark(&item.callback);
            }
        }
    }

    fn remove_event_listener_str(
        this: Class<'js, Self>,
        ctx: &Ctx<'js>,
        event: &str,
        listener: Function<'js>,
    ) -> Result<Object<'js>> {
        let event = to_event(ctx, event)?;
        Self::remove_event_listener(This(class_to_obj(this)?), ctx.clone(), event, listener)
    }

    fn remove_event_listener(
        this: This<Object<'js>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Object<'js>> {
        let events = Self::resolve_events_from(&ctx, &this)?;
        let mut events = events.write().or_throw(&ctx)?;

        let key = EventKey::from_value(&ctx, event.clone())?;
        let mut removed_last = false;
        let mut removed_cb: Option<Function<'js>> = None;
        if let Some(index) = events.iter_mut().position(|(k, _)| k == &key) {
            let items = &mut events[index].1;
            // Node removes the *most recently* registered matching listener.
            if let Some(pos) = items.iter().rposition(|item| item.callback == listener) {
                removed_cb = Some(items.remove(pos).callback);
                if items.is_empty() {
                    events.remove(index);
                    removed_last = true;
                }
            }
        };
        drop(events);

        // Node: emit "removeListener" after removal, without holding the list lock.
        if let Some(removed) = removed_cb {
            Self::do_emit(
                to_event(&ctx, "removeListener")?,
                This(this.0.clone()),
                &ctx,
                Rest(vec![event, removed.into_value()]),
                false,
            )?;
        }

        if removed_last {
            if let Some(class) = Class::<Self>::from_object(&this.0) {
                class.borrow_mut().on_event_changed(key, false)?;
            }
        }

        Ok(this.0)
    }

    fn add_event_listener_str(
        this: Class<'js, Self>,
        ctx: &Ctx<'js>,
        event: &str,
        listener: Function<'js>,
        prepend: bool,
        once: bool,
    ) -> Result<Object<'js>> {
        let event = to_event(ctx, event)?;
        Self::add_event_listener(
            This(class_to_obj(this)?),
            ctx.clone(),
            event,
            listener,
            prepend,
            once,
        )
    }

    fn once(
        this: This<Object<'js>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Object<'js>> {
        Self::add_event_listener(this, ctx, event, listener, false, true)
    }

    fn on(
        this: This<Object<'js>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Object<'js>> {
        Self::add_event_listener(this, ctx, event, listener, false, false)
    }

    fn prepend_listener(
        this: This<Object<'js>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Object<'js>> {
        Self::add_event_listener(this, ctx, event, listener, true, false)
    }

    fn prepend_once_listener(
        this: This<Object<'js>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Object<'js>> {
        Self::add_event_listener(this, ctx, event, listener, true, true)
    }

    fn evt_add_event_listener(
        this: This<Object<'js>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
        options: Opt<Object<'js>>,
    ) -> Result<Object<'js>> {
        let mut once = false;
        if let Some(opt) = options.0 {
            if let Some(once_opt) = opt.get("once")? {
                once = once_opt;
            }
        }
        Self::add_event_listener(this, ctx, event, listener, false, once)
    }

    fn add_event_listener(
        this: This<Object<'js>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
        prepend: bool,
        once: bool,
    ) -> Result<Object<'js>> {
        // Node emits "newListener" *before* inserting, without holding the list lock.
        // Always emit, including when event is "newListener" or "removeListener"
        // (emit-before-insert avoids infinite recursion on the same registration).
        // Propagate listener exceptions (do not swallow).
        Self::do_emit(
            to_event(&ctx, "newListener")?,
            This(this.0.clone()),
            &ctx,
            Rest(vec![event.clone(), listener.clone().into_value()]),
            false,
        )?;

        let events = Self::resolve_events_from(&ctx, &this)?;
        let mut events = events.write().or_throw(&ctx)?;
        let key = EventKey::from_value(&ctx, event)?;
        let mut is_new = false;

        let items = match events.iter_mut().find(|(k, _)| k == &key) {
            Some((_, entry_items)) => entry_items,
            None => {
                is_new = true;
                events.push((key.clone(), Vec::new()));
                &mut events.last_mut().unwrap().1
            },
        };

        let item = EventItem {
            id: NEXT_LISTENER_ID.fetch_add(1, Ordering::Relaxed),
            callback: listener,
            once,
            fired: Rc::new(Cell::new(false)),
        };
        if !prepend {
            items.push(item);
        } else {
            items.insert(0, item);
        }
        if is_new {
            if let Some(class) = Class::<Self>::from_object(&this.0) {
                class.borrow_mut().on_event_changed(key, true)?;
            }
        }
        Ok(this.0)
    }

    fn has_listener_str(&self, event: &str) -> bool {
        let key = EventKey::String(event.into());
        has_key(self.get_event_list(), key)
    }

    #[allow(dead_code)]
    fn has_listener(&self, ctx: Ctx<'js>, event: Value<'js>) -> Result<bool> {
        let key = EventKey::from_value(&ctx, event)?;
        Ok(has_key(self.get_event_list(), key))
    }

    #[allow(dead_code)]
    fn get_listeners(&self, ctx: &Ctx<'js>, event: Value<'js>) -> Result<Vec<Function<'js>>> {
        let key = EventKey::from_value(ctx, event)?;
        Ok(find_all_listeners(self.get_event_list(), key))
    }

    fn get_listeners_str(&self, event: &str) -> Vec<Function<'js>> {
        let key = EventKey::String(event.into());
        find_all_listeners(self.get_event_list(), key)
    }

    fn do_emit(
        event: Value<'js>,
        this: This<Object<'js>>,
        ctx: &Ctx<'js>,
        args: Rest<Value<'js>>,
        defer: bool,
    ) -> Result<bool> {
        let events = Self::resolve_events_from(ctx, &this)?;
        let mut events = events.write().or_throw(ctx)?;
        let key = EventKey::from_value(ctx, event.clone())?;

        if let Some(index) = events.iter_mut().position(|(k, _)| k == &key) {
            // Snapshot by ID + shared fired flag (Node onceWrapper semantics).
            let items = &events[index].1;
            let callbacks: Vec<(u64, Function<'js>, bool, Rc<Cell<bool>>)> = items
                .iter()
                .map(|item| {
                    (
                        item.id,
                        item.callback.clone(),
                        item.once,
                        Rc::clone(&item.fired),
                    )
                })
                .collect();
            drop(events);

            for (id, callback, was_once, fired) in callbacks {
                if was_once {
                    // Already consumed by a recursive emit — skip (Node fired).
                    if fired.get() {
                        continue;
                    }
                    // Node: removeListener, then fired=true, then call listener.
                    let _ = remove_listener_by_id(ctx, &this, &key, id, true)?;
                    fired.set(true);
                }
                let call_args: Vec<Value<'js>> = args.iter().map(|arg| arg.to_owned()).collect();
                let call_args = Rest(call_args);
                let this_val = This(this.0.clone().into_value());
                if defer {
                    callback.defer((this_val, call_args))?;
                } else {
                    callback.call::<_, ()>((this_val, call_args))?;
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn emit_str(
        this: Class<'js, Self>,
        ctx: &Ctx<'js>,
        event: &str,
        args: Vec<Value<'js>>,
        defer: bool,
    ) -> Result<()> {
        let event = to_event(ctx, event)?;
        Self::do_emit(event, This(class_to_obj(this)?), ctx, args.into(), defer)?;
        Ok(())
    }

    fn emit(
        this: This<Object<'js>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        args: Rest<Value<'js>>,
    ) -> Result<bool> {
        Self::do_emit(event, this, &ctx, args, false)
    }

    fn evt_dispatch_event(
        this: This<Object<'js>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
    ) -> Result<bool> {
        let event_type = event.get_optional("type")?.unwrap();
        Self::do_emit(event_type, this, &ctx, Rest(vec![event]), false)
    }

    fn event_names(this: This<Object<'js>>, ctx: Ctx<'js>) -> Result<Vec<Value<'js>>> {
        let events = Self::resolve_events_from(&ctx, &this)?;
        let events = events.read().or_throw(&ctx)?;

        let mut names = Vec::with_capacity(events.len());
        for (key, _entry) in events.iter() {
            let value = match key {
                EventKey::Symbol(symbol) => symbol.clone().into_value(),
                EventKey::String(str) => JsString::from_str(ctx.clone(), str)?.into(),
            };

            names.push(value)
        }

        Ok(names)
    }

    fn listener_count(this: This<Object<'js>>, ctx: Ctx<'js>, event: Value<'js>) -> Result<usize> {
        let events = Self::resolve_events_from(&ctx, &this)?;
        let key = EventKey::from_value(&ctx, event)?;
        let events = events.read().or_throw(&ctx)?;
        Ok(events
            .iter()
            .find(|(k, _)| k == &key)
            .map_or(0, |(_, items)| items.len()))
    }

    /// Returns a copy of the array of listeners for the event named `event`.
    fn listeners(
        this: This<Object<'js>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
    ) -> Result<Vec<Function<'js>>> {
        let events = Self::resolve_events_from(&ctx, &this)?;
        let key = EventKey::from_value(&ctx, event)?;
        Ok(find_all_listeners(events, key))
    }

    fn remove_all_listeners(
        this: This<Object<'js>>,
        ctx: Ctx<'js>,
        event: Opt<Value<'js>>,
    ) -> Result<Object<'js>> {
        match event.0 {
            Some(event_val) if !event_val.is_undefined() => {
                let key = EventKey::from_value(&ctx, event_val)?;
                remove_all_for_key(&ctx, &this, &key)?;
            },
            _ => {
                // Snapshot current event names; process non-removeListener first,
                // then removeListener itself (Node order).
                let events = Self::resolve_events_from(&ctx, &this)?;
                let events = events.read().or_throw(&ctx)?;
                let keys: Vec<EventKey<'js>> = events
                    .iter()
                    .filter(|(_, items)| !items.is_empty())
                    .map(|(k, _)| k.clone())
                    .collect();
                drop(events);

                for key in keys.iter().filter(|k| !is_remove_listener_key(k)) {
                    remove_all_for_key(&ctx, &this, key)?;
                }
                remove_all_for_key(&ctx, &this, &EventKey::String("removeListener".into()))?;
            },
        }

        Ok(this.0)
    }
}

fn find_all_listeners<'js>(
    events: Arc<RwLock<EventList<'js>>>,
    key: EventKey<'js>,
) -> Vec<Function<'js>> {
    let events = events.read().unwrap();
    let items = events.iter().find(|(k, _)| k == &key);
    if let Some((_, callbacks)) = items {
        callbacks.iter().map(|item| item.callback.clone()).collect()
    } else {
        vec![]
    }
}

fn has_key<'js>(event_list: Arc<RwLock<EventList<'js>>>, key: EventKey<'js>) -> bool {
    event_list.read().unwrap().iter().any(|(k, _)| k == &key)
}

fn to_event<'js>(ctx: &Ctx<'js>, event: &str) -> Result<Value<'js>> {
    let event = JsString::from_str(ctx.clone(), event)?;
    Ok(event.into_value())
}

pub struct EventsModule;

impl ModuleDef for EventsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(EventEmitter))?;
        declare.declare("on")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let ctor = Class::<EventEmitter>::create_constructor(ctx)?
            .expect("Can't create EventEmitter constructor");
        ctor.set(stringify!(EventEmitter), ctor.clone())?;

        // Node: EventEmitter.on(emitter, eventName[, options]) → AsyncIterator
        // Semantics aligned with Node 24.3 (always yield args as array; watermark
        // pause/resume; no event drop over HWM).
        let on_static: Function = ctx.eval(
            r#"(function () {
  return function on(emitter, event, options) {
    if (emitter == null || typeof emitter.on !== "function") {
      throw new TypeError('The "emitter" argument must be an EventEmitter');
    }
    if (options === undefined || options === null) {
      options = {};
    } else if (typeof options !== "object" || Array.isArray(options)) {
      throw new TypeError('The "options" argument must be of type object. Received ' + options);
    }

    const signal = options.signal;
    if (signal != null) {
      if (typeof signal !== "object" || typeof signal.aborted !== "boolean") {
        throw new TypeError('The "options.signal" property must be an AbortSignal.');
      }
      if (signal.aborted) {
        const err = new Error("The operation was aborted");
        err.name = "AbortError";
        if (signal.reason !== undefined) {
          try { err.cause = signal.reason; } catch (_) {}
        }
        throw err;
      }
    }

    const closeEvents = options.close;
    if (closeEvents != null && !Array.isArray(closeEvents)) {
      throw new TypeError('The "options.close" property must be an array. Received ' + closeEvents);
    }
    const closes = closeEvents || [];

    function validatePositiveInteger(value, name) {
      if (typeof value !== "number" || !Number.isInteger(value) || value < 1 || !Number.isFinite(value)) {
        throw new TypeError('The "' + name + '" property must be a positive integer. Received ' + value);
      }
      return value;
    }

    // Support both highWaterMark/highWatermark and lowWaterMark/lowWatermark (Node).
    const highWaterMark = validatePositiveInteger(
      options.highWaterMark ?? options.highWatermark ?? Number.MAX_SAFE_INTEGER,
      "options.highWaterMark"
    );
    const lowWaterMark = validatePositiveInteger(
      options.lowWaterMark ?? options.lowWatermark ?? 1,
      "options.lowWaterMark"
    );

    const queue = [];
    const waiters = [];
    let finished = false;
    let error = null;
    let paused = false;

    function maybePause() {
      if (!paused && queue.length >= highWaterMark && typeof emitter.pause === "function") {
        paused = true;
        try { emitter.pause(); } catch (_) {}
      }
    }

    function maybeResume() {
      if (paused && queue.length <= lowWaterMark && typeof emitter.resume === "function") {
        paused = false;
        try { emitter.resume(); } catch (_) {}
      }
    }

    function rejectAll(err) {
      while (waiters.length) {
        const w = waiters.shift();
        try { w.reject(err); } catch (_) {}
      }
    }

    function resolveAllDone() {
      const result = { value: undefined, done: true };
      while (waiters.length) {
        const w = waiters.shift();
        try { w.resolve(result); } catch (_) {}
      }
    }

    function onData() {
      if (finished) return;
      // Always yield args as an array (never unwrap a single arg).
      const value = Array.prototype.slice.call(arguments);
      if (waiters.length) {
        const w = waiters.shift();
        w.resolve({ value: value, done: false });
      } else {
        queue.push(value);
        // Do NOT drop events over HWM — pause the source once instead.
        maybePause();
      }
    }

    function onError(err) {
      if (finished) return;
      error = err;
      finished = true;
      cleanup();
      paused = false;
      rejectAll(err);
    }

    function onClose() {
      if (finished) return;
      finished = true;
      cleanup();
      paused = false;
      resolveAllDone();
    }

    function cleanup() {
      try {
        if (typeof emitter.off === "function") emitter.off(event, onData);
        else if (typeof emitter.removeListener === "function") emitter.removeListener(event, onData);
      } catch (_) {}
      try {
        if (typeof emitter.off === "function") emitter.off("error", onError);
        else if (typeof emitter.removeListener === "function") emitter.removeListener("error", onError);
      } catch (_) {}
      for (let i = 0; i < closes.length; i++) {
        const ce = closes[i];
        try {
          if (typeof emitter.off === "function") emitter.off(ce, onClose);
          else if (typeof emitter.removeListener === "function") emitter.removeListener(ce, onClose);
        } catch (_) {}
      }
      if (signal && typeof signal.removeEventListener === "function") {
        try { signal.removeEventListener("abort", onAbort); } catch (_) {}
      }
    }

    function makeAbortError() {
      const err = new Error("The operation was aborted");
      err.name = "AbortError";
      if (signal && signal.reason !== undefined) {
        try { err.cause = signal.reason; } catch (_) {}
      }
      return err;
    }

    function onAbort() {
      if (finished) return;
      error = makeAbortError();
      finished = true;
      cleanup();
      paused = false;
      rejectAll(error);
    }

    emitter.on(event, onData);
    if (event !== "error" && typeof emitter.on === "function") {
      emitter.on("error", onError);
    }
    for (let i = 0; i < closes.length; i++) {
      emitter.on(closes[i], onClose);
    }
    if (signal && typeof signal.addEventListener === "function") {
      signal.addEventListener("abort", onAbort, { once: true });
    }

    return {
      [Symbol.asyncIterator]() { return this; },
      next() {
        if (error) return Promise.reject(error);
        if (queue.length) {
          const value = queue.shift();
          maybeResume();
          return Promise.resolve({ value: value, done: false });
        }
        if (finished) return Promise.resolve({ value: undefined, done: true });
        return new Promise(function (resolve, reject) {
          waiters.push({ resolve: resolve, reject: reject });
        });
      },
      return() {
        if (!finished) {
          finished = true;
          cleanup();
          paused = false;
          resolveAllDone();
        }
        return Promise.resolve({ value: undefined, done: true });
      },
      throw(err) {
        if (!finished) {
          finished = true;
          cleanup();
          paused = false;
        }
        error = err;
        rejectAll(err);
        return Promise.reject(err);
      },
    };
  };
})()"#,
        )?;
        ctor.set("on", on_static.clone())?;

        exports.export(stringify!(EventEmitter), ctor.clone())?;
        exports.export("on", on_static)?;
        exports.export("default", ctor)?;

        EventEmitter::add_event_emitter_prototype(ctx)?;

        Ok(())
    }
}

impl From<EventsModule> for ModuleInfo<EventsModule> {
    fn from(val: EventsModule) -> Self {
        ModuleInfo {
            name: "events",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    Class::<EventTarget>::define(&globals)?;
    Class::<CustomEvent>::define(&globals)?;
    Class::<Event>::define(&globals)?;

    EventTarget::add_event_target_prototype(ctx)?;

    Ok(())
}
