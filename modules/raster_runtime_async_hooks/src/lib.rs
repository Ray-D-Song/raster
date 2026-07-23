// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

use raster_runtime_hooking::{register_finalization_registry, HOOKING_MODE};
use raster_runtime_utils::{
    module::{export_default, ModuleInfo},
    result::ResultExt,
};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    promise::PromiseHookType,
    qjs,
    runtime::PromiseHook,
    Ctx, Exception, Function, JsLifetime, Object, Result, Value,
};
use tracing::trace;

mod finalization_registry;

use crate::finalization_registry::init_finalization_registry;

type AsyncId = u64;
type TriggerAsyncId = i64;

const BOOTSTRAP_CONTEXT: (AsyncId, TriggerAsyncId) = (1, 1);

struct Hook<'js> {
    enabled: Rc<RefCell<bool>>,
    init: Option<Function<'js>>,
    before: Option<Function<'js>>,
    after: Option<Function<'js>>,
    promise_resolve: Option<Function<'js>>,
    destroy: Option<Function<'js>>,
}

struct AsyncHookState<'js> {
    hooks: Vec<Hook<'js>>,
}

impl Default for AsyncHookState<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncHookState<'_> {
    fn new() -> Self {
        Self { hooks: Vec::new() }
    }
}

unsafe impl<'js> JsLifetime<'js> for AsyncHookState<'js> {
    type Changed<'to> = AsyncHookState<'to>;
}

struct AsyncHookIds<'js> {
    next_async_id: AsyncId,
    id_map: HashMap<usize, (AsyncId, TriggerAsyncId)>,
    current_id: (AsyncId, TriggerAsyncId),
    scope_stack: Vec<(AsyncId, TriggerAsyncId)>,
    _marker: PhantomData<&'js ()>,
}

impl Default for AsyncHookIds<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncHookIds<'_> {
    fn new() -> Self {
        Self {
            next_async_id: 1,
            id_map: HashMap::new(),
            current_id: BOOTSTRAP_CONTEXT,
            scope_stack: Vec::new(),
            _marker: PhantomData,
        }
    }
}

unsafe impl<'js> JsLifetime<'js> for AsyncHookIds<'js> {
    type Changed<'to> = AsyncHookIds<'to>;
}

/// Shared ID allocator for promises, timers, DNS, and AsyncResource.
/// Skips 0 and the bootstrap ID 1.
fn allocate_async_id(ids: &mut AsyncHookIds<'_>) -> AsyncId {
    loop {
        ids.next_async_id = ids.next_async_id.wrapping_add(1);
        let id = ids.next_async_id;
        if id > 1 {
            return id;
        }
    }
}

fn create_hook<'js>(ctx: Ctx<'js>, hooks_obj: Object<'js>) -> Result<Value<'js>> {
    let init = hooks_obj.get::<_, Function>("init").ok();
    let before = hooks_obj.get::<_, Function>("before").ok();
    let after = hooks_obj.get::<_, Function>("after").ok();
    let promise_resolve = hooks_obj.get::<_, Function>("promiseResolve").ok();
    let destroy = hooks_obj.get::<_, Function>("destroy").ok();
    let enabled = Rc::new(RefCell::new(false));

    let hook = Hook {
        enabled: enabled.clone(),
        init,
        before,
        after,
        promise_resolve,
        destroy,
    };

    let binding = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(&ctx)?;
    let mut state = binding.borrow_mut();
    state.hooks.push(hook);

    let obj = Object::new(ctx.clone())?;
    {
        let enabled_clone = enabled.clone();
        obj.set(
            "enable",
            Function::new(ctx.clone(), move || -> Result<()> {
                *enabled_clone.borrow_mut() = true;
                Ok(())
            }),
        )?;
    }
    {
        let enabled_clone = enabled.clone();
        obj.set(
            "disable",
            Function::new(ctx.clone(), move || -> Result<()> {
                *enabled_clone.borrow_mut() = false;
                Ok(())
            }),
        )?;
    }

    Ok(obj.into())
}

fn current_id() -> u64 {
    // NOTE: This method is now obsolete. Therefore, it does not return a valid value.
    // But we will define it because it is used by cls-hooked.
    0
}

fn execution_async_id(ctx: Ctx<'_>) -> Result<u64> {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().or_throw(&ctx)?;
    let ids = bind_ids.borrow();
    Ok(ids.current_id.0)
}

fn trigger_async_id(ctx: Ctx<'_>) -> Result<i64> {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().or_throw(&ctx)?;
    let ids = bind_ids.borrow();
    Ok(ids.current_id.1)
}

#[derive(Clone, Copy)]
enum HookKind {
    Init,
    Before,
    After,
    PromiseResolve,
    Destroy,
}

/// Collect enabled hook callbacks of one kind, cloning functions so the state
/// borrow can be released before user JS runs.
fn collect_enabled_hooks<'js>(state: &AsyncHookState<'js>, kind: HookKind) -> Vec<Function<'js>> {
    let mut funcs = Vec::new();
    for hook in &state.hooks {
        if !*hook.enabled.as_ref().borrow() {
            continue;
        }
        let func = match kind {
            HookKind::Init => hook.init.as_ref(),
            HookKind::Before => hook.before.as_ref(),
            HookKind::After => hook.after.as_ref(),
            HookKind::PromiseResolve => hook.promise_resolve.as_ref(),
            HookKind::Destroy => hook.destroy.as_ref(),
        };
        if let Some(func) = func {
            funcs.push(func.clone());
        }
    }
    funcs
}

fn has_enabled_destroy_hooks(ctx: &Ctx<'_>) -> Result<bool> {
    let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(ctx)?;
    let state = bind_state.borrow();
    Ok(state
        .hooks
        .iter()
        .any(|hook| *hook.enabled.as_ref().borrow() && hook.destroy.is_some()))
}

fn has_enabled_init_hooks(ctx: &Ctx<'_>) -> Result<bool> {
    let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(ctx)?;
    let state = bind_state.borrow();
    Ok(state
        .hooks
        .iter()
        .any(|hook| *hook.enabled.as_ref().borrow() && hook.init.is_some()))
}

fn push_async_context(ctx: &Ctx<'_>, id: (AsyncId, TriggerAsyncId)) -> Result<()> {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().or_throw(ctx)?;
    let mut ids = bind_ids.borrow_mut();
    let previous = ids.current_id;
    ids.scope_stack.push(previous);
    ids.current_id = id;
    Ok(())
}

fn pop_async_context(ctx: &Ctx<'_>) -> Result<()> {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().or_throw(ctx)?;
    let mut ids = bind_ids.borrow_mut();
    ids.current_id = ids.scope_stack.pop().unwrap_or(BOOTSTRAP_CONTEXT);
    Ok(())
}

// ---------------------------------------------------------------------------
// AsyncResource native bridges (not exported on globalThis)
// ---------------------------------------------------------------------------

/// Allocate a stable async ID without dispatching init hooks. The JS constructor
/// writes IDs onto the instance first, then calls [`async_resource_emit_init`] so
/// init callbacks observe a fully initialized resource (matching Node).
///
/// Destroy-registration is intentionally *not* decided here: init callbacks may
/// enable/disable destroy hooks, so that decision happens after emit init.
fn async_resource_allocate(ctx: Ctx<'_>) -> Result<Object<'_>> {
    let async_id = {
        let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().or_throw(&ctx)?;
        let mut ids = bind_ids.borrow_mut();
        allocate_async_id(&mut ids)
    };
    // Creating an AsyncResource must not pollute the caller's execution ID.

    let out = Object::new(ctx.clone())?;
    out.set("asyncId", async_id)?;
    Ok(out)
}

/// Dispatch init hooks, then return whether GC auto-destroy should be registered.
/// The boolean is computed *after* init runs so enable/disable inside init is observed
/// (Node-compatible).
fn async_resource_emit_init<'js>(
    ctx: Ctx<'js>,
    resource: Object<'js>,
    type_name: String,
    async_id: u64,
    trigger_async_id: i64,
) -> Result<bool> {
    if !*HOOKING_MODE {
        return Ok(false);
    }

    if type_name.is_empty() && has_enabled_init_hooks(&ctx)? {
        return Err(Exception::throw_type(
            &ctx,
            "The \"type\" argument must be of type string and non-empty when init hooks exist.",
        ));
    }

    let funcs = {
        let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(&ctx)?;
        let state = bind_state.borrow();
        collect_enabled_hooks(&state, HookKind::Init)
    };

    let resource_val: Value = resource.into_value();
    for func in funcs {
        let _ = func.call::<_, ()>((
            async_id,
            type_name.as_str(),
            trigger_async_id,
            resource_val.clone(),
        ));
    }

    // After init: re-check destroy hooks (init may have enabled/disabled them).
    Ok(has_enabled_destroy_hooks(&ctx)?)
}

fn async_resource_before(ctx: Ctx<'_>, async_id: u64, trigger_async_id: i64) -> Result<()> {
    push_async_context(&ctx, (async_id, trigger_async_id))?;

    let funcs = {
        let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(&ctx)?;
        let state = bind_state.borrow();
        collect_enabled_hooks(&state, HookKind::Before)
    };
    for func in funcs {
        let _ = func.call::<_, ()>((async_id,));
    }
    Ok(())
}

fn async_resource_after(ctx: Ctx<'_>, async_id: u64) -> Result<()> {
    let funcs = {
        let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(&ctx)?;
        let state = bind_state.borrow();
        collect_enabled_hooks(&state, HookKind::After)
    };
    // After hooks still see the resource context.
    for func in funcs {
        let _ = func.call::<_, ()>((async_id,));
    }

    pop_async_context(&ctx)?;
    Ok(())
}

fn async_resource_destroy(ctx: Ctx<'_>, async_id: u64) -> Result<()> {
    let funcs = {
        let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(&ctx)?;
        let state = bind_state.borrow();
        collect_enabled_hooks(&state, HookKind::Destroy)
    };
    for func in funcs {
        let _ = func.call::<_, ()>((async_id,));
    }
    Ok(())
}

const ASYNC_RESOURCE_FACTORY_JS: &str = r#"(function (
  allocateNative,
  emitInitNative,
  beforeNative,
  afterNative,
  destroyNative,
  executionAsyncId
) {
  const kAsyncId = Symbol("asyncId");
  const kTriggerAsyncId = Symbol("triggerAsyncId");
  // Shared with FinalizationRegistry held value so emitDestroy can suppress
  // a later GC-driven destroy (Node uses the same pattern).
  const kDestroyed = Symbol("destroyed");

  const destroyRegistry = new FinalizationRegistry((held) => {
    if (held && !held.destroyed) {
      held.destroyed = true;
      destroyNative(held.asyncId);
    }
  });

  function validateType(type) {
    if (typeof type !== "string") {
      throw new TypeError('The "type" argument must be of type string. Received type ' + typeof type);
    }
  }

  function validateFunction(fn, name) {
    if (typeof fn !== "function") {
      throw new TypeError(
        'The "' + name + '" argument must be of type function. Received type ' + typeof fn
      );
    }
  }

  function validateTriggerAsyncId(triggerAsyncId) {
    if (!Number.isSafeInteger(triggerAsyncId) || triggerAsyncId < -1) {
      throw new RangeError(
        'The "triggerAsyncId" argument must be a safe integer >= -1. Received ' + triggerAsyncId
      );
    }
  }

  class AsyncResource {
    constructor(type, options) {
      validateType(type);

      if (options === undefined) {
        options = {};
      }

      let triggerAsyncId;
      let requireManualDestroy = false;

      if (typeof options === "number") {
        triggerAsyncId = options;
      } else {
        if (options === null || typeof options !== "object") {
          throw new TypeError(
            'The "options" argument must be of type object. Received type ' + typeof options
          );
        }
        triggerAsyncId =
          options.triggerAsyncId === undefined
            ? executionAsyncId()
            : options.triggerAsyncId;
        requireManualDestroy = Boolean(options.requireManualDestroy);
      }

      validateTriggerAsyncId(triggerAsyncId);

      // 1) allocate ID, 2) write instance fields, 3) dispatch init — so init
      // hooks observe a fully initialized resource (Node-compatible).
      // 4) registerDestroy is decided *after* init, matching Node (init may
      // enable/disable destroy hooks).
      const state = allocateNative();
      this[kAsyncId] = state.asyncId;
      this[kTriggerAsyncId] = triggerAsyncId;
      const registerDestroy = emitInitNative(
        this,
        type,
        state.asyncId,
        triggerAsyncId
      );

      if (!requireManualDestroy && registerDestroy) {
        // held.destroyed is a separate object so the registry does not keep
        // the AsyncResource instance itself alive.
        const destroyed = { destroyed: false, asyncId: state.asyncId };
        this[kDestroyed] = destroyed;
        destroyRegistry.register(this, destroyed);
      }
    }

    runInAsyncScope(fn, thisArg, ...args) {
      validateFunction(fn, "fn");

      beforeNative(this[kAsyncId], this[kTriggerAsyncId]);

      try {
        return Reflect.apply(fn, thisArg, args);
      } finally {
        afterNative(this[kAsyncId]);
      }
    }

    asyncId() {
      return this[kAsyncId];
    }

    triggerAsyncId() {
      return this[kTriggerAsyncId];
    }

    emitDestroy() {
      const destroyed = this[kDestroyed];
      if (destroyed !== undefined) {
        // Suppress the FinalizationRegistry callback; still allow repeated
        // explicit destroy dispatch (Node 24 keeps re-emitting).
        destroyed.destroyed = true;
      }
      destroyNative(this[kAsyncId]);
      return this;
    }

    bind(fn, thisArg) {
      validateFunction(fn, "fn");

      const resource = this;
      const dynamicThis = thisArg === undefined;

      const bound = function (...args) {
        return resource.runInAsyncScope(
          fn,
          dynamicThis ? this : thisArg,
          ...args
        );
      };

      let exposedResource = resource;

      Object.defineProperties(bound, {
        length: {
          configurable: true,
          enumerable: false,
          writable: false,
          value: fn.length,
        },
        asyncResource: {
          configurable: true,
          enumerable: true,
          get() {
            return exposedResource;
          },
          set(value) {
            exposedResource = value;
          },
        },
      });

      return bound;
    }

    static bind(fn, type, thisArg) {
      validateFunction(fn, "fn");
      type ||= fn.name;
      return new AsyncResource(type || "bound-anonymous-fn").bind(fn, thisArg);
    }
  }

  return AsyncResource;
})"#;

fn create_async_resource_constructor<'js>(ctx: &Ctx<'js>) -> Result<Value<'js>> {
    let factory: Function = ctx.eval(ASYNC_RESOURCE_FACTORY_JS)?;
    let allocate_native = Function::new(ctx.clone(), async_resource_allocate)?;
    let emit_init_native = Function::new(ctx.clone(), async_resource_emit_init)?;
    let before_native = Function::new(ctx.clone(), async_resource_before)?;
    let after_native = Function::new(ctx.clone(), async_resource_after)?;
    let destroy_native = Function::new(ctx.clone(), async_resource_destroy)?;
    let execution_fn = Function::new(ctx.clone(), execution_async_id)?;

    factory.call((
        allocate_native,
        emit_init_native,
        before_native,
        after_native,
        destroy_native,
        execution_fn,
    ))
}

pub struct AsyncHooksModule;

impl ModuleDef for AsyncHooksModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("createHook")?;
        declare.declare("currentId")?;
        declare.declare("executionAsyncId")?;
        declare.declare("triggerAsyncId")?;
        declare.declare("AsyncResource")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let async_resource = create_async_resource_constructor(ctx)?;

        export_default(ctx, exports, |default| {
            default.set("AsyncResource", async_resource)?;
            default.set("createHook", Func::from(create_hook))?;
            default.set("currentId", Func::from(current_id))?;
            default.set("executionAsyncId", Func::from(execution_async_id))?;
            default.set("triggerAsyncId", Func::from(trigger_async_id))?;

            Ok(())
        })?;

        Ok(())
    }
}

impl From<AsyncHooksModule> for ModuleInfo<AsyncHooksModule> {
    fn from(val: AsyncHooksModule) -> Self {
        ModuleInfo {
            name: "async_hooks",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let global = ctx.globals();

    let _ = ctx.store_userdata(RefCell::new(AsyncHookState::default()));
    let _ = ctx.store_userdata(RefCell::new(AsyncHookIds::default()));

    global.set(
        "invokeAsyncHook",
        Func::from(
            move |ctx: Ctx<'_>, type_: String, async_type: String, uid: usize| {
                let type_ = match type_.as_ref() {
                    "init" => PromiseHookType::Init,
                    "before" => PromiseHookType::Before,
                    "after" => PromiseHookType::After,
                    "resolve" => PromiseHookType::Resolve,
                    _ => return,
                };

                let _ = invoke_async_hook(&ctx, type_, async_type.as_ref(), uid, None, None);
            },
        ),
    )?;

    init_finalization_registry(ctx)?;

    Ok(())
}

pub fn promise_hook_tracker() -> PromiseHook {
    Box::new(
        |ctx: Ctx<'_>, type_: PromiseHookType, promise: Value<'_>, parent: Value<'_>| {
            // SAFETY: Since it checks in advance whether it is an Object type, we can always get a pointer to the object.
            let object = promise
                .as_object()
                .map(|v| unsafe { qjs::JS_VALUE_GET_PTR(v.as_raw()) } as usize)
                .unwrap();
            let parent = parent
                .as_object()
                .map(|v| unsafe { qjs::JS_VALUE_GET_PTR(v.as_raw()) } as usize);

            if type_ == PromiseHookType::Init {
                let _ = register_finalization_registry(&ctx, promise.clone(), object);
            }

            let _ = invoke_async_hook(&ctx, type_, "PROMISE", object, parent, Some(promise));
        },
    )
}

