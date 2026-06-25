use base64::{Engine as _, engine::general_purpose::STANDARD};
use gpui::{AnyElement, IntoElement, Radians, SharedString, Styled, px};
use gpui_component::{Icon, Sizable, Size};

use crate::{
    common::mount::RetainedNodeKind,
    gpui_backend::{
        components::helper::props::{bool_prop, component_props, number_prop, string_prop},
        render_model::{
            model::RenderModel,
            style::{apply_style, parse_color},
        },
        retained_tree::node::RetainedNode,
    },
};

pub(in crate::gpui_backend) fn icon_from_svg(svg: &str) -> Icon {
    Icon::empty().path(svg_to_data_url(svg))
}

fn svg_to_data_url(svg: &str) -> SharedString {
    let encoded = STANDARD.encode(svg.as_bytes());
    format!("data:image/svg+xml;base64,{encoded}").into()
}

pub(in crate::gpui_backend) fn render_icon_from_node(node: &RetainedNode) -> Option<AnyElement> {
    if !is_icon_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let props = component_props(node);
    let mut icon = if let Some(svg) = string_prop(props, "svg").or_else(|| string_prop(props, "src")) {
        icon_from_svg(&svg)
    } else if bool_prop(props, "empty") == Some(true) {
        Icon::empty()
    } else {
        Icon::empty()
    };

    if let Some(rotation) = number_prop(props, "rotate") {
        icon = icon.rotate(Radians(rotation as f32));
    }
    if let Some(color) = string_prop(props, "color")
        .as_deref()
        .and_then(parse_color)
        .or(model.style.color)
    {
        icon = icon.text_color(color);
    }

    let mut icon = apply_style(icon, &model.style);
    icon = apply_icon_dimensions(icon, props);

    Some(icon.into_any_element())
}

fn apply_icon_dimensions(icon: Icon, props: &std::collections::BTreeMap<String, crate::common::mount::NodeValue>) -> Icon {
    let width = number_prop(props, "width").or_else(|| number_prop(props, "size"));
    let height = number_prop(props, "height").or_else(|| number_prop(props, "size"));

    match (width, height) {
        (Some(w), Some(h)) if dimensions_equal(w, h) => icon.with_size(Size::Size(px(w as f32))),
        (Some(w), Some(h)) => icon.w(px(w as f32)).h(px(h as f32)),
        (Some(w), None) => icon.with_size(Size::Size(px(w as f32))),
        (None, Some(h)) => icon.with_size(Size::Size(px(h as f32))),
        _ => icon,
    }
}

fn dimensions_equal(a: f64, b: f64) -> bool {
    (a - b).abs() < f64::EPSILON
}

pub(in crate::gpui_backend) fn is_icon_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Icon"
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::common::{
        ids::{NativeObjectId, SurfaceId},
        mount::{NodePayload, NodeValue, RetainedNodeKind},
    };
    use crate::gpui_backend::retained_tree::node::RetainedNode;

    use super::*;

    fn icon_node(props: BTreeMap<String, NodeValue>) -> RetainedNode {
        RetainedNode::new(
            NativeObjectId(1),
            SurfaceId(1),
            RetainedNodeKind::Widget,
            "Widget",
            None,
            NodePayload {
                props: [
                    ("name".to_owned(), NodeValue::String("Icon".to_owned())),
                    ("props".to_owned(), NodeValue::Object(props)),
                ]
                .into(),
                ..NodePayload::default()
            },
        )
    }

    #[test]
    fn render_icon_from_inline_svg() {
        let mut props = BTreeMap::new();
        props.insert(
            "svg".to_owned(),
            NodeValue::String("<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".to_owned()),
        );
        props.insert("size".to_owned(), NodeValue::Number(20.0));

        let rendered = render_icon_from_node(&icon_node(props));
        assert!(rendered.is_some());
    }

    #[test]
    fn svg_to_data_url_uses_base64_payload() {
        let svg = "<svg></svg>";
        let data_url = svg_to_data_url(svg);
        assert!(data_url.starts_with("data:image/svg+xml;base64,"));
    }
}