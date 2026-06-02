use gpui::{AnyElement, IntoElement};
use gpui_component::label::{HighlightsMatch, Label};

use crate::{
    common::mount::RetainedNodeKind,
    gpui_backend::{
        components::helper::props::{bool_prop, component_props, string_prop},
        retained_tree::node::RetainedNode,
    },
};

/// Lightweight text representation for JSX text nodes and non-selectable labels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelRenderModel {
    pub text: String,
    pub secondary: Option<String>,
    pub masked: bool,
    pub highlights: Option<String>,
}

impl LabelRenderModel {
    pub fn into_label(self) -> Label {
        let mut label = Label::new(self.text);
        if let Some(secondary) = self.secondary {
            label = label.secondary(secondary);
        }
        if self.masked {
            label = label.masked(true);
        }
        if let Some(highlights) = self.highlights {
            label = label.highlights(HighlightsMatch::Full(highlights.into()));
        }
        label
    }

    pub fn into_any_element(self) -> AnyElement {
        self.into_label().into_any_element()
    }
}

pub fn label_model_from_node<'a>(
    node: &RetainedNode,
    child_text: impl IntoIterator<Item = &'a str>,
) -> Option<LabelRenderModel> {
    match node.kind {
        RetainedNodeKind::Text => Some(LabelRenderModel {
            text: node.payload.text.clone().unwrap_or_default(),
            secondary: None,
            masked: false,
            highlights: None,
        }),
        RetainedNodeKind::Widget if node.component_name() == "Label" => Some(LabelRenderModel {
            text: label_text(node, child_text),
            secondary: string_prop(component_props(node), "secondary"),
            masked: bool_prop(component_props(node), "masked").unwrap_or(false),
            highlights: string_prop(component_props(node), "highlights"),
        }),
        _ => None,
    }
}

pub fn render_label_from_node<'a>(
    node: &RetainedNode,
    child_text: impl IntoIterator<Item = &'a str>,
) -> Option<Label> {
    label_model_from_node(node, child_text).map(LabelRenderModel::into_label)
}

pub fn render_label_element_from_node<'a>(
    node: &RetainedNode,
    child_text: impl IntoIterator<Item = &'a str>,
) -> Option<AnyElement> {
    label_model_from_node(node, child_text).map(LabelRenderModel::into_any_element)
}

pub fn label_text<'a>(
    node: &RetainedNode,
    child_text: impl IntoIterator<Item = &'a str>,
) -> String {
    let props = component_props(node);
    string_prop(props, "label")
        .or_else(|| string_prop(props, "text"))
        .or_else(|| string_prop(props, "content"))
        .unwrap_or_else(|| child_text.into_iter().collect::<String>())
}