fn invoke_async_hook<'js>(
    ctx: &Ctx<'js>,
    type_: PromiseHookType,
    async_type: &str,
    object: usize,
    parent: Option<usize>,
    resource: Option<Value<'js>>,
) -> Result<()> {
    let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(ctx)?;

    // Snapshot enabled callbacks, then release the state borrow before calling JS.
    let (init_fns, before_fns, after_fns, resolve_fns) = {
        let state = bind_state.borrow();
        if state.hooks.is_empty() {
            return Ok(());
        }
        (
            collect_enabled_hooks(&state, HookKind::Init),
            collect_enabled_hooks(&state, HookKind::Before),
            collect_enabled_hooks(&state, HookKind::After),
            collect_enabled_hooks(&state, HookKind::PromiseResolve),
        )
    };

    match type_ {
        PromiseHookType::Init => {
            let resource_id = insert_id_map(ctx, object, parent)?;
            trace!("Init(async_id, trigger_id): {:?}", resource_id);
            // Init must not change current_id.

            let resource = resource.unwrap_or_else(|| Value::new_undefined(ctx.clone()));
            for func in init_fns {
                let _ = func.call::<_, ()>((
                    resource_id.0,
                    async_type,
                    resource_id.1,
                    resource.clone(),
                ));
            }
        },
        PromiseHookType::Before => {
            let resource_id = get_id_map(ctx, object)?;
            if resource_id.0 == 0 {
                return Ok(());
            }

            trace!("Before(async_id, trigger_id): {:?}", resource_id);
            push_async_context(ctx, resource_id)?;

            for func in before_fns {
                let _ = func.call::<_, ()>((resource_id.0,));
            }
        },
        PromiseHookType::After => {
            let resource_id = get_id_map(ctx, object)?;
            if resource_id.0 == 0 {
                return Ok(());
            }

            trace!("After(async_id, trigger_id): {:?}", resource_id);
            // After hooks still see the resource context.
            for func in after_fns {
                let _ = func.call::<_, ()>((resource_id.0,));
            }

            pop_async_context(ctx)?;
        },
        PromiseHookType::Resolve => {
            let resource_id = get_id_map(ctx, object)?;
            if resource_id.0 == 0 {
                return Ok(());
            }

            trace!("Resolve(async_id, trigger_id): {:?}", resource_id);
            // Resolve must not change current_id.
            for func in resolve_fns {
                let _ = func.call::<_, ()>((resource_id.0,));
            }
        },
    }
    Ok(())
}

fn insert_id_map(
    ctx: &Ctx<'_>,
    target: usize,
    parent: Option<usize>,
) -> Result<(AsyncId, TriggerAsyncId)> {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().or_throw(ctx)?;
    let mut ids = bind_ids.borrow_mut();
    let async_id = allocate_async_id(&mut ids);
    // Causality: child resources are triggered by the currently executing
    // resource (executionAsyncId), not by that resource's own trigger.
    // When a parent promise pointer is available, use its asyncId instead.
    let trigger_id = parent
        .and_then(|tid| ids.id_map.get(&tid).map(|id| id.0 as TriggerAsyncId))
        .unwrap_or(ids.current_id.0 as TriggerAsyncId);
    ids.id_map.insert(target, (async_id, trigger_id));
    Ok((async_id, trigger_id))
}

fn get_id_map(ctx: &Ctx<'_>, target: usize) -> Result<(AsyncId, TriggerAsyncId)> {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().or_throw(ctx)?;
    let ids = bind_ids.borrow();
    Ok(*ids.id_map.get(&target).unwrap_or(&(0, 0)))
}

pub(crate) fn remove_id_map(ctx: &Ctx<'_>, target: usize) -> Result<(AsyncId, TriggerAsyncId)> {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().or_throw(ctx)?;
    let mut ids = bind_ids.borrow_mut();
    Ok(ids
        .id_map
        .remove_entry(&target)
        .map(|(_, (async_id, trigger_id))| (async_id, trigger_id))
        .unwrap_or((0, 0)))
}

pub(crate) fn dispatch_destroy_hooks(ctx: &Ctx<'_>, async_id: AsyncId) -> Result<()> {
    let funcs = {
        let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(ctx)?;
        let state = bind_state.borrow();
        collect_enabled_hooks(&state, HookKind::Destroy)
    };
    for func in funcs {
        let _ = func.call::<_, ()>((async_id,));
    }
    Ok(())
}
