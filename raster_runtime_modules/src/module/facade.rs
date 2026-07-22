// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, collections::HashSet};

use raster_runtime_path as path;
use raster_runtime_utils::{ctx::CtxExt, result::ResultExt};
use rquickjs::{
    function::{Constructor, Opt},
    prelude::{Func, This},
    Array, Ctx, Exception, Function, JsLifetime, Object, Result, Value,
};

use crate::modules::path::resolve_path;
use crate::package::resolver::{node_module_paths, require_resolve_with_options};

use super::{register_hooks, ModuleNames, RequireState};

#[derive(JsLifetime)]
pub struct ModuleFacadeState<'js> {
    pub constructor: Constructor<'js>,
    pub cache: Object<'js>,
}

pub fn module_not_found(ctx: &Ctx<'_>, request: &str, parent: &str) -> Result<()> {
    let message = format!("Cannot find module '{request}' from '{parent}'");
    let error_ctor: Constructor = ctx.globals().get("Error")?;
    let err: Object = error_ctor.construct((message,))?;
    err.set("code", "MODULE_NOT_FOUND")?;
    Err(ctx.throw(err.into_value()))
}

fn normalize_builtin_request(request: &str) -> String {
    request
        .trim_start_matches("node:")
        .trim_end_matches('/')
        .to_string()
}

fn is_builtin_request(ctx: &Ctx<'_>, request: &str) -> bool {
    let module_list = ctx
        .userdata::<ModuleNames>()
        .map_or_else(HashSet::new, |v| v.get_list());
    let name = normalize_builtin_request(request);
    module_list.contains(name.as_str())
}

pub fn canonical_parent_filename(ctx: &Ctx<'_>, parent: Option<Object<'_>>) -> Result<String> {
    if let Some(parent) = parent {
        if let Ok(filename) = parent.get::<_, String>("filename") {
            return Ok(filename);
        }
        if let Ok(id) = parent.get::<_, String>("id") {
            return Ok(id);
        }
    }

    let name = ctx.get_script_or_module_name()?;
    let name = name.trim_start_matches(super::super::CJS_IMPORT_PREFIX);
    resolve_path([name].iter())
}

fn build_search_paths(ctx: &Ctx<'_>, options: Opt<Object<'_>>) -> Result<Option<Vec<String>>> {
    let options = match options.0 {
        Some(options) => options,
        None => return Ok(None),
    };

    if !options.contains_key("paths")? {
        return Ok(None);
    }

    let paths_value: Value = options.get("paths")?;
    if paths_value.is_null() || paths_value.is_undefined() {
        return Ok(None);
    }

    let array = paths_value
        .as_array()
        .ok_or_else(|| Exception::throw_type(ctx, "options.paths must be an array"))?;
    let mut paths = Vec::with_capacity(array.len());
    for value in array.iter::<String>() {
        paths.push(value?);
    }

    Ok(Some(paths))
}

pub fn default_resolve_filename(
    ctx: Ctx<'_>,
    request: String,
    parent: Opt<Value<'_>>,
    _is_main: Opt<bool>,
    options: Opt<Object<'_>>,
) -> Result<String> {
    let parent_object = match parent.0 {
        Some(value) if value.is_null() || value.is_undefined() => None,
        Some(value) => value.as_object().map(|object| object.clone()),
        None => None,
    };

    if is_builtin_request(&ctx, &request) {
        return Ok(normalize_builtin_request(&request));
    }

    let parent_filename = canonical_parent_filename(&ctx, parent_object)?;
    let search_paths = build_search_paths(&ctx, options)?;
    let globals = ctx.globals();
    let hooked_fn: Option<Function> = globals.get("__require_hook").ok();

    match require_resolve_with_options(
        &ctx,
        &request,
        &parent_filename,
        hooked_fn,
        false,
        search_paths,
    ) {
        Ok(resolved) => Ok(resolved.into_owned()),
        Err(_) => module_not_found(&ctx, &request, &parent_filename).map(|_| unreachable!()),
    }
}

pub fn call_resolve_filename<'js>(
    ctx: &Ctx<'js>,
    request: &str,
    parent: Option<Object<'js>>,
    options: Option<Object<'js>>,
) -> Result<String> {
    let facade = ctx.userdata::<RefCell<ModuleFacadeState>>().or_throw(ctx)?;
    let constructor = facade.borrow().constructor.clone();
    let resolve_fn: Function = constructor.get("_resolveFilename")?;

    let request = request.to_string();
    let options = match options {
        Some(options) => options.into_value(),
        None => Object::new(ctx.clone())?.into_value(),
    };
    let parent_value = match parent {
        Some(parent) => parent.into_value(),
        None => Value::new_null(ctx.clone()),
    };

    resolve_fn.call((request, parent_value, false, options))
}

fn populate_module_instance<'js>(
    ctx: &Ctx<'js>,
    module: &Object<'js>,
    id: &str,
    filename: &str,
    parent: Option<Object<'js>>,
) -> Result<()> {
    let dir = path::dirname(filename);
    let node_paths = node_module_paths(&dir).unwrap_or_default();
    let paths = Array::new(ctx.clone())?;
    for (index, entry) in node_paths.into_iter().enumerate() {
        paths.set(index, entry)?;
    }
    let exports = Object::new(ctx.clone())?;
    let children = Array::new(ctx.clone())?;

    module.set("id", id)?;
    module.set("filename", filename)?;
    module.set("path", dir)?;
    module.set("paths", paths)?;
    module.set("exports", exports)?;
    module.set("loaded", false)?;
    module.set("children", children)?;
    match parent {
        Some(parent) => module.set("parent", parent)?,
        None => module.set("parent", Value::new_null(ctx.clone()))?,
    }
    Ok(())
}

fn link_child_to_parent<'js>(parent: &Object<'js>, child: &Object<'js>) -> Result<()> {
    let children: Array = parent.get("children")?;
    children.set(children.len(), child.clone())?;
    Ok(())
}

fn module_constructor<'js>(
    ctx: Ctx<'js>,
    id: String,
    parent: Opt<Value<'js>>,
) -> Result<Object<'js>> {
    let parent_object = match parent.0 {
        Some(value) if value.is_null() || value.is_undefined() => None,
        Some(value) => Some(value.into_object().ok_or_else(|| {
            Exception::throw_type(
                &ctx,
                "The \"parent\" argument must be an instance of Module",
            )
        })?),
        None => None,
    };
    let filename = if path::is_absolute(&id) {
        path::replace_backslash(id)
    } else {
        resolve_path([id.as_str()].iter())?
    };
    let module = Object::new(ctx.clone())?;
    populate_module_instance(&ctx, &module, &filename, &filename, parent_object)?;
    Ok(module)
}

fn module_prototype_require<'js>(
    ctx: Ctx<'js>,
    this: This<Object<'js>>,
    request: String,
) -> Result<Value<'js>> {
    super::require::require_from_module(ctx, this.0.clone(), request, true)
}

fn node_module_paths_fn(_ctx: Ctx<'_>, from: String) -> Result<Vec<String>> {
    let from = if from.is_empty() {
        ".".to_string()
    } else {
        from
    };
    node_module_paths(&from)
}

fn require_resolve_fn<'js>(
    ctx: Ctx<'js>,
    request: String,
    options: Opt<Object<'js>>,
    parent: Opt<Value<'js>>,
) -> Result<String> {
    let parent_object = match parent.0 {
        Some(value) if value.is_null() || value.is_undefined() => None,
        Some(value) => value.as_object().map(|object| object.clone()),
        None => None,
    };
    call_resolve_filename(&ctx, &request, parent_object, options.0)
}

fn global_require_impl<'js>(ctx: Ctx<'js>, request: String) -> Result<Value<'js>> {
    let current_module = ctx
        .userdata::<RefCell<RequireState>>()
        .or_throw(&ctx)?
        .borrow()
        .current_module
        .clone();
    if let Some(current) = current_module {
        return super::require::require_from_module(ctx, current, request, true);
    }

    let parent_filename = canonical_parent_filename(&ctx, None)?;
    let parent = get_or_create_module_record(&ctx, &parent_filename, None)?;
    super::require::require_from_module(ctx, parent, request, false)
}

fn global_require_resolve_impl<'js>(
    ctx: Ctx<'js>,
    request: String,
    options: Opt<Object<'js>>,
) -> Result<String> {
    if request.starts_with("node:") && is_builtin_request(&ctx, &request) {
        return Ok(request);
    }
    require_resolve_fn(ctx, request, options, Opt(None))
}

fn require_from_filename<'js>(
    ctx: Ctx<'js>,
    parent_filename: String,
    request: String,
) -> Result<Value<'js>> {
    let parent = get_or_create_module_record(&ctx, &parent_filename, None)?;
    super::require::require_from_module(ctx, parent, request, true)
}

fn resolve_from_filename<'js>(
    ctx: Ctx<'js>,
    parent_filename: String,
    request: String,
    options: Opt<Object<'js>>,
) -> Result<String> {
    if request.starts_with("node:") && is_builtin_request(&ctx, &request) {
        return Ok(request);
    }
    let parent = get_or_create_module_record(&ctx, &parent_filename, None)?;
    call_resolve_filename(&ctx, &request, Some(parent), options.0)
}

fn install_create_require_factory(ctx: &Ctx<'_>) -> Result<()> {
    ctx.globals()
        .set("__rasterRequireFrom", Func::from(require_from_filename))?;
    ctx.globals()
        .set("__rasterResolveFrom", Func::from(resolve_from_filename))?;
    ctx.eval::<(), _>(
        r#"
        globalThis.__rasterCreateRequire = function(filename) {
            function req(request) {
                return __rasterRequireFrom(filename, request);
            }
            req.resolve = function(request, options) {
                return __rasterResolveFrom(filename, request, options || {});
            };
            Object.defineProperty(req, "cache", {
                get() { return require.cache; },
                configurable: true,
                enumerable: true,
            });
            return req;
        };
        "#,
    )?;
    Ok(())
}

fn normalize_create_require_filename(filename: String) -> Result<String> {
    let filename = if filename.starts_with("file://") {
        filename.trim_start_matches("file://").to_string()
    } else {
        filename
    };

    if path::is_absolute(&filename) {
        Ok(path::replace_backslash(filename))
    } else {
        resolve_path([filename.as_str()].iter())
    }
}

fn create_require_filename_from_value(ctx: &Ctx<'_>, filename: Value<'_>) -> Result<String> {
    if let Some(url) = filename.as_string() {
        return normalize_create_require_filename(url.to_string()?);
    }

    if let Some(obj) = filename.as_object() {
        if let Ok(href) = obj.get::<_, String>("href") {
            return normalize_create_require_filename(href);
        }
        if let Ok(to_string) = obj.get::<_, Function>("toString") {
            let href = to_string.call::<_, String>(())?;
            return normalize_create_require_filename(href);
        }
    }

    Err(Exception::throw_type(
        ctx,
        "The argument 'filename' must be a file URL object, file URL string, or absolute path string",
    ))
}

fn create_require_impl<'js>(ctx: Ctx<'js>, filename: Value<'js>) -> Result<Value<'js>> {
    let filename = create_require_filename_from_value(&ctx, filename)?;

    let factory: Function = ctx.globals().get("__rasterCreateRequire")?;
    factory.call((filename,))
}

pub fn get_or_create_module_record<'js>(
    ctx: &Ctx<'js>,
    filename: &str,
    parent: Option<Object<'js>>,
) -> Result<Object<'js>> {
    let facade = ctx.userdata::<RefCell<ModuleFacadeState>>().or_throw(ctx)?;
    let constructor = facade.borrow().constructor.clone();
    let cache = facade.borrow().cache.clone();

    if let Ok(record) = cache.get::<_, Object>(filename) {
        return Ok(record);
    }

    let record = module_constructor(
        ctx.clone(),
        filename.to_string(),
        Opt(parent.clone().map(|p| p.into_value())),
    )?;
    let prototype: Object = constructor.get("prototype")?;
    record.set_prototype(Some(&prototype))?;
    cache.set(filename, record.clone())?;

    if let Some(parent) = parent {
        link_child_to_parent(&parent, &record)?;
    }

    Ok(record)
}

pub fn init_module_facade<'js>(
    ctx: &Ctx<'js>,
    module_list: HashSet<String>,
) -> Result<Constructor<'js>> {
    let prototype = Object::new(ctx.clone())?;
    prototype.set("require", Func::from(module_prototype_require))?;

    let module_ctor = Constructor::new_prototype(ctx, prototype, module_constructor)?;

    let builtin_modules = Array::new(ctx.clone())?;
    let mut sorted_modules: Vec<_> = module_list.iter().collect();
    sorted_modules.sort_unstable();
    for (index, name) in sorted_modules.into_iter().enumerate() {
        builtin_modules.set(index, name.as_str())?;
    }

    let cache = Object::new(ctx.clone())?;
    module_ctor.set("_cache", cache.clone())?;
    module_ctor.set("_nodeModulePaths", Func::from(node_module_paths_fn))?;
    module_ctor.set("_resolveFilename", Func::from(default_resolve_filename))?;
    module_ctor.set("builtinModules", builtin_modules.clone())?;
    module_ctor.set("createRequire", Func::from(create_require_impl))?;
    module_ctor.set(
        "isBuiltin",
        Func::from(|ctx: Ctx<'_>, name: String| -> Result<bool> {
            Ok(is_builtin_request(&ctx, &name))
        }),
    )?;
    module_ctor.set("registerHooks", Func::from(register_hooks))?;
    module_ctor.set("Module", module_ctor.clone())?;

    install_create_require_factory(ctx)?;

    ctx.store_userdata(RefCell::new(ModuleFacadeState {
        constructor: module_ctor.clone(),
        cache,
    }))?;

    Ok(module_ctor)
}

pub fn init_global_require(ctx: &Ctx<'_>) -> Result<()> {
    let facade = ctx.userdata::<RefCell<ModuleFacadeState>>().or_throw(ctx)?;
    let cache = facade.borrow().cache.clone();

    let require_fn = Function::new(ctx.clone(), global_require_impl)?;
    let resolve_fn = Function::new(ctx.clone(), global_require_resolve_impl)?;
    require_fn.set("resolve", resolve_fn)?;
    require_fn.set("cache", cache)?;

    ctx.globals().set("require", require_fn)?;
    Ok(())
}
