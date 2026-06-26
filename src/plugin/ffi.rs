use std::os::raw::{c_char, c_void};
use std::sync::{Arc, OnceLock, Weak};

use crate::bridge::state::SharedBridgeState;
use crate::plugin::host::{PluginHost, RasterPluginCall, cstr_to_str, register_echo_plugin};

static PLUGIN_HOST: OnceLock<Weak<PluginHostState>> = OnceLock::new();

pub struct PluginHostState {
    host: PluginHost,
}

impl PluginHostState {
    pub fn new(bridge: SharedBridgeState) -> Self {
        let host = PluginHost::new(bridge);
        register_echo_plugin(&host);
        Self { host }
    }

    pub fn host(&self) -> &PluginHost {
        &self.host
    }
}

pub fn install_plugin_host(bridge: SharedBridgeState) -> Arc<PluginHostState> {
    let state = Arc::new(PluginHostState::new(bridge));
    let _ = PLUGIN_HOST.set(Arc::downgrade(&state));
    state
}

pub(crate) fn plugin_host() -> Option<Arc<PluginHostState>> {
    PLUGIN_HOST.get()?.upgrade()
}

#[unsafe(no_mangle)]
pub extern "C" fn raster_plugin_register_method(
    plugin: *const c_char,
    method: *const c_char,
    handler: extern "C" fn(*const RasterPluginCall),
    context: *mut c_void,
) -> bool {
    let Some(state) = plugin_host() else {
        return false;
    };
    let Some(plugin) = cstr_to_str(plugin) else {
        return false;
    };
    let Some(method) = cstr_to_str(method) else {
        return false;
    };
    state.host().register_method(plugin, method, handler, context)
}

#[unsafe(no_mangle)]
pub extern "C" fn raster_plugin_reply_ok(call_id: u64, result_json: *const c_char) {
    let Some(state) = plugin_host() else {
        return;
    };
    let Some(result_json) = cstr_to_str(result_json) else {
        state
            .host()
            .reply_err(call_id, "INVALID_RESULT", "result_json is null");
        return;
    };
    state.host().reply_ok(call_id, result_json);
}

#[unsafe(no_mangle)]
pub extern "C" fn raster_plugin_reply_err(
    call_id: u64,
    code: *const c_char,
    message: *const c_char,
) {
    let Some(state) = plugin_host() else {
        return;
    };
    let code = cstr_to_str(code).unwrap_or("PLUGIN_ERROR");
    let message = cstr_to_str(message).unwrap_or("plugin call failed");
    state.host().reply_err(call_id, code, message);
}

#[unsafe(no_mangle)]
pub extern "C" fn raster_plugin_emit_event(
    plugin: *const c_char,
    event: *const c_char,
    data_json: *const c_char,
) {
    let Some(state) = plugin_host() else {
        return;
    };
    let Some(plugin) = cstr_to_str(plugin) else {
        return;
    };
    let Some(event) = cstr_to_str(event) else {
        return;
    };
    let data_json = cstr_to_str(data_json).unwrap_or("null");
    state.host().emit_event(plugin, event, data_json);
}