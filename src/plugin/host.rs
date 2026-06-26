use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::sync::Mutex;

use anyhow::{Context as _, anyhow};

use crate::bridge::envelope::BridgeEnvelope;
use crate::bridge::state::SharedBridgeState;
use crate::bridge::value::BridgeValue;
use crate::plugin::json::{bridge_value_from_json, bridge_value_to_json_string, parse_json_args};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MethodKey {
    plugin: String,
    method: String,
}

#[derive(Clone, Copy)]
struct RegisteredHandler {
    handler: extern "C" fn(*const RasterPluginCall),
    context: *mut c_void,
}

unsafe impl Send for RegisteredHandler {}
unsafe impl Sync for RegisteredHandler {}

#[repr(C)]
pub struct RasterPluginCall {
    pub call_id: u64,
    pub plugin: *const std::os::raw::c_char,
    pub method: *const std::os::raw::c_char,
    pub args_json: *const std::os::raw::c_char,
    pub context: *mut c_void,
}

pub struct PluginHost {
    bridge: SharedBridgeState,
    handlers: Mutex<HashMap<MethodKey, RegisteredHandler>>,
    pending: Mutex<HashSet<u64>>,
    call_strings: Mutex<HashMap<u64, CallStrings>>,
}

struct CallStrings {
    plugin: CString,
    method: CString,
    args_json: CString,
}

impl PluginHost {
    pub fn new(bridge: SharedBridgeState) -> Self {
        Self {
            bridge,
            handlers: Mutex::new(HashMap::new()),
            pending: Mutex::new(HashSet::new()),
            call_strings: Mutex::new(HashMap::new()),
        }
    }

    pub fn register_method(
        &self,
        plugin: &str,
        method: &str,
        handler: extern "C" fn(*const RasterPluginCall),
        context: *mut c_void,
    ) -> bool {
        let Ok(mut handlers) = self.handlers.lock() else {
            return false;
        };
        handlers.insert(
            MethodKey {
                plugin: plugin.to_owned(),
                method: method.to_owned(),
            },
            RegisteredHandler { handler, context },
        );
        true
    }

    pub fn invoke(
        &self,
        call_id: u64,
        plugin: &str,
        method: &str,
        args: BridgeValue,
    ) -> anyhow::Result<()> {
        let handler = {
            let handlers = self
                .handlers
                .lock()
                .map_err(|_| anyhow!("plugin registry lock poisoned"))?;
            handlers
                .get(&MethodKey {
                    plugin: plugin.to_owned(),
                    method: method.to_owned(),
                })
                .copied()
        };

        let Some(handler) = handler else {
            self.reply_err(
                call_id,
                "UNIMPLEMENTED",
                &format!("{plugin}.{method} is not registered"),
            );
            return Ok(());
        };

        let args_json = bridge_value_to_json_string(&args);
        let plugin_c = CString::new(plugin).context("plugin name contains nul byte")?;
        let method_c = CString::new(method).context("method name contains nul byte")?;
        let args_c = CString::new(args_json).context("args json contains nul byte")?;

        {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| anyhow!("plugin pending lock poisoned"))?;
            pending.insert(call_id);
        }

        {
            let mut call_strings = self
                .call_strings
                .lock()
                .map_err(|_| anyhow!("plugin call string lock poisoned"))?;
            call_strings.insert(
                call_id,
                CallStrings {
                    plugin: plugin_c,
                    method: method_c,
                    args_json: args_c,
                },
            );
        }

        let call = {
            let call_strings = self
                .call_strings
                .lock()
                .map_err(|_| anyhow!("plugin call string lock poisoned"))?;
            let strings = call_strings
                .get(&call_id)
                .ok_or_else(|| anyhow!("missing call strings for id {call_id}"))?;
            RasterPluginCall {
                call_id,
                plugin: strings.plugin.as_ptr(),
                method: strings.method.as_ptr(),
                args_json: strings.args_json.as_ptr(),
                context: handler.context,
            }
        };

        (handler.handler)(&call as *const RasterPluginCall);
        Ok(())
    }

    pub fn reply_ok(&self, call_id: u64, result_json: &str) {
        let Some(payload) = parse_result_json(result_json) else {
            self.reply_err(call_id, "INVALID_RESULT", "plugin result is not valid JSON");
            return;
        };
        self.complete_call(call_id, Ok(payload));
    }

    pub fn reply_err(&self, call_id: u64, code: &str, message: &str) {
        let error = serde_json::json!({ "code": code, "message": message }).to_string();
        self.complete_call(call_id, Err(error));
    }

    pub fn emit_event(&self, plugin: &str, event: &str, data_json: &str) {
        let data = serde_json::from_str::<serde_json::Value>(data_json)
            .map(bridge_value_from_json)
            .unwrap_or(BridgeValue::Null);
        let payload = BridgeValue::object([
            ("event", BridgeValue::string(event)),
            ("data", data),
        ]);
        let _ = self
            .bridge
            .send_egress(BridgeEnvelope::event("plugin.event", plugin, payload));
    }

    fn complete_call(&self, call_id: u64, result: Result<BridgeValue, String>) {
        let removed = self
            .pending
            .lock()
            .map(|mut pending| pending.remove(&call_id))
            .unwrap_or(false);
        let _ = self
            .call_strings
            .lock()
            .map(|mut strings| strings.remove(&call_id));

        if !removed {
            return;
        }

        let envelope = match result {
            Ok(payload) => BridgeEnvelope::reply_ok(call_id, payload),
            Err(error) => BridgeEnvelope::reply_err(call_id, error),
        };
        let _ = self.bridge.send_egress(envelope);
    }
}

fn parse_result_json(result_json: &str) -> Option<BridgeValue> {
    let value = serde_json::from_str::<serde_json::Value>(result_json).ok()?;
    Some(bridge_value_from_json(value))
}

pub fn cstr_to_str<'a>(ptr: *const std::os::raw::c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().ok()
}

pub fn handle_plugin_invoke(
    host: &PluginHost,
    call_id: u64,
    payload: BridgeValue,
) -> anyhow::Result<()> {
    let plugin = payload
        .get_str("plugin")
        .ok_or_else(|| anyhow!("plugin invoke requires plugin"))?;
    let method = payload
        .get_str("method")
        .ok_or_else(|| anyhow!("plugin invoke requires method"))?;
    let args = payload.get("args").cloned().unwrap_or(BridgeValue::Null);
    host.invoke(call_id, plugin, method, args)
}

pub fn register_echo_plugin(host: &PluginHost) {
    host.register_method("Echo", "echo", echo_handler, std::ptr::null_mut());
}

extern "C" fn echo_handler(call: *const RasterPluginCall) {
    let Some(state) = crate::plugin::ffi::plugin_host() else {
        return;
    };
    let Some(call) = (unsafe { call.as_ref() }) else {
        return;
    };
    let args_json = cstr_to_str(call.args_json).unwrap_or("null");
    let args = parse_json_args(args_json);
    let message = args
        .get("msg")
        .or_else(|| args.get("message"))
        .and_then(|value| match value {
            BridgeValue::String(text) => Some(text.as_str()),
            _ => None,
        })
        .unwrap_or("");
    let result = serde_json::json!({ "echo": message }).to_string();
    state.host().reply_ok(call.call_id, &result);
}