//! Shared mount mutations sent from the JS runtime thread to the GPUI app thread.

#![allow(dead_code)]

use std::collections::BTreeMap;

use crate::common::ids::{HandlerId, NativeObjectId, SurfaceId};

/// Retained node category shared by mutation producers and consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetainedNodeKind {
    View,
    Text,
    Input,
    Textarea,
    Widget,
    Fragment,
}

impl RetainedNodeKind {
    pub fn is_widget(&self) -> bool {
        matches!(self, RetainedNodeKind::Widget)
    }
}

/// JSON-like value stored after crossing the JS/native boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<NodeValue>),
    Object(BTreeMap<String, NodeValue>),
}

impl NodeValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            NodeValue::String(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn to_json_value(&self) -> serde_json::Value {
        match self {
            NodeValue::Null => serde_json::Value::Null,
            NodeValue::Bool(value) => serde_json::Value::Bool(*value),
            NodeValue::Number(value) => serde_json::Number::from_f64(*value)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            NodeValue::String(value) => serde_json::Value::String(value.clone()),
            NodeValue::Array(items) => {
                serde_json::Value::Array(items.iter().map(NodeValue::to_json_value).collect())
            }
            NodeValue::Object(entries) => serde_json::Value::Object(
                entries
                    .iter()
                    .map(|(key, value)| (key.clone(), value.to_json_value()))
                    .collect(),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerBindingKind {
    Event,
    Query,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerBinding {
    pub property: String,
    pub event_or_query_type: Option<String>,
    pub handler_id: HandlerId,
}

/// Platform-neutral payload attached to a retained node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NodePayload {
    pub props: BTreeMap<String, NodeValue>,
    pub style: BTreeMap<String, NodeValue>,
    pub text: Option<String>,
    pub event_bindings: Vec<HandlerBinding>,
    pub query_bindings: Vec<HandlerBinding>,
}

/// One React commit worth of retained tree mutations.
#[derive(Debug, Clone, PartialEq)]
pub struct MountMutationBatch {
    pub surface_id: SurfaceId,
    pub sequence: u64,
    pub mutations: Vec<MountMutation>,
}

/// DOM-like operations applied by the GPUI app thread to its retained tree.
#[derive(Debug, Clone, PartialEq)]
pub enum MountMutation {
    CreateNode {
        id: NativeObjectId,
        kind: RetainedNodeKind,
        name: String,
        key: Option<String>,
        payload: NodePayload,
    },
    CreateText {
        id: NativeObjectId,
        text: String,
        payload: NodePayload,
    },
    UpdateNode {
        id: NativeObjectId,
        payload: NodePayload,
    },
    UpdateText {
        id: NativeObjectId,
        text: String,
    },
    AppendChild {
        parent: NativeObjectId,
        child: NativeObjectId,
    },
    InsertBefore {
        parent: NativeObjectId,
        child: NativeObjectId,
        before: NativeObjectId,
    },
    RemoveChild {
        parent: NativeObjectId,
        child: NativeObjectId,
    },
    DeleteNode {
        id: NativeObjectId,
    },
    SetRootChildren {
        surface_id: SurfaceId,
        children: Vec<NativeObjectId>,
    },
}
