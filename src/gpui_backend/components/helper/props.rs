use std::collections::BTreeMap;

use crate::{
    common::{
        ids::HandlerId,
        mount::{NodeValue, RetainedNodeKind},
    },
    gpui_backend::retained_tree::node::RetainedNode,
};

pub(in crate::gpui_backend) fn component_props(
    node: &RetainedNode,
) -> &BTreeMap<String, NodeValue> {
    match node.payload.props.get("props") {
        Some(NodeValue::Object(props)) => props,
        _ => &node.payload.props,
    }
}

pub(in crate::gpui_backend) fn string_prop(
    props: &BTreeMap<String, NodeValue>,
    name: &str,
) -> Option<String> {
    props.get(name).map(display_value)
}

pub(in crate::gpui_backend) fn bool_prop(
    props: &BTreeMap<String, NodeValue>,
    name: &str,
) -> Option<bool> {
    match props.get(name) {
        Some(NodeValue::Bool(value)) => Some(*value),
        _ => None,
    }
}

pub(in crate::gpui_backend) fn number_prop(
    props: &BTreeMap<String, NodeValue>,
    name: &str,
) -> Option<f64> {
    match props.get(name) {
        Some(NodeValue::Number(value)) => Some(*value),
        Some(NodeValue::String(value)) => value.parse().ok(),
        _ => None,
    }
}

pub(in crate::gpui_backend) fn event_handler(
    node: &RetainedNode,
    property: &str,
) -> Option<HandlerId> {
    node.payload
        .event_bindings
        .iter()
        .find(|binding| binding.property == property)
        .map(|binding| binding.handler_id)
}

pub(in crate::gpui_backend) fn prop_or_child_text<'a>(
    props: &BTreeMap<String, NodeValue>,
    name: &str,
    child_text: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    string_prop(props, name).or_else(|| {
        let text = child_text.into_iter().collect::<String>();
        (!text.is_empty()).then_some(text)
    })
}

pub(in crate::gpui_backend) fn is_widget_component(node: &RetainedNode, name: &str) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == name
}

pub(in crate::gpui_backend) fn display_value(value: &NodeValue) -> String {
    match value {
        NodeValue::Null => String::new(),
        NodeValue::Bool(value) => value.to_string(),
        NodeValue::Number(value) => {
            if value.fract() == 0.0 {
                (*value as i64).to_string()
            } else {
                value.to_string()
            }
        }
        NodeValue::String(value) => value.clone(),
        NodeValue::Array(_) | NodeValue::Object(_) => value.to_json_value().to_string(),
    }
}
