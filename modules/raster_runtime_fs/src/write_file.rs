// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::errors::throw_fs_error;
use either::Either;
use raster_runtime_utils::{bytes::ObjectBytes, object::ObjectExt};
use rquickjs::{function::Opt, Ctx, Error, FromJs, Result, Value};
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub async fn write_file<'js>(
    ctx: Ctx<'js>,
    path: String,
    data: Value<'js>,
    options: Opt<Either<String, WriteFileOptions>>,
) -> Result<()> {
    let mut file = fs::File::create(&path)
        .await
        .map_err(|err| throw_fs_error(&ctx, err, "open", &path))?;

    #[cfg(unix)]
    if let Some(Either::Right(opts)) = options.0 {
        use std::os::unix::fs::PermissionsExt;

        let perm = PermissionsExt::from_mode(opts.mode.unwrap_or(0o666));
        file.set_permissions(perm)
            .await
            .map_err(|err| throw_fs_error(&ctx, err, "fchmod", &path))?;
    }
    #[cfg(not(unix))]
    {
        _ = options;
        if let Some(Either::Right(opts)) = options.0 {
            _ = opts.mode;
        }
    }

    let bytes = ObjectBytes::from(&ctx, &data)?;
    file.write_all(bytes.as_bytes(&ctx)?)
        .await
        .map_err(|err| throw_fs_error(&ctx, err, "write", &path))?;
    file.flush()
        .await
        .map_err(|err| throw_fs_error(&ctx, err, "write", &path))?;

    Ok(())
}

pub fn write_file_sync<'js>(
    ctx: Ctx<'js>,
    path: String,
    bytes: ObjectBytes<'js>,
    options: Opt<Either<String, WriteFileOptions>>,
) -> Result<()> {
    std::fs::write(&path, bytes.as_bytes(&ctx)?)
        .map_err(|err| throw_fs_error(&ctx, err, "open", &path))?;

    #[cfg(unix)]
    {
        if let Some(Either::Right(opts)) = options.0 {
            use std::os::unix::fs::PermissionsExt;

            std::fs::set_permissions(&path, PermissionsExt::from_mode(opts.mode.unwrap_or(0o666)))
                .map_err(|err| throw_fs_error(&ctx, err, "fchmod", &path))?;
        }
    }
    #[cfg(not(unix))]
    {
        _ = options;
        if let Some(Either::Right(opts)) = options.0 {
            _ = opts.mode;
        }
    }

    Ok(())
}

pub(crate) struct WriteFileOptions {
    pub mode: Option<u32>,
}

impl<'js> FromJs<'js> for WriteFileOptions {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        let mode = obj.get_optional::<_, u32>("mode")?;

        Ok(Self { mode })
    }
}
