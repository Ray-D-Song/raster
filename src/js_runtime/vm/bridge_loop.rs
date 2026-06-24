use std::sync::mpsc;

use crate::common::channel::WakeSignal;
use crate::common::utils::logger;

pub struct JsBridgeWake {
    pub sender: mpsc::Sender<()>,
}

impl JsBridgeWake {
    pub fn new(sender: mpsc::Sender<()>) -> Self {
        Self { sender }
    }
}

impl WakeSignal for JsBridgeWake {
    fn wake(&self) {
        let _ = self.sender.send(());
    }
}

pub async fn drain_bridge_egress(
    runtime: &crate::js_runtime::vm::JsRuntime,
    bridge: &crate::bridge::SharedBridgeState,
) {
    while let Some(envelope) = bridge.try_recv_egress() {
        if let Err(error) = runtime.dispatch_bridge_envelope(envelope).await {
            logger::error(format!("failed to dispatch bridge egress: {error}"));
        }
    }
}