use gpui::Context;

use crate::bridge::{
    BridgeEnvelope, BridgeValue,
    host::handle_assets_call,
    value::bridge_value_to_node,
};
use crate::plugin::{ffi::plugin_host, handle_plugin_invoke};
use crate::common::{
    channel::{NotificationCommandPayload, NotificationType, UiCommand},
    ids::NativeObjectId,
    mount::NodeValue,
};
use crate::gpui_backend::app::RasterRootView;

pub(in crate::gpui_backend) fn drain_bridge_ingress(
    root: &mut RasterRootView,
    cx: &mut Context<RasterRootView>,
) {
    let bridge = root.bridge();
    let envelopes = bridge.drain_ingress();
    let mut asset_loaded = false;
    for envelope in envelopes {
        if let BridgeEnvelope::Call {
            id,
            channel,
            method,
            payload,
        } = envelope
        {
            let channel_name = channel.as_str();
            let method_name = method.as_str();
            let deferred = match channel_name {
                "host.assets" => {
                    let reply = handle_assets_call(&bridge, method_name, payload);
                    if method_name == "load" && reply.is_ok() {
                        asset_loaded = true;
                    }
                    send_bridge_reply(&bridge, id, reply);
                    false
                }
                "host.ui" => {
                    let reply = handle_ui_call(root, cx, method_name, payload);
                    send_bridge_reply(&bridge, id, reply);
                    false
                }
                "host.plugin" => {
                    if method_name != "invoke" {
                        send_bridge_reply(
                            &bridge,
                            id,
                            Err(anyhow::anyhow!("unknown host.plugin method: {method_name}")),
                        );
                        false
                    } else if let Some(host_state) = plugin_host() {
                        match handle_plugin_invoke(host_state.host(), id, payload) {
                            Ok(()) => true,
                            Err(error) => {
                                send_bridge_reply(&bridge, id, Err(error));
                                false
                            }
                        }
                    } else {
                        send_bridge_reply(
                            &bridge,
                            id,
                            Err(anyhow::anyhow!("plugin host not initialized")),
                        );
                        false
                    }
                }
                _ => {
                    send_bridge_reply(
                        &bridge,
                        id,
                        Err(anyhow::anyhow!("unknown bridge channel: {channel}")),
                    );
                    false
                }
            };
            let _ = deferred;
        }
    }
    if asset_loaded {
        root.notify_all_owners(cx);
    }
}

fn send_bridge_reply(
    bridge: &crate::bridge::SharedBridgeState,
    id: u64,
    reply: anyhow::Result<BridgeValue>,
) {
    if id == 0 {
        return;
    }
    let envelope = match reply {
        Ok(value) => BridgeEnvelope::reply_ok(id, value),
        Err(error) => BridgeEnvelope::reply_err(id, error.to_string()),
    };
    let _ = bridge.send_egress(envelope);
}

fn handle_ui_call(
    root: &mut RasterRootView,
    cx: &mut Context<RasterRootView>,
    method: &str,
    payload: BridgeValue,
) -> anyhow::Result<BridgeValue> {
    match method {
        "notificationShow" => {
            root.apply_ui_command(UiCommand::ShowNotification(notification_payload_from_bridge(payload)), cx);
            Ok(BridgeValue::Null)
        }
        "notificationDismiss" => {
            let id = payload
                .get_str("id")
                .ok_or_else(|| anyhow::anyhow!("notificationDismiss requires id"))?;
            root.apply_ui_command(UiCommand::DismissNotification { id: id.to_owned() }, cx);
            Ok(BridgeValue::Null)
        }
        "notificationClear" => {
            root.apply_ui_command(UiCommand::ClearNotifications, cx);
            Ok(BridgeValue::Null)
        }
        "chartAppendData" => {
            let node_id = native_object_id_from_bridge(&payload)?;
            let rows = node_rows_from_bridge(&payload)?;
            root.dispatch_chart_command(node_id, cx, move |state| state.append_data(rows));
            Ok(BridgeValue::Null)
        }
        "chartReplaceData" => {
            let node_id = native_object_id_from_bridge(&payload)?;
            let rows = node_rows_from_bridge(&payload)?;
            root.dispatch_chart_command(node_id, cx, move |state| state.replace_data(rows));
            Ok(BridgeValue::Null)
        }
        "chartClearData" => {
            let node_id = native_object_id_from_bridge(&payload)?;
            root.dispatch_chart_command(node_id, cx, |state| state.clear_data());
            Ok(BridgeValue::Null)
        }
        other => anyhow::bail!("unknown host.ui method: {other}"),
    }
}

fn native_object_id_from_bridge(payload: &BridgeValue) -> anyhow::Result<NativeObjectId> {
    payload
        .get_f64("nodeId")
        .map(|value| NativeObjectId(value as u64))
        .ok_or_else(|| anyhow::anyhow!("bridge ui payload requires nodeId"))
}

fn node_rows_from_bridge(payload: &BridgeValue) -> anyhow::Result<Vec<NodeValue>> {
    match payload {
        BridgeValue::Object(map) => match map.get("rows") {
            Some(BridgeValue::Array(items)) => Ok(items
                .iter()
                .cloned()
                .map(bridge_value_to_node)
                .collect()),
            _ => Ok(Vec::new()),
        },
        _ => Ok(Vec::new()),
    }
}

fn notification_payload_from_bridge(payload: BridgeValue) -> NotificationCommandPayload {
    NotificationCommandPayload {
        id: payload.get_str("id").map(str::to_owned),
        type_: notification_type_from_str(payload.get_str("type").unwrap_or("info")),
        title: payload.get_str("title").map(str::to_owned),
        message: payload.get_str("message").unwrap_or_default().to_owned(),
        autohide: matches!(payload, BridgeValue::Object(ref map) if map.get("autohide").and_then(|value| match value {
            BridgeValue::Bool(value) => Some(*value),
            _ => None,
        }).unwrap_or(true)),
    }
}

fn notification_type_from_str(value: &str) -> NotificationType {
    match value {
        "success" => NotificationType::Success,
        "warning" => NotificationType::Warning,
        "error" => NotificationType::Error,
        _ => NotificationType::Info,
    }
}