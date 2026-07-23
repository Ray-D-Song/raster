// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{prelude::Func, Ctx, Result, Value};
use tracing::trace;

use super::{dispatch_destroy_hooks, remove_id_map};

pub(crate) fn init_finalization_registry(ctx: &Ctx<'_>) -> Result<()> {
    let global = ctx.globals();

    global.set(
        "__invokeFinalizationHook",
        Func::from(invoke_finalization_hook),
    )?;

    let _: () = ctx.eval(
        r#"
        globalThis.asyncFinalizationRegistry = (() => {
            const registry = new FinalizationRegistry(__invokeFinalizationHook);
            return {
                register(target, heldValue) {
                    registry.register(target, heldValue);
                }
            };
        })();
        "#,
    )?;

    global.remove("__invokeFinalizationHook")?;

    Ok(())
}

fn invoke_finalization_hook<'js>(ctx: Ctx<'js>, uid: Value<'js>) -> Result<()> {
    let uid = uid.as_number().unwrap() as usize;

    let resource_id = remove_id_map(&ctx, uid)?;
    if resource_id.0 == 0 {
        return Ok(());
    }

    // Destroy must not change current_id.
    trace!("Destroy[{}](async_id, trigger_id): {:?}", uid, resource_id);

    dispatch_destroy_hooks(&ctx, resource_id.0)?;
    Ok(())
}
