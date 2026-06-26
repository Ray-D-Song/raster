use std::collections::BTreeMap;

use base64::Engine as _;
use serde_json::Value as JsonValue;

use crate::bridge::value::BridgeValue;

pub fn bridge_value_from_json(value: JsonValue) -> BridgeValue {
    match value {
        JsonValue::Null => BridgeValue::Null,
        JsonValue::Bool(value) => BridgeValue::Bool(value),
        JsonValue::Number(number) => BridgeValue::Number(number.as_f64().unwrap_or(0.0)),
        JsonValue::String(text) => BridgeValue::String(text),
        JsonValue::Array(items) => {
            BridgeValue::Array(items.into_iter().map(bridge_value_from_json).collect())
        }
        JsonValue::Object(entries) => BridgeValue::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key, bridge_value_from_json(value)))
                .collect(),
        ),
    }
}

pub fn bridge_value_to_json(value: &BridgeValue) -> JsonValue {
    match value {
        BridgeValue::Null => JsonValue::Null,
        BridgeValue::Bool(value) => JsonValue::Bool(*value),
        BridgeValue::Number(value) => serde_json::Number::from_f64(*value)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        BridgeValue::String(value) => JsonValue::String(value.clone()),
        BridgeValue::Array(items) => {
            JsonValue::Array(items.iter().map(bridge_value_to_json).collect())
        }
        BridgeValue::Object(entries) => JsonValue::Object(
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

pub fn bridge_value_to_json_string(value: &BridgeValue) -> String {
    serde_json::to_string(&bridge_value_to_json(value)).unwrap_or_else(|_| "null".to_owned())
}

pub fn parse_json_args(args_json: &str) -> BTreeMap<String, BridgeValue> {
    match serde_json::from_str::<JsonValue>(args_json) {
        Ok(JsonValue::Object(entries)) => entries
            .into_iter()
            .map(|(key, value)| (key, bridge_value_from_json(value)))
            .collect(),
        Ok(_) => BTreeMap::new(),
        Err(_) => BTreeMap::new(),
    }
}