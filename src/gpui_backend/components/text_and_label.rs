use gpui::{AnyElement, Context, IntoElement, Window};
use gpui_component::label::Label;

use crate::gpui_backend::{
    app::NodeOwnerView,
    components::internal::{rich_text::render_rich_text_from_node, text::render_label_from_node},
    render_model::{model::RenderModel, style::apply_text_style},
    retained_tree::node::RetainedNode,
};

pub fn render_text_label_from_node<'a>(
    node: &RetainedNode,
    child_text: impl IntoIterator<Item = &'a str>,
) -> Option<Label> {
    render_label_from_node(node, child_text)
}

pub fn render_text_label_element_from_node<'a>(
    node: &RetainedNode,
    child_text: impl IntoIterator<Item = &'a str>,
) -> Option<AnyElement> {
    let label = render_label_from_node(node, child_text)?;
    match &node.render_model {
        RenderModel::Label(model) => Some(apply_text_style(label, &model.style).into_any_element()),
        RenderModel::Widget(model) => Some(apply_text_style(label, &model.style).into_any_element()),
        _ => Some(label.into_any_element()),
    }
}

pub(in crate::gpui_backend) fn render_text_label_or_rich_text_from_node<'a>(
    node: &RetainedNode,
    child_text: impl IntoIterator<Item = &'a str>,
    window: &mut Window,
    cx: &mut Context<NodeOwnerView>,
) -> Option<AnyElement> {
    let child_text = child_text.into_iter().collect::<Vec<_>>();
    render_rich_text_from_node(node, child_text.iter().copied(), window, cx)
        .or_else(|| render_text_label_element_from_node(node, child_text.iter().copied()))
}
