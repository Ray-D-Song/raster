use std::collections::BTreeMap;

use gpui_component::IndexPath;

use crate::{
    common::{
        ids::{NativeObjectId, SurfaceId},
        mount::{NodePayload, NodeValue, RetainedNodeKind},
    },
    gpui_backend::{components::select::RasterSelectModel, retained_tree::node::RetainedNode},
};

#[test]
fn select_model_parses_options_and_sections() {
    let options_node = select_node([(
        "options",
        NodeValue::Array(vec![
            option("stable", "Stable", false),
            option("nightly", "Nightly", true),
        ]),
    )]);
    let options = RasterSelectModel::from_node(&options_node);

    assert_eq!(options.sections.len(), 1);
    assert_eq!(options.sections[0].items[0].title, "Stable");
    assert_eq!(
        options.sections[0].items[0].value,
        NodeValue::String("stable".to_owned())
    );
    assert!(!options.sections[0].items[0].disabled);
    assert!(options.sections[0].items[1].disabled);
    assert_eq!(
        options.position(&NodeValue::String("nightly".to_owned())),
        Some(IndexPath::default().section(0).row(1))
    );

    let sections_node = select_node([(
        "sections",
        NodeValue::Array(vec![NodeValue::Object(
            [
                ("label".to_owned(), NodeValue::String("Release".to_owned())),
                (
                    "items".to_owned(),
                    NodeValue::Array(vec![option("stable", "Stable", false)]),
                ),
            ]
            .into(),
        )]),
    )]);
    let sections = RasterSelectModel::from_node(&sections_node);

    assert_eq!(sections.sections.len(), 1);
    assert_eq!(sections.sections[0].label.as_deref(), Some("Release"));
    assert_eq!(
        sections
            .item(IndexPath::default().section(0).row(0))
            .map(|item| item.title.as_str()),
        Some("Stable")
    );
}

fn select_node(props: impl IntoIterator<Item = (&'static str, NodeValue)>) -> RetainedNode {
    let component_props = props
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect();
    RetainedNode::new(
        NativeObjectId(1),
        SurfaceId(1),
        RetainedNodeKind::Widget,
        "Widget",
        None,
        NodePayload {
            props: [
                ("name".to_owned(), NodeValue::String("Select".to_owned())),
                ("props".to_owned(), NodeValue::Object(component_props)),
            ]
            .into(),
            ..NodePayload::default()
        },
    )
}

fn option(value: &str, label: &str, disabled: bool) -> NodeValue {
    let mut props = BTreeMap::new();
    props.insert("id".to_owned(), NodeValue::String(value.to_owned()));
    props.insert("value".to_owned(), NodeValue::String(value.to_owned()));
    props.insert("label".to_owned(), NodeValue::String(label.to_owned()));
    props.insert("disabled".to_owned(), NodeValue::Bool(disabled));
    NodeValue::Object(props)
}
