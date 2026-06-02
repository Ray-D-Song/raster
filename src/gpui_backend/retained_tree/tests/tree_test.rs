use crate::common::{
    ids::HandlerId,
    mount::{HandlerBinding, NodePayload, NodeValue, RetainedNodeKind},
};
use crate::gpui_backend::{render_model::model::RenderModel, retained_tree::tree::RetainedTree};

#[test]
fn create_view_node_stores_view_render_model() {
    let mut tree = RetainedTree::new();
    let surface = tree.create_surface();

    let view = tree
        .create_node(
            surface,
            RetainedNodeKind::View,
            "View",
            None,
            NodePayload::default(),
        )
        .unwrap();

    let node = tree.node(view).expect("view node should be created");
    assert_eq!(node.id, view);
    assert_eq!(node.surface_id, surface);
    assert_eq!(node.kind, RetainedNodeKind::View);
    assert!(matches!(node.render_model, RenderModel::View(_)));

    let text = tree.create_text(surface, "Hello").unwrap();
    tree.append_child(view, text).unwrap();

    let view_node = tree.node(view).expect("view node should still exist");
    assert_eq!(view_node.children, vec![text]);

    let text_node = tree.node(text).expect("text node should be created");
    assert_eq!(text_node.parent, Some(view));
    assert_eq!(text_node.payload.text.as_deref(), Some("Hello"));
    assert!(matches!(text_node.render_model, RenderModel::Label(_)));

    let same_text = tree.update_text(text, "Hello").unwrap();
    assert!(same_text.is_clean());

    let next_text = tree.update_text(text, "Raster").unwrap();
    assert!(!next_text.is_clean());
}

#[test]
fn update_node_skips_noop_and_handler_only_dirty() {
    let mut tree = RetainedTree::new();
    let surface = tree.create_surface();

    let mut payload = NodePayload::default();
    payload
        .props
        .insert("label".to_owned(), NodeValue::String("Click".to_owned()));
    let button = tree
        .create_node(
            surface,
            RetainedNodeKind::Widget,
            "Button",
            None,
            payload.clone(),
        )
        .unwrap();

    let noop = tree.update_node(button, payload.clone()).unwrap();
    assert!(noop.is_clean());

    let mut handler_payload = payload.clone();
    handler_payload.event_bindings.push(HandlerBinding {
        property: "onClick".to_owned(),
        event_or_query_type: Some("click".to_owned()),
        handler_id: HandlerId(1),
    });
    let handler_only = tree.update_node(button, handler_payload).unwrap();
    assert!(handler_only.is_clean());

    let mut visual_payload = payload;
    visual_payload
        .props
        .insert("label".to_owned(), NodeValue::String("Next".to_owned()));
    let visual = tree.update_node(button, visual_payload).unwrap();
    assert!(!visual.is_clean());
}
