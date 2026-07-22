// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    rc::Rc,
};

use raster_runtime_utils::{ctx::CtxExt, module::ModuleInfo, result::ResultExt};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    object::Accessor,
    prelude::Func,
    Ctx, Error, Exception, Function, JsLifetime, Module, Object, Result, Value,
};

mod current_module;
pub mod extensions;
pub mod facade;
pub mod import_load;
pub mod loader;
mod require;
pub mod resolver;

use crate::CJS_IMPORT_PREFIX;

use facade::init_global_require;
use facade::init_module_facade;

#[derive(JsLifetime)]
pub struct ModuleNames<'js> {
    list: HashSet<String>,
    _marker: PhantomData<&'js ()>,
}

impl ModuleNames<'_> {
    pub fn new(names: HashSet<String>) -> Self {
        Self {
            list: names,
            _marker: PhantomData,
        }
    }

    pub fn get_list(&self) -> HashSet<String> {
        self.list.clone()
    }
}

#[derive(Default)]
pub struct RequireState<'js> {
    pub cache: HashMap<Rc<str>, Value<'js>>,
    pub exports: HashMap<Rc<str>, Value<'js>>,
    pub progress: HashMap<Rc<str>, Object<'js>>,
    pub current_module: Option<Object<'js>>,
}

unsafe impl<'js> JsLifetime<'js> for RequireState<'js> {
    type Changed<'to> = RequireState<'to>;
}

#[derive(Clone, JsLifetime)]
struct Hook<'js> {
    resolve: Option<Function<'js>>,
    load: Option<Function<'js>>,
}

#[derive(JsLifetime)]
pub struct ModuleHookState<'js> {
    hooks: Vec<Hook<'js>>,
}

impl Default for ModuleHookState<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleHookState<'_> {
    fn new() -> Self {
        Self { hooks: Vec::new() }
    }
}

#[derive(Default)]
pub(crate) struct ModuleCache<'js> {
    pub(crate) esm: HashMap<Rc<str>, Module<'js>>,
}

unsafe impl<'js> JsLifetime<'js> for ModuleCache<'js> {
    type Changed<'to> = ModuleCache<'to>;
}

pub struct ModuleModule;

fn is_builtin(ctx: Ctx<'_>, name: String) -> Result<bool> {
    let module_list = ctx
        .userdata::<ModuleNames>()
        .ok_or_else(|| Exception::throw_reference(&ctx, "is_builtin is not supported"))?
        .get_list();

    let name = name.trim_start_matches("node:").trim_end_matches('/');
    Ok(module_list.contains(name))
}

pub fn register_hooks<'js>(ctx: Ctx<'js>, hooks_obj: Object<'js>) -> Result<()> {
    let resolve = hooks_obj.get::<_, Function>("resolve").ok();
    let load = hooks_obj.get::<_, Function>("load").ok();

    let hook = Hook { resolve, load };

    let binding = ctx.userdata::<RefCell<ModuleHookState>>().or_throw(&ctx)?;
    let mut state = binding.borrow_mut();
    state.hooks.push(hook);

    Ok(())
}

impl ModuleDef for ModuleModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("builtinModules")?;
        declare.declare("createRequire")?;
        declare.declare("isBuiltin")?;
        declare.declare("registerHooks")?;
        declare.declare("_nodeModulePaths")?;
        declare.declare("_resolveFilename")?;
        declare.declare("_cache")?;
        declare.declare("_extensions")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let module_list = ctx
            .userdata::<ModuleNames>()
            .map_or_else(HashSet::new, |v| v.get_list());

        let module_ctor = if let Some(facade) = ctx.userdata::<RefCell<facade::ModuleFacadeState>>()
        {
            facade.borrow().constructor.clone()
        } else {
            init_module_facade(ctx, module_list.clone())?
        };

        exports.export("default", module_ctor.clone())?;
        exports.export(
            "builtinModules",
            module_ctor.get::<_, Value>("builtinModules")?,
        )?;
        exports.export(
            "createRequire",
            module_ctor.get::<_, Value>("createRequire")?,
        )?;
        exports.export("isBuiltin", Func::from(is_builtin))?;
        exports.export("registerHooks", Func::from(register_hooks))?;
        exports.export(
            "_nodeModulePaths",
            module_ctor.get::<_, Value>("_nodeModulePaths")?,
        )?;
        exports.export(
            "_resolveFilename",
            module_ctor.get::<_, Value>("_resolveFilename")?,
        )?;
        exports.export("_cache", module_ctor.get::<_, Value>("_cache")?)?;
        exports.export("_extensions", module_ctor.get::<_, Value>("_extensions")?)?;

        Ok(())
    }
}

impl From<ModuleModule> for ModuleInfo<ModuleModule> {
    fn from(val: ModuleModule) -> Self {
        ModuleInfo {
            name: "module",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    ctx.store_userdata(RefCell::new(RequireState::default()))?;
    ctx.store_userdata(RefCell::new(ModuleHookState::default()))?;
    ctx.store_userdata(RefCell::new(ModuleCache::default()))?;

    let module_list = ctx
        .userdata::<ModuleNames>()
        .map_or_else(HashSet::new, |v| v.get_list());
    init_module_facade(ctx, module_list)?;

    let exports_accessor = Accessor::new(
        |ctx| {
            struct Args<'js>(Ctx<'js>);
            let Args(ctx) = Args(ctx);
            let name = ctx.get_script_or_module_name()?;
            let name = name.trim_start_matches(CJS_IMPORT_PREFIX);

            let binding = ctx.userdata::<RefCell<RequireState>>().unwrap();
            let mut state = binding.borrow_mut();

            if let Some(value) = state.exports.get(name) {
                Ok::<_, Error>(value.clone())
            } else {
                let obj = Object::new(ctx.clone())?.into_value();
                state.exports.insert(name.into(), obj.clone());
                Ok::<_, Error>(obj)
            }
        },
        |ctx, exports| {
            struct Args<'js>(Ctx<'js>, Value<'js>);
            let Args(ctx, exports) = Args(ctx, exports);
            let name = ctx.get_script_or_module_name()?;
            let name = name.trim_start_matches(CJS_IMPORT_PREFIX);
            let binding = ctx.userdata::<RefCell<RequireState>>().unwrap();
            let mut state = binding.borrow_mut();
            state.exports.insert(name.into(), exports);
            Ok::<_, Error>(())
        },
    )
    .configurable()
    .enumerable();

    init_global_require(ctx)?;

    let module_instance = facade::get_or_create_module_record(
        ctx,
        &facade::canonical_parent_filename(ctx, None)?,
        None,
    )?;
    module_instance.prop("exports", exports_accessor)?;
    globals.prop("module", module_instance)?;
    globals.prop("exports", exports_accessor)?;

    Ok(())
}
