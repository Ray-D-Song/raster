// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, collections::HashSet, rc::Rc};

use raster_runtime_utils::io::BYTECODE_FILE_EXT;
use rquickjs::{Ctx, Object, Result, Value};
use tracing::trace;

use crate::CJS_IMPORT_PREFIX;

use super::extensions::load_via_extensions;
use super::facade::{call_resolve_filename, get_or_create_module_record};
use super::import_load::load_via_import;
use super::{ModuleNames, RequireState};

fn normalize_builtin_request(request: &str) -> String {
    request
        .trim_start_matches("node:")
        .trim_end_matches('/')
        .to_string()
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
    let is_bytecode = resolved.ends_with(BYTECODE_FILE_EXT);

    let import_name: Rc<str>;
    let import_specifier: Rc<str>;
    let is_builtin;

    if !is_cjs_import {
        let normalized = if is_bytecode {
            resolved.clone()
        } else {
            normalize_builtin_request(&resolved)
        };

        is_builtin = module_list.contains(normalized.as_str());
        if is_builtin {
            import_name = normalized.into();
            import_specifier = import_name.clone();
        } else if is_bytecode {
            import_name = resolved.clone().into();
            import_specifier = import_name.clone();
        } else {
            import_name = resolved.clone().into();
            import_specifier = import_name.clone();
        }
    } else {
        is_builtin = false;
        import_name = resolved[CJS_IMPORT_PREFIX.len()..].into();
        import_specifier = resolved.into();
    };

    trace!("Require resolved: {} -> {}", specifier, import_specifier);

    {
        let facade = ctx
            .userdata::<RefCell<super::facade::ModuleFacadeState>>()
            .unwrap();
        let js_cache = facade.borrow().cache.clone();
        if let Ok(record) = js_cache.get::<_, Object>(import_name.as_ref()) {
            if record.get::<_, bool>("loaded").unwrap_or(false) {
                return record.get("exports");
            }
        }
    }

    {
        let binding = ctx.userdata::<RefCell<RequireState>>().unwrap();
        let mut state = binding.borrow_mut();

        if let Some(cached_value) = state.cache.get(import_name.as_ref()) {
            if ctx
                .userdata::<RefCell<super::facade::ModuleFacadeState>>()
                .unwrap()
                .borrow()
                .cache
                .get::<_, Object>(import_name.as_ref())
                .is_err()
            {
                state.cache.remove(import_name.as_ref());
            } else {
                return Ok(cached_value.clone());
            }
        }

        if let Some(obj) = state.progress.get(&import_name) {
            if let Some(record) = obj.as_object() {
                if let Ok(exports) = record.get::<_, Value>("exports") {
                    return Ok(exports);
                }
            }
            return Ok(obj.clone().into_value());
        }
    }

    if is_builtin || is_bytecode || is_cjs_import {
        let record = get_or_create_module_record(&ctx, import_name.as_ref(), record_parent)?;
        record.set("loaded", false)?;
        return load_via_import(ctx, import_name, import_specifier, record);
    }

    let record = get_or_create_module_record(&ctx, import_name.as_ref(), record_parent)?;
    load_via_extensions(ctx, import_name, import_specifier, record)
}
