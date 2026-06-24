use std::collections::BTreeMap;
use std::io::Cursor;

use gpui::ImageSource;

use crate::{
    bridge::{BridgeState, new_asset_store},
    common::{
        ids::{NativeObjectId, SurfaceId},
        mount::{NodePayload, NodeValue, RetainedNodeKind},
    },
    gpui_backend::{
        asset_context::with_render_assets,
        components::{
            helper::image_source::resolve_image_source,
            image::{is_image_node, render_image_from_node},
        },
        retained_tree::node::RetainedNode,
    },
};

#[test]
fn image_renders_when_src_prop_is_nested_under_widget_props() {
    let node = image_node([(
        "src",
        NodeValue::String("https://example.com/photo.png".to_owned()),
    )]);
    assert!(is_image_node(&node));
    assert!(
        render_image_from_node(&node, dummy_dispatch()).is_some(),
        "Image with src should produce a renderable element"
    );
}

#[test]
fn image_renders_for_each_object_fit_value() {
    for fit in ["fill", "contain", "cover", "scaleDown", "none"] {
        let node = image_node([
            ("src", NodeValue::String("https://example.com/photo.png".to_owned())),
            ("objectFit", NodeValue::String(fit.to_owned())),
        ]);
        assert!(
            render_image_from_node(&node, dummy_dispatch()).is_some(),
            "Image with objectFit={fit} should render"
        );
    }
}

#[test]
fn resolve_image_source_returns_render_for_cached_remote_uri() {
    let uri = "https://example.com/cached.png";
    let bytes = png_bytes(8, 8);
    let store = new_asset_store();
    {
        let mut locked = store.lock().expect("asset store lock");
        locked.load_image(uri, &bytes).expect("load test image");
    }

    with_render_assets(store, || {
        let source = resolve_image_source(uri).expect("cached remote image should resolve");
        assert!(matches!(source, ImageSource::Render(_)));
    });
}

fn png_bytes(width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba([1, 2, 3, 255]));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("encode png");
    bytes
}

fn image_node(props: impl IntoIterator<Item = (&'static str, NodeValue)>) -> RetainedNode {
    let component_props = props
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect::<BTreeMap<_, _>>();
    RetainedNode::new(
        NativeObjectId(2),
        SurfaceId(1),
        RetainedNodeKind::Widget,
        "Widget",
        None,
        NodePayload {
            props: [
                ("name".to_owned(), NodeValue::String("Image".to_owned())),
                ("props".to_owned(), NodeValue::Object(component_props)),
            ]
            .into(),
            ..NodePayload::default()
        },
    )
}

fn dummy_dispatch() -> crate::bridge::BridgeEventDispatch {
    crate::bridge::bridge_event_dispatcher(BridgeState::new(new_asset_store()))
}