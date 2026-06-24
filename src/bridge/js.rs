use std::collections::BTreeMap;

use base64::Engine;
use llrt_core::{Ctx, Exception, Function, Object, Result as JsResult, Value};

use crate::bridge::envelope::BridgeEnvelope;
use crate::bridge::state::SharedBridgeState;
use crate::bridge::value::BridgeValue;

pub fn install_bridge_binding<'js>(ctx: Ctx<'js>, bridge: SharedBridgeState) -> JsResult<()> {
    let object = Object::new(ctx.clone())?;

    {
        let bridge = bridge.clone();
        object.set(
            "call",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, channel: String, method: String, payload: Value<'js>| {
                    let payload = bridge_value_from_js(payload)?;
                    let id = bridge.next_call_id();
                    bridge
                        .send_ingress(BridgeEnvelope::Call {
                            id,
                            channel,
                            method,
                            payload,
                        })
                        .map_err(|error| Exception::throw_message(&ctx, &error.to_string()))?;
                    Ok::<u64, llrt_core::Error>(id)
                },
            )?,
        )?;
    }

    {
        let bridge = bridge.clone();
        object.set(
            "post",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, channel: String, method: String, payload: Value<'js>| {
                    let payload = bridge_value_from_js(payload)?;
                    bridge
                        .send_ingress(BridgeEnvelope::Call {
                            id: 0,
                            channel,
                            method,
                            payload,
                        })
                        .map_err(|error| Exception::throw_message(&ctx, &error.to_string()))?;
                    Ok::<(), llrt_core::Error>(())
                },
            )?,
        )?;
    }

    ctx.globals().set("__rasterBridge", object)
}

pub fn bridge_value_from_js<'js>(value: Value<'js>) -> JsResult<BridgeValue> {
    if let Some(object) = value.as_object() {
        if object.get::<_, bool>("__bridgeBytes").unwrap_or(false) {
            let data = object.get::<_, String>("data")?;
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data.as_bytes())
            .map_err(|error| Exception::throw_message(value.ctx(), &error.to_string()))?;
            return Ok(BridgeValue::Bytes(bytes));
        }
    }

    bridge_value_from_js_value(value)
}

fn bridge_value_from_js_value<'js>(value: Value<'js>) -> JsResult<BridgeValue> {
    match value.type_of() {
        llrt_core::Type::Uninitialized | llrt_core::Type::Undefined | llrt_core::Type::Null => {
            Ok(BridgeValue::Null)
        }
        llrt_core::Type::Bool => Ok(BridgeValue::Bool(value.as_bool().unwrap_or(false))),
        llrt_core::Type::Int | llrt_core::Type::Float => {
            Ok(BridgeValue::Number(value.as_number().unwrap_or(0.0)))
        }
        llrt_core::Type::String => Ok(BridgeValue::String(
            value
                .as_string()
                .and_then(|string| string.to_string().ok())
                .unwrap_or_default(),
        )),
        llrt_core::Type::Array => {
            let array = value
                .as_array()
                .ok_or_else(|| Exception::throw_type(value.ctx(), "expected array"))?;
            let mut items = Vec::new();
            for item in array.iter::<Value>() {
                items.push(bridge_value_from_js(item?)?);
            }
            Ok(BridgeValue::Array(items))
        }
        llrt_core::Type::Object => {
            let object = value
                .as_object()
                .ok_or_else(|| Exception::throw_type(value.ctx(), "expected object"))?;
            let mut entries = BTreeMap::new();
            for key in object.keys::<String>() {
                let key = key?;
                let child = object.get::<_, Value>(&key)?;
                entries.insert(key, bridge_value_from_js(child)?);
            }
            Ok(BridgeValue::Object(entries))
        }
        _ => Ok(BridgeValue::Null),
    }
}

pub fn bridge_envelope_to_json_value(envelope: &BridgeEnvelope) -> serde_json::Value {
    match envelope {
        BridgeEnvelope::Call {
            id,
            channel,
            method,
            payload,
        } => serde_json::json!({
            "kind": "call",
            "id": id,
            "channel": channel,
            "method": method,
            "payload": bridge_value_to_json(payload),
        }),
        BridgeEnvelope::Reply {
            id,
            ok,
            payload,
            error,
        } => serde_json::json!({
            "kind": "reply",
            "id": id,
            "ok": ok,
            "payload": bridge_value_to_json(payload),
            "error": error,
        }),
        BridgeEnvelope::Event {
            channel,
            name,
            payload,
        } => serde_json::json!({
            "kind": "event",
            "channel": channel,
            "name": name,
            "payload": bridge_value_to_json(payload),
        }),
    }
}

fn bridge_value_to_json(value: &BridgeValue) -> serde_json::Value {
    match value {
        BridgeValue::Null => serde_json::Value::Null,
        BridgeValue::Bool(value) => serde_json::Value::Bool(*value),
        BridgeValue::Number(value) => serde_json::Number::from_f64(*value)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        BridgeValue::String(value) => serde_json::Value::String(value.clone()),
        BridgeValue::Array(items) => {
            serde_json::Value::Array(items.iter().map(bridge_value_to_json).collect())
        }
        BridgeValue::Object(entries) => serde_json::Value::Object(
            entries
                .iter()
                .map(|(key, value)| (key.clone(), bridge_value_to_json(value)))
                .collect(),
        ),
        BridgeValue::Bytes(bytes) => serde_json::json!({
            "__bridgeBytes": true,
            "data": base64::engine::general_purpose::STANDARD.encode(bytes),
        }),
    }
}