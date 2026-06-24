use std::rc::Rc;

use crate::bridge::envelope::BridgeEnvelope;
use crate::bridge::state::SharedBridgeState;
use crate::bridge::value::{BridgeValue, node_value_to_bridge};
use crate::common::{ids::HandlerId, mount::NodeValue};

pub type BridgeEventDispatch = Rc<dyn Fn(HandlerId, NodeValue)>;

pub fn bridge_event_dispatcher(bridge: SharedBridgeState) -> BridgeEventDispatch {
    Rc::new(move |handler_id, payload| {
        emit_handler_invoke(&bridge, handler_id, payload);
    })
}

pub fn emit_handler_invoke(bridge: &BridgeState, handler_id: HandlerId, payload: NodeValue) {
    let _ = bridge.send_egress(BridgeEnvelope::event(
        "host.event",
        "invoke",
        BridgeValue::object([
            ("handlerId", BridgeValue::Number(handler_id.0 as f64)),
            ("payload", node_value_to_bridge(payload)),
        ]),
    ));
}

pub fn emit_runtime_event(bridge: &BridgeState, name: impl Into<String>, payload: NodeValue) {
    let _ = bridge.send_egress(BridgeEnvelope::event(
        "runtime.lifecycle",
        name,
        node_value_to_bridge(payload),
    ));
}

use crate::bridge::state::BridgeState;