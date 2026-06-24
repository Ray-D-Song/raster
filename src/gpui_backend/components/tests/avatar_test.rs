use std::collections::BTreeMap;

use crate::{
    common::{
        ids::{NativeObjectId, SurfaceId},
        mount::{NodePayload, NodeValue, RetainedNodeKind},
    },
    gpui_backend::{
        components::avatar::{is_avatar_node, render_avatar_from_node},
        retained_tree::node::RetainedNode,
    },
};

#[test]
fn avatar_renders_when_src_prop_is_nested_under_widget_props() {
    let node = avatar_node([(
        "src",
        NodeValue::String("https://example.com/avatar.png".to_owned()),
    )]);
    assert!(is_avatar_node(&node));
    assert!(
        render_avatar_from_node(&node).is_some(),
        "Avatar with src should produce a renderable element"
    );
}

fn avatar_node(props: impl IntoIterator<Item = (&'static str, NodeValue)>) -> RetainedNode {
    let component_props = props
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect::<BTreeMap<_, _>>();
    RetainedNode::new(
        NativeObjectId(1),
        SurfaceId(1),
        RetainedNodeKind::Widget,
        "Widget",
        None,
        NodePayload {
            props: [
                ("name".to_owned(), NodeValue::String("Avatar".to_owned())),
                ("props".to_owned(), NodeValue::Object(component_props)),
            ]
            .into(),
            ..NodePayload::default()
        },
    )
}