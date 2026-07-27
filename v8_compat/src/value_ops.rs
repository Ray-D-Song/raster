use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

use crate::bridge::{with_state, with_state_ref, RasterV8Status};

fn value_from_root(state: &crate::bridge::BridgeState, root: u64) -> Option<rquickjs::qjs::JSValue> {
    state.roots.get(root)
}

#[repr(u8)]
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum ValueLayoutKind {
    Object = 0,
    String = 1,
    Oddball = 2,
    HeapNumber = 3,
    Int32Smi = 4,
}

pub unsafe extern "C" fn value_layout_kind(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out_kind: *mut u8,
    out_smi: *mut i32,
) -> RasterV8Status {
    if out_kind.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        let kind = if rquickjs::qjs::JS_IsString(val) {
            ValueLayoutKind::String
        } else if rquickjs::qjs::JS_IsBool(val)
            || rquickjs::qjs::JS_IsNull(val)
            || rquickjs::qjs::JS_IsUndefined(val)
        {
            ValueLayoutKind::Oddball
        } else if unsafe { rquickjs::qjs::JS_VALUE_GET_NORM_TAG(val) == rquickjs::qjs::JS_TAG_INT } {
            ValueLayoutKind::Int32Smi
        } else if rquickjs::qjs::JS_IsNumber(val) {
            ValueLayoutKind::HeapNumber
        } else {
            ValueLayoutKind::Object
        };
        *out_kind = kind as u8;
        if !out_smi.is_null() && kind == ValueLayoutKind::Int32Smi {
            let mut n = 0i32;
            if rquickjs::qjs::JS_ToInt32(state.ctx_ptr(), &mut n, val) < 0 {
                return RasterV8Status::Exception;
            }
            *out_smi = n;
        }
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn value_is_object(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out: *mut bool,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        *out = rquickjs::qjs::JS_IsObject(val);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn value_is_array(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out: *mut bool,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        *out = rquickjs::qjs::JS_IsArray(val);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn value_is_function(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out: *mut bool,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        *out = rquickjs::qjs::JS_IsFunction(state.ctx_ptr(), val);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn value_is_number(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out: *mut bool,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        *out = rquickjs::qjs::JS_IsNumber(val);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn value_is_int32(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out: *mut bool,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        *out = unsafe { rquickjs::qjs::JS_VALUE_GET_NORM_TAG(val) == rquickjs::qjs::JS_TAG_INT };
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn value_is_bigint(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out: *mut bool,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        *out = unsafe { rquickjs::qjs::JS_VALUE_GET_NORM_TAG(val) == rquickjs::qjs::JS_TAG_BIG_INT };
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn value_is_boolean(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out: *mut bool,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        *out = rquickjs::qjs::JS_IsBool(val);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn value_to_boolean(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out: *mut bool,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        *out = rquickjs::qjs::JS_ToBool(state.ctx_ptr(), val) != 0;
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn value_strict_equals(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    a: u64,
    b: u64,
    out: *mut bool,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(va) = value_from_root(state, a) else {
            return RasterV8Status::Error;
        };
        let Some(vb) = value_from_root(state, b) else {
            return RasterV8Status::Error;
        };
        *out = rquickjs::qjs::JS_IsSameValue(state.ctx_ptr(), va, vb);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn value_to_float64(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out: *mut f64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        let mut d = 0.0;
        if rquickjs::qjs::JS_ToFloat64(state.ctx_ptr(), &mut d, val) < 0 {
            return RasterV8Status::Exception;
        }
        *out = d;
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn value_to_int32(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out: *mut i32,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        let mut n = 0i32;
        if rquickjs::qjs::JS_ToInt32(state.ctx_ptr(), &mut n, val) < 0 {
            return RasterV8Status::Exception;
        }
        *out = n;
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn value_to_int64(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out: *mut i64,
    lossless: *mut bool,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        let mut n = 0i64;
        let rc = rquickjs::qjs::JS_ToBigInt64(state.ctx_ptr(), &mut n, val);
        if rc < 0 {
            return RasterV8Status::Exception;
        }
        *out = n;
        if !lossless.is_null() {
            *lossless = rc == 0;
        }
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn array_length(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out: *mut u32,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        let len = rquickjs::qjs::JS_GetPropertyStr(state.ctx_ptr(), val, c"length".as_ptr());
        if rquickjs::qjs::JS_IsException(len) {
            return RasterV8Status::Exception;
        }
        let mut n = 0i32;
        if rquickjs::qjs::JS_ToInt32(state.ctx_ptr(), &mut n, len) < 0 {
            rquickjs::qjs::JS_FreeValue(state.ctx_ptr(), len);
            return RasterV8Status::Exception;
        }
        rquickjs::qjs::JS_FreeValue(state.ctx_ptr(), len);
        *out = n.max(0) as u32;
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn function_new_instance(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    func_root: u64,
    argc: c_int,
    args: *const u64,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    let (ctx, func, mut argv) = match with_state_ref(|state| {
        let (_func_root, func) = match crate::bridge::resolve_constructor_root(state, func_root) {
            Some(pair) => pair,
            None => return Err(RasterV8Status::Error),
        };
        let ctx = state.ctx_ptr();
        let mut argv: Vec<rquickjs::qjs::JSValue> = Vec::with_capacity(argc.max(0) as usize);
        if !args.is_null() && argc > 0 {
            for &root in std::slice::from_raw_parts(args, argc as usize) {
                let Some(arg) = value_from_root(state, root) else {
                    return Err(RasterV8Status::Error);
                };
                argv.push(unsafe { rquickjs::qjs::JS_DupValue(ctx, arg) });
            }
        }
        Ok((ctx, func, argv))
    }) {
        Ok(v) => v,
        Err(status) => return status,
    };
    let result = unsafe {
        rquickjs::qjs::JS_CallConstructor(ctx, func, argc, argv.as_mut_ptr())
    };
    for arg in argv {
        unsafe { rquickjs::qjs::JS_FreeValue(ctx, arg) };
    }
    if unsafe { rquickjs::qjs::JS_IsException(result) } {
        return RasterV8Status::Exception;
    }
    with_state(|state| {
        *out = state.roots.insert(ctx, result);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn buffer_new_copy(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    data: *const u8,
    len: usize,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() || data.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let buf = rquickjs::qjs::JS_NewArrayBufferCopy(
            state.ctx_ptr(),
            data,
            len as u64,
        );
        if rquickjs::qjs::JS_IsException(buf) {
            return RasterV8Status::Exception;
        }
        *out = state.roots.insert(state.ctx_ptr(), buf);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn buffer_data(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> RasterV8Status {
    if out_ptr.is_null() || out_len.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        let mut len: u64 = 0;
        let ptr = rquickjs::qjs::JS_GetArrayBuffer(state.ctx_ptr(), &mut len, val);
        if ptr.is_null() {
            return RasterV8Status::Error;
        }
        *out_ptr = ptr;
        *out_len = len as usize;
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn buffer_has_instance(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root: u64,
    out: *mut bool,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = value_from_root(state, root) else {
            return RasterV8Status::Error;
        };
        let is_ab = rquickjs::qjs::JS_IsArrayBuffer(val);
        let is_u8 = if !is_ab {
            let global = rquickjs::qjs::JS_GetGlobalObject(state.ctx_ptr());
            let u8_ctor = rquickjs::qjs::JS_GetPropertyStr(state.ctx_ptr(), global, c"Uint8Array".as_ptr());
            rquickjs::qjs::JS_FreeValue(state.ctx_ptr(), global);
            if rquickjs::qjs::JS_IsException(u8_ctor) {
                rquickjs::qjs::JS_FreeValue(state.ctx_ptr(), u8_ctor);
                false
            } else {
                let ok = rquickjs::qjs::JS_IsInstanceOf(state.ctx_ptr(), val, u8_ctor);
                rquickjs::qjs::JS_FreeValue(state.ctx_ptr(), u8_ctor);
                ok != 0
            }
        } else {
            false
        };
        *out = is_ab || is_u8;
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn add_env_cleanup_hook(
    _isolate: *mut crate::bridge::RasterV8IsolateState,
    _cb: Option<unsafe extern "C" fn(*mut c_void)>,
    _arg: *mut c_void,
) {
}
