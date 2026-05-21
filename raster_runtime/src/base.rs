// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub use self::base::*;

#[allow(clippy::module_inception)]
mod base {
    pub use raster_runtime_core::bytecode;
    pub use raster_runtime_core::compiler;
    pub use raster_runtime_core::environment;
    pub use raster_runtime_core::libs;
    pub use raster_runtime_core::modules;
    pub use raster_runtime_core::vm;
}
pub use raster_runtime_core::VERSION;

// rquickjs components
#[allow(unused_imports)]
pub use raster_runtime_core::{
    atom::PredefinedAtom, context::EvalOptions, function::Rest, AsyncContext, CatchResultExt, Ctx,
    Error, Object, Promise,
};
