use std::collections::BTreeMap;

use crate::common::mount::NodeValue;

/// JSON-like value with binary support for cross-thread bridge messages.
#[derive(Debug, Clone, PartialEq)]
pub enum BridgeValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<BridgeValue>),
    Object(BTreeMap<String, BridgeValue>),
    Bytes(Vec<u8>),
}

impl BridgeValue {
    pub fn null() -> Self {
        Self::Null
    }

    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    pub fn number(value: f64) -> Self {
        Self::Number(value)
    }

    pub fn object(entries: impl IntoIterator<Item = (impl Into<String>, BridgeValue)>) -> Self {
        Self::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        match self {
            Self::Object(map) => map.get(key).and_then(|value| match value {
                Self::String(text) => Some(text.as_str()),
                _ => None,
            }),
            _ => None,
        }
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        match self {
            Self::Object(map) => map.get(key).and_then(|value| match value {
                Self::Number(number) => Some(*number),
                _ => None,
            }),
            _ => None,
        }
    }

    pub fn get_bytes(&self, key: &str) -> Option<&[u8]> {
        match self {
            Self::Object(map) => map.get(key).and_then(|value| match value {
                Self::Bytes(bytes) => Some(bytes.as_slice()),
                _ => None,
            }),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&BridgeValue> {
        match self {
            Self::Object(map) => map.get(key),
            _ => None,
        }
    }
}

impl From<NodeValue> for BridgeValue {
    fn from(value: NodeValue) -> Self {
        match value {
            NodeValue::Null => Self::Null,
            NodeValue::Bool(value) => Self::Bool(value),
            NodeValue::Number(value) => Self::Number(value),
            NodeValue::String(value) => Self::String(value),
            NodeValue::Array(items) => Self::Array(items.into_iter().map(Self::from).collect()),
            NodeValue::Object(entries) => Self::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, Self::from(value)))
                    .collect(),
            ),
        }
    }
}

impl From<BridgeValue> for NodeValue {
    fn from(value: BridgeValue) -> Self {
        match value {
            BridgeValue::Null => Self::Null,
            BridgeValue::Bool(value) => Self::Bool(value),
            BridgeValue::Number(value) => Self::Number(value),
            BridgeValue::String(value) => Self::String(value),
            BridgeValue::Array(items) => Self::Array(items.into_iter().map(Self::from).collect()),
            BridgeValue::Object(entries) => Self::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, Self::from(value)))
                    .collect(),
            ),
            BridgeValue::Bytes(bytes) => Self::String(base64_encode(&bytes)),
        }
    }
}

pub fn node_value_to_bridge(value: NodeValue) -> BridgeValue {
    BridgeValue::from(value)
}

pub fn bridge_value_to_node(value: BridgeValue) -> NodeValue {
    NodeValue::from(value)
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}