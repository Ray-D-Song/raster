use llrt_core::modules::module_builder::ModuleBuilder;

use crate::js_runtime::bundle::{
    RasterComponentModule, RasterComponentsModule, RasterCoreModule, RasterModule,
    RasterReactModule, ReactJsxRuntimeModule, ReactModule, ReactRasterModule,
};

pub fn build_module_builder() -> ModuleBuilder {
    ModuleBuilder::default()
        .with_module(ReactModule)
        .with_module(ReactJsxRuntimeModule)
        .with_module(ReactRasterModule)
        .with_module(RasterReactModule)
        .with_module(RasterModule)
        .with_module(RasterCoreModule)
        .with_module(RasterComponentsModule)
        .with_module(RasterComponentModule)
}
