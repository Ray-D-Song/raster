// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, collections::HashSet, fs, rc::Rc};

use raster_runtime_hooking::{invoke_async_hook, register_finalization_registry, HookType};
use raster_runtime_json::parse::json_parse;
use raster_runtime_utils::{io::BYTECODE_FILE_EXT, provider::ProviderType};
use rquickjs::{atom::PredefinedAtom, qjs, Ctx, Module, Object, Result, Value};
use tokio::time::Instant;
use tracing::trace;

use crate::modules::timers::poll_timers;
use crate::CJS_IMPORT_PREFIX;

use super::facade::{call_resolve_filename, get_or_create_module_record};
use super::{ModuleNames, RequireState};

struct CurrentModuleGuard<'js> {
    ctx: Ctx<'js>,
}

impl Drop for CurrentModuleGuard<'_> {
    fn drop(&mut self) {
        if let Some(binding) = self.ctx.userdata::<RefCell<RequireState>>() {
            binding.borrow_mut().current_module = None;
        }
    }
}

pub fn require_from_module<'js>(
    ctx: Ctx<'js>,
    parent: Object<'js>,
    specifier: String,
    link_parent: bool,
) -> Result<Value<'js>> {
    let module_list = ctx
        .userdata::<ModuleNames>()
        .map_or_else(HashSet::new, |v| v.get_list());

    let resolved = call_resolve_filename(&ctx, &specifier, Some(parent.clone()), None)?;
    let record_parent = if link_parent { Some(parent) } else { None };

    let is_cjs_import = resolved.starts_with(CJS_IMPORT_PREFIX);
    let is_json = resolved.ends_with(".json");

    let import_name: Rc<str>;
    let import_specifier: Rc<str>;

    if !is_cjs_import {
        let is_bytecode = resolved.ends_with(BYTECODE_FILE_EXT);
        let is_bytecode_or_json = is_json || is_bytecode;
        let normalized = if is_bytecode_or_json {
            resolved.clone()
        } else {
            normalize_builtin_request(&resolved)
        };

        if module_list.contains(normalized.as_str()) {
            import_name = normalized.into();
            import_specifier = import_name.clone();
        } else {
            import_name = resolved.clone().into();
            import_specifier = if is_bytecode_or_json {
                import_name.clone()
            } else {
                [CJS_IMPORT_PREFIX, &import_name].concat().into()
            };
        }
    } else {
        import_name = resolved[CJS_IMPORT_PREFIX.len()..].into();
        import_specifier = resolved.into();
    };

    trace!("Require resolved: {} -> {}", specifier, import_specifier);

    let facade = ctx
        .userdata::<RefCell<super::facade::ModuleFacadeState>>()
        .unwrap();
    let js_cache = facade.borrow().cache.clone();
    if let Ok(record) = js_cache.get::<_, Object>(import_name.as_ref()) {
        if record.get::<_, bool>("loaded").unwrap_or(false) {
            return record.get("exports");
        }
    }

    let binding = ctx.userdata::<RefCell<RequireState>>().unwrap();
    let mut state = binding.borrow_mut();

    if let Some(cached_value) = state.cache.get(import_name.as_ref()) {
        if js_cache.get::<_, Object>(import_name.as_ref()).is_err() {
            state.cache.remove(import_name.as_ref());
        } else {
            return Ok(cached_value.clone());
        }
    }

    if is_json {
        let json = fs::read_to_string(import_name.as_ref())?;
        let json = json_parse(&ctx, json)?;
        let record =
            get_or_create_module_record(&ctx, import_name.as_ref(), record_parent.clone())?;
        record.set("loaded", true)?;
        record.set("exports", json.clone())?;
        state.cache.insert(import_name, json.clone());
        return Ok(json);
    }

    if let Some(obj) = state.progress.get(&import_name) {
        return Ok(obj.clone().into_value());
    }

    trace!("Require loading: {}", import_specifier);

    let record = get_or_create_module_record(&ctx, import_name.as_ref(), record_parent)?;
    record.set("loaded", false)?;

    let obj = Object::new(ctx.clone())?;
    state.progress.insert(import_name.clone(), obj.clone());
    drop(state);

    {
        binding.borrow_mut().current_module = Some(record.clone());
    }
    let _current_module_guard = CurrentModuleGuard { ctx: ctx.clone() };

    let import_promise = Module::import(&ctx, import_specifier.as_bytes())?;

    let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };

    let mut deadline = Instant::now();
    let mut executing_timers = Vec::new();

    let uid = unsafe { qjs::JS_VALUE_GET_PTR(obj.as_object().unwrap().as_raw()) } as usize;
    register_finalization_registry(&ctx, obj.clone().into_value(), uid)?;
    invoke_async_hook(&ctx, HookType::Init, ProviderType::TimerWrap, uid)?;

    let imported_object = loop {
        if let Some(x) = import_promise.result::<Object>() {
            break x?;
        }

        if deadline < Instant::now() {
            poll_timers(rt, &mut executing_timers, None, Some(&mut deadline))?;
        }

        ctx.execute_pending_job();
    };

    let binding = ctx.userdata::<RefCell<RequireState>>().unwrap();
    let mut state = binding.borrow_mut();

    let exports_obj = state.exports.get(&import_name).cloned();
    state.progress.remove(import_name.as_ref());

    if let Some(exports_obj) = exports_obj {
        if exports_obj.type_of() == rquickjs::Type::Object {
            drop(state);
            let exports = unsafe { exports_obj.as_object().unwrap_unchecked() };

            for prop in exports
                .own_props::<Value, Value>(rquickjs::Filter::new().private().string().symbol())
            {
                let (key, value) = prop?;
                obj.set(key, value)?;
            }
        } else {
            state.cache.insert(import_name.clone(), exports_obj.clone());
            record.set("loaded", true)?;
            record.set("exports", exports_obj.clone())?;
            return Ok(exports_obj);
        }
    } else {
        drop(state);
    }

    let binding = ctx.userdata::<RefCell<RequireState>>().unwrap();
    let mut state = binding.borrow_mut();

    let props = imported_object.props::<String, Value>();
    let default_export: Option<Value> = imported_object.get(PredefinedAtom::Default)?;

    if let Some(default_export) = default_export {
        if let Some(default_object) = default_export.as_object() {
            for prop in props {
                let (key, value) = prop?;
                if !default_object.contains_key(&key)? {
                    default_object.set(key, value)?;
                }
            }
            let default_object = default_object.clone().into_value();
            state
                .cache
                .insert(import_name.clone(), default_object.clone());
            record.set("loaded", true)?;
            record.set("exports", default_object.clone())?;
            return Ok(default_object);
        }
    }

    for prop in props {
        let (key, value) = prop?;
        obj.set(key, value)?;
    }

    let value = obj.into_value();
    state.cache.insert(import_name.clone(), value.clone());
    record.set("loaded", true)?;
    record.set("exports", value.clone())?;
    Ok(value)
}

fn normalize_builtin_request(request: &str) -> String {
    request
        .trim_start_matches("node:")
        .trim_end_matches('/')
        .to_string()
}
