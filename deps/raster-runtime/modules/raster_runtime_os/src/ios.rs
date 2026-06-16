// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::ffi::CStr;

use once_cell::sync::Lazy;
use rquickjs::{
    prelude::{Opt, Rest},
    Ctx, Exception, IntoJs, Null, Object, Result, Value,
};

use crate::get_home_dir;

static OS_INFO: Lazy<(String, String, String)> = Lazy::new(uname);
pub static EOL: &str = "\n";
pub static DEV_NULL: &str = "/dev/null";

pub fn get_priority(_who: Opt<u32>) -> i32 {
    0
}

pub fn set_priority(ctx: Ctx<'_>, _args: Rest<Value>) -> Result<()> {
    Err(Exception::throw_syntax(
        &ctx,
        "setPriority is not implemented on iOS.",
    ))
}

pub fn get_type() -> &'static str {
    &OS_INFO.0
}

pub fn get_user_info<'js>(ctx: Ctx<'js>, _options: Opt<Value>) -> Result<Object<'js>> {
    let obj = Object::new(ctx.clone())?;

    obj.set("uid", Null.into_js(&ctx)?)?;
    obj.set("gid", Null.into_js(&ctx)?)?;
    obj.set("username", Null.into_js(&ctx)?)?;
    obj.set("homedir", get_home_dir(ctx.clone()))?;
    obj.set("shell", Null.into_js(&ctx)?)?;
    Ok(obj)
}

pub fn get_release() -> &'static str {
    &OS_INFO.1
}

pub fn get_version() -> &'static str {
    &OS_INFO.2
}

fn uname() -> (String, String, String) {
    let mut info = std::mem::MaybeUninit::uninit();
    let res = unsafe { libc::uname(info.as_mut_ptr()) };
    if res != 0 {
        return ("iOS".to_owned(), String::new(), String::new());
    }
    let info = unsafe { info.assume_init() };
    (
        unsafe {
            CStr::from_ptr(info.sysname.as_ptr())
                .to_string_lossy()
                .into_owned()
        },
        unsafe {
            CStr::from_ptr(info.release.as_ptr())
                .to_string_lossy()
                .into_owned()
        },
        unsafe {
            CStr::from_ptr(info.version.as_ptr())
                .to_string_lossy()
                .into_owned()
        },
    )
}
