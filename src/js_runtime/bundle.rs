use llrt_core::{
    Ctx, Function, Object, Result, Symbol, Value,
    module::{Declarations, Exports, ModuleDef},
};
use llrt_utils::module::ModuleInfo;

const RASTER_COMPONENT_EXPORTS: &[&str] = &[
    "createComponent",
    "Avatar",
    "AvatarGroup",
    "Alert",
    "Button",
    "ButtonGroup",
    "Checkbox",
    "ColorPicker",
    "DatePicker",
    "Dialog",
    "Field",
    "Form",
    "Icon",
    "LineChart",
    "BarChart",
    "AreaChart",
    "PieChart",
    "CandlestickChart",
    "Radio",
    "RadioGroup",
    "Select",
    "Sheet",
    "Switch",
    "Tab",
    "TabBar",
    "VirtualList",
];

pub fn install_runtime_bundle<'js>(ctx: Ctx<'js>) -> Result<()> {
    ctx.eval::<(), _>(include_str!("../runtime/js/generated/runtime_bundle.js"))?;
    Ok(())
}

fn runtime_bundle<'js>(ctx: &Ctx<'js>) -> Result<Object<'js>> {
    ctx.globals().get::<_, Object<'js>>("__RasterBundle")
}

fn export_bundle_values<'js>(
    bundle: &Object<'js>,
    exports: &Exports<'js>,
    names: &[&str],
) -> Result<()> {
    for name in names {
        exports.export(*name, bundle.get::<_, Value<'js>>(*name)?)?;
    }
    Ok(())
}

fn declare_react_renderer(declare: &Declarations) -> Result<()> {
    declare.declare("createRoot")?;
    declare.declare("createRasterRoot")?;
    Ok(())
}

fn evaluate_react_renderer<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
    let bundle = runtime_bundle(ctx)?;
    exports.export("createRoot", bundle.get::<_, Function<'js>>("createRoot")?)?;
    exports.export(
        "createRasterRoot",
        bundle.get::<_, Function<'js>>("createRasterRoot")?,
    )?;
    Ok(())
}

fn declare_raster_core(declare: &Declarations) -> Result<()> {
    declare.declare("ConfigProvider")?;
    declare.declare("Input")?;
    declare.declare("Label")?;
    declare.declare("Slot")?;
    declare.declare("Text")?;
    declare.declare("Textarea")?;
    declare.declare("View")?;
    declare.declare("Widget")?;
    Ok(())
}

fn evaluate_raster_core<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
    let bundle = runtime_bundle(ctx)?;
    exports.export(
        "ConfigProvider",
        bundle.get::<_, Function<'js>>("ConfigProvider")?,
    )?;
    exports.export("Input", bundle.get::<_, String>("Input")?)?;
    exports.export("Label", bundle.get::<_, String>("Label")?)?;
    exports.export("Slot", bundle.get::<_, String>("Slot")?)?;
    exports.export("Text", bundle.get::<_, String>("Text")?)?;
    exports.export("Textarea", bundle.get::<_, String>("Textarea")?)?;
    exports.export("View", bundle.get::<_, String>("View")?)?;
    exports.export("Widget", bundle.get::<_, Function<'js>>("Widget")?)?;
    Ok(())
}

fn declare_raster_component(declare: &Declarations) -> Result<()> {
    declare.declare("ConfigProvider")?;
    declare.declare("Input")?;
    declare.declare("Label")?;
    declare.declare("Text")?;
    declare.declare("Textarea")?;
    declare.declare("View")?;
    for name in RASTER_COMPONENT_EXPORTS {
        declare.declare(*name)?;
    }
    declare.declare("notification")?;
    Ok(())
}

fn evaluate_raster_component<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
    let bundle = runtime_bundle(ctx)?;
    exports.export(
        "ConfigProvider",
        bundle.get::<_, Function<'js>>("ConfigProvider")?,
    )?;
    exports.export("Input", bundle.get::<_, String>("Input")?)?;
    exports.export("Label", bundle.get::<_, String>("Label")?)?;
    exports.export("Text", bundle.get::<_, String>("Text")?)?;
    exports.export("Textarea", bundle.get::<_, String>("Textarea")?)?;
    exports.export("View", bundle.get::<_, String>("View")?)?;
    export_bundle_values(&bundle, exports, RASTER_COMPONENT_EXPORTS)?;
    exports.export(
        "notification",
        bundle.get::<_, Object<'js>>("notification")?,
    )?;
    Ok(())
}

pub struct ReactModule;

impl ModuleDef for ReactModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("Activity")?;
        declare.declare("Component")?;
        declare.declare("Suspense")?;
        declare.declare("useEffect")?;
        declare.declare("useRef")?;
        declare.declare("useState")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let bundle = runtime_bundle(ctx)?;
        exports.export("Activity", bundle.get::<_, Symbol<'js>>("Activity")?)?;
        exports.export("Component", bundle.get::<_, Function<'js>>("Component")?)?;
        exports.export("Suspense", bundle.get::<_, Symbol<'js>>("Suspense")?)?;
        exports.export("useEffect", bundle.get::<_, Function<'js>>("useEffect")?)?;
        exports.export("useRef", bundle.get::<_, Function<'js>>("useRef")?)?;
        exports.export("useState", bundle.get::<_, Function<'js>>("useState")?)?;
        Ok(())
    }
}

impl From<ReactModule> for ModuleInfo<ReactModule> {
    fn from(val: ReactModule) -> Self {
        Self {
            name: "react",
            module: val,
        }
    }
}

pub struct ReactJsxRuntimeModule;

impl ModuleDef for ReactJsxRuntimeModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("jsx")?;
        declare.declare("jsxs")?;
        declare.declare("Fragment")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let bundle = runtime_bundle(ctx)?;
        exports.export("jsx", bundle.get::<_, Function<'js>>("jsx")?)?;
        exports.export("jsxs", bundle.get::<_, Function<'js>>("jsxs")?)?;
        exports.export("Fragment", bundle.get::<_, Symbol<'js>>("Fragment")?)?;
        Ok(())
    }
}

impl From<ReactJsxRuntimeModule> for ModuleInfo<ReactJsxRuntimeModule> {
    fn from(val: ReactJsxRuntimeModule) -> Self {
        Self {
            name: "react/jsx-runtime",
            module: val,
        }
    }
}

macro_rules! react_renderer_module {
    ($type_name:ident, $module_name:literal) => {
        pub struct $type_name;

        impl ModuleDef for $type_name {
            fn declare(declare: &Declarations) -> Result<()> {
                declare_react_renderer(declare)
            }

            fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
                evaluate_react_renderer(ctx, exports)
            }
        }

        impl From<$type_name> for ModuleInfo<$type_name> {
            fn from(val: $type_name) -> Self {
                Self {
                    name: $module_name,
                    module: val,
                }
            }
        }
    };
}

macro_rules! raster_core_module {
    ($type_name:ident, $module_name:literal) => {
        pub struct $type_name;

        impl ModuleDef for $type_name {
            fn declare(declare: &Declarations) -> Result<()> {
                declare_raster_core(declare)
            }

            fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
                evaluate_raster_core(ctx, exports)
            }
        }

        impl From<$type_name> for ModuleInfo<$type_name> {
            fn from(val: $type_name) -> Self {
                Self {
                    name: $module_name,
                    module: val,
                }
            }
        }
    };
}

macro_rules! raster_component_module {
    ($type_name:ident, $module_name:literal) => {
        pub struct $type_name;

        impl ModuleDef for $type_name {
            fn declare(declare: &Declarations) -> Result<()> {
                declare_raster_component(declare)
            }

            fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
                evaluate_raster_component(ctx, exports)
            }
        }

        impl From<$type_name> for ModuleInfo<$type_name> {
            fn from(val: $type_name) -> Self {
                Self {
                    name: $module_name,
                    module: val,
                }
            }
        }
    };
}

react_renderer_module!(ReactRasterModule, "react-raster");
react_renderer_module!(RasterReactModule, "raster/react");
raster_core_module!(RasterModule, "raster");
raster_core_module!(RasterCoreModule, "raster/core");
raster_component_module!(RasterComponentsModule, "raster/components");
raster_component_module!(RasterComponentModule, "raster/component");
