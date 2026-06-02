use crate::common::mount::{NodePayload, RetainedNodeKind};
use crate::gpui_backend::render_model::{
    model::{LabelModel, RenderModel, ViewModel, WidgetModel},
    style::parse_render_style,
};

/// Builds the retained render model for a node payload.
pub fn build_render_model(
    kind: &RetainedNodeKind,
    name: &str,
    payload: &NodePayload,
) -> RenderModel {
    let style = parse_render_style(&payload.style);
    match kind {
        RetainedNodeKind::View => RenderModel::View(ViewModel { style }),
        RetainedNodeKind::Text => RenderModel::Label(LabelModel {
            text: payload.text.clone().unwrap_or_default(),
            style,
        }),
        RetainedNodeKind::Input | RetainedNodeKind::Textarea => RenderModel::Widget(WidgetModel {
            component_name: name.to_owned(),
            style,
        }),
        RetainedNodeKind::Widget => RenderModel::Widget(WidgetModel {
            component_name: widget_component_name(name, payload).to_owned(),
            style,
        }),
        RetainedNodeKind::Fragment => RenderModel::Fragment,
    }
}

fn widget_component_name<'a>(name: &'a str, payload: &'a NodePayload) -> &'a str {
    payload
        .props
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or(name)
}
