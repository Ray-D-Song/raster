use gpui::{AnyElement, Context, IntoElement, Window};
use gpui_component::text::TextView;

use crate::{
    common::mount::RetainedNodeKind,
    gpui_backend::{
        app::NodeOwnerView,
        components::{
            helper::props::{bool_prop, component_props, string_prop},
            internal::text::label_text,
        },
        render_model::{model::RenderModel, style::apply_style},
        retained_tree::node::RetainedNode,
    },
};

pub fn should_render_rich_text(node: &RetainedNode) -> bool {
    matches!(node.kind, RetainedNodeKind::Widget)
        && (node.component_name() == "TextView"
            || (node.component_name() == "Label"
                && bool_prop(component_props(node), "selectable") == Some(true)))
}

pub(in crate::gpui_backend) fn render_rich_text_from_node<'a>(
    node: &RetainedNode,
    child_text: impl IntoIterator<Item = &'a str>,
    _window: &mut Window,
    _cx: &mut Context<NodeOwnerView>,
) -> Option<AnyElement> {
    if !should_render_rich_text(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let content = if node.component_name() == "TextView" {
        string_prop(component_props(node), "content")
            .unwrap_or_else(|| child_text.into_iter().collect::<String>())
    } else {
        label_text(node, child_text)
    };

    let text_type =
        string_prop(component_props(node), "type").unwrap_or_else(|| "markdown".to_owned());
    let mut text_view = match text_type.as_str() {
        "html" => TextView::html(("raster-rich-text", node.id.0), content),
        _ => TextView::markdown(("raster-rich-text", node.id.0), content),
    };

    let selectable = bool_prop(component_props(node), "selectable").unwrap_or(true);
    text_view = text_view.selectable(selectable);

    if let Some(scrollable) = bool_prop(component_props(node), "scrollable") {
        text_view = text_view.scrollable(scrollable);
    }

    Some(apply_style(text_view, &model.style).into_any_element())
}
