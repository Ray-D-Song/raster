//! JNI bridge for `dev.raster.plugin.RasterPlugin` Kotlin SDK.

use std::ffi::c_void;
use std::sync::Mutex;

use gpui_mobile::android::jni::{self as jni_helpers};
use jni::objects::{JObject, JValue};

use crate::plugin::ffi::plugin_host;
use crate::plugin::host::{RasterPluginCall, cstr_to_str};

static JAVA_ACTIVITY_PTR: Mutex<u64> = Mutex::new(0);

extern "C" fn android_plugin_trampoline(call: *const RasterPluginCall) {
    let Some(call) = (unsafe { call.as_ref() }) else {
        return;
    };
    let call_id = call.call_id;
    let plugin = cstr_to_str(call.plugin).unwrap_or("").to_owned();
    let method = cstr_to_str(call.method).unwrap_or("").to_owned();
    let args_json = cstr_to_str(call.args_json).unwrap_or("null").to_owned();

    let _ = jni_helpers::with_env(|env| {
        let cls = jni_helpers::find_app_class(env, "dev.raster.plugin.RasterPlugin")?;
        let j_plugin = env
            .new_string(&plugin)
            .map_err(|error| error.to_string())?;
        let j_method = env
            .new_string(&method)
            .map_err(|error| error.to_string())?;
        let j_args = env
            .new_string(&args_json)
            .map_err(|error| error.to_string())?;
        env.call_static_method(
            &cls,
            jni::jni_str!("dispatchFromNative"),
            jni::jni_sig!("(JLjava/lang/String;Ljava/lang/String;Ljava/lang/String;)V"),
            &[
                JValue::Long(i64::try_from(call_id).unwrap_or(i64::MAX)),
                JValue::Object(&j_plugin),
                JValue::Object(&j_method),
                JValue::Object(&j_args),
            ],
        )
        .map_err(|error| {
            env.exception_clear();
            error.to_string()
        })?;
        Ok(())
    });
}

fn jstring_to_rust(env: &mut jni::Env<'_>, value: *mut c_void) -> String {
    if value.is_null() {
        return String::new();
    }
    let raw = value as jni::sys::jobject;
    unsafe {
        let obj = JObject::from_raw(env, raw);
        jni_helpers::get_string(env, &obj)
    }
}

pub fn bind_plugin_activity() {
    let _ = jni_helpers::with_env(|env| {
        let activity = jni_helpers::activity(env)?;
        let cls = jni_helpers::find_app_class(env, "dev.raster.plugin.RasterPlugin")?;
        env.call_static_method(
            &cls,
            jni::jni_str!("bindActivity"),
            jni::jni_sig!("(Landroid/app/Activity;)V"),
            &[JValue::Object(&activity)],
        )
        .map_err(|error| {
            env.exception_clear();
            error.to_string()
        })?;
        Ok(())
    });
}

pub fn register_linked_plugins() {
    let _ = jni_helpers::with_env(|env| {
        let cls = jni_helpers::find_app_class(env, "dev.raster.generated.RasterPlugins")?;
        env.call_static_method(
            &cls,
            jni::jni_str!("registerAll"),
            jni::jni_sig!("()V"),
            &[],
        )
        .map_err(|error| {
            env.exception_clear();
            error.to_string()
        })?;
        crate::common::utils::logger::info("registered linked Raster plugins");
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_raster_plugin_RasterPlugin_nativeRegister(
    env: *mut c_void,
    _class: *mut c_void,
    plugin: *mut c_void,
    method: *mut c_void,
) {
    let Some(state) = plugin_host() else {
        return;
    };
    let _ = jni_helpers::with_env(|env| {
        let plugin_name = jstring_to_rust(env, plugin);
        let method_name = jstring_to_rust(env, method);
        if plugin_name.is_empty() || method_name.is_empty() {
            return Ok(());
        }
        state.host().register_method(
            &plugin_name,
            &method_name,
            android_plugin_trampoline,
            std::ptr::null_mut(),
        );
        Ok(())
    });
    let _ = env;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_raster_plugin_RasterPlugin_nativeReplyOk(
    _env: *mut c_void,
    _class: *mut c_void,
    call_id: i64,
    result_json: *mut c_void,
) {
    let Some(state) = plugin_host() else {
        return;
    };
    let _ = jni_helpers::with_env(|env| {
        let result = jstring_to_rust(env, result_json);
        if call_id < 0 {
            return Ok(());
        }
        state
            .host()
            .reply_ok(u64::try_from(call_id).unwrap_or(u64::MAX), &result);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_raster_plugin_RasterPlugin_nativeReplyErr(
    _env: *mut c_void,
    _class: *mut c_void,
    call_id: i64,
    code: *mut c_void,
    message: *mut c_void,
) {
    let Some(state) = plugin_host() else {
        return;
    };
    let _ = jni_helpers::with_env(|env| {
        let code = jstring_to_rust(env, code);
        let message = jstring_to_rust(env, message);
        if call_id < 0 {
            return Ok(());
        }
        state.host().reply_err(
            u64::try_from(call_id).unwrap_or(u64::MAX),
            if code.is_empty() { "PLUGIN_ERROR" } else { &code },
            if message.is_empty() { "plugin call failed" } else { &message },
        );
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_raster_plugin_RasterPlugin_nativeEmitEvent(
    _env: *mut c_void,
    _class: *mut c_void,
    plugin: *mut c_void,
    event: *mut c_void,
    data_json: *mut c_void,
) {
    let Some(state) = plugin_host() else {
        return;
    };
    let _ = jni_helpers::with_env(|env| {
        let plugin = jstring_to_rust(env, plugin);
        let event = jstring_to_rust(env, event);
        let data_json = jstring_to_rust(env, data_json);
        if plugin.is_empty() || event.is_empty() {
            return Ok(());
        }
        state.host().emit_event(
            &plugin,
            &event,
            if data_json.is_empty() { "null" } else { &data_json },
        );
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_raster_plugin_RasterPlugin_nativeCurrentActivity(
    _env: *mut c_void,
    _class: *mut c_void,
) -> i64 {
    let pointer = crate::android::raster_android_current_activity() as u64;
    if pointer != 0 {
        return i64::try_from(pointer).unwrap_or(0);
    }
    JAVA_ACTIVITY_PTR
        .lock()
        .map(|guard| i64::try_from(*guard).unwrap_or(0))
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_raster_plugin_RasterPlugin_nativeSetCurrentActivity(
    _env: *mut c_void,
    _class: *mut c_void,
    activity: *mut c_void,
) {
    if activity.is_null() {
        return;
    }
    let pointer = activity as u64;
    if let Ok(mut guard) = JAVA_ACTIVITY_PTR.lock() {
        *guard = pointer;
    }
}