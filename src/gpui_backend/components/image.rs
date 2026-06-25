use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use gpui::{
    AnyElement, App, ImgResourceLoader, IntoElement, ObjectFit, ParentElement, Styled,
    StyledImage, Window, div, hsla, img, px,
};

use crate::{
    bridge::BridgeEventDispatch,
    common::{
        ids::HandlerId,
        mount::{NodeValue, RetainedNodeKind},
    },
    gpui_backend::{
        components::helper::{
            image_source::{
                classify_image_src, is_remote_uri, resolve_image_source, resource_from_src,
                ImageSrcKind,
            },
            props::{bool_prop, component_props, event_handler, number_prop, string_prop},
        },
        render_model::{model::RenderModel, style::apply_style},
        retained_tree::node::RetainedNode,
    },
};

static FIRED_IMAGE_EVENTS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

pub(in crate::gpui_backend) fn render_image_from_node(
    node: &RetainedNode,
    dispatch_event: BridgeEventDispatch,
) -> Option<AnyElement> {
    if !is_image_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let props = component_props(node);
    let src = string_prop(props, "src")?;
    let show_loading = bool_prop(props, "showLoading").unwrap_or(true);
    let alt = string_prop(props, "alt").unwrap_or_default();
    let object_fit = parse_object_fit(props);
    let on_load = event_handler(node, "onLoad");
    let on_error = event_handler(node, "onError");
    let node_id = node.id.0;

    let src_kind = classify_image_src(&src);

    if matches!(src_kind, ImageSrcKind::Invalid) {
        fire_image_event_once(node_id, &src, "error", on_error, &dispatch_event);
        return Some(fallback_element(&alt));
    }

    if is_remote_uri(&src) && resolve_image_source(&src).is_none() {
        if show_loading {
            return Some(
                apply_style(loading_placeholder(), &model.style).into_any_element(),
            );
        }
        return Some(apply_style(div(), &model.style).into_any_element());
    }

    let source = if is_remote_uri(&src) {
        resolve_image_source(&src)?
    } else {
        local_tracked_source(src.clone(), node_id, on_load, dispatch_event.clone())
    };

    let src_for_fallback = src.clone();
    let dispatch_for_fallback = dispatch_event.clone();
    let alt_for_fallback = alt.clone();

    let mut image = img(source)
        .object_fit(object_fit)
        .with_loading(|| loading_placeholder().into_any_element())
        .with_fallback(move || {
            fire_image_event_once(
                node_id,
                &src_for_fallback,
                "error",
                on_error,
                &dispatch_for_fallback,
            );
            fallback_element(&alt_for_fallback)
        });

    image = apply_image_dimensions(image, props);
    Some(apply_style(image, &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn is_image_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Image"
}

fn parse_object_fit(props: &std::collections::BTreeMap<String, NodeValue>) -> ObjectFit {
    match string_prop(props, "objectFit").as_deref() {
        Some("fill") => ObjectFit::Fill,
        Some("cover") => ObjectFit::Cover,
        Some("scaleDown") => ObjectFit::ScaleDown,
        Some("none") => ObjectFit::None,
        _ => ObjectFit::Contain,
    }
}

fn apply_image_dimensions(
    mut image: gpui::Img,
    props: &std::collections::BTreeMap<String, NodeValue>,
) -> gpui::Img {
    if let Some(width) = number_prop(props, "width") {
        image = image.w(px(width as f32));
    }
    if let Some(height) = number_prop(props, "height") {
        image = image.h(px(height as f32));
    }
    image
}

fn local_tracked_source(
    src: String,
    node_id: u64,
    on_load: Option<HandlerId>,
    dispatch_event: BridgeEventDispatch,
) -> gpui::ImageSource {
    gpui::ImageSource::from(move |window: &mut Window, cx: &mut App| {
        let Some(resource) = resource_from_src(&src) else {
            return None;
        };
        let result = window.use_asset::<ImgResourceLoader>(&resource, cx);
        if let Some(Ok(_)) = &result {
            fire_image_event_once(node_id, &src, "load", on_load, &dispatch_event);
        }
        result
    })
}

fn fire_image_event_once(
    node_id: u64,
    src: &str,
    event: &str,
    handler: Option<HandlerId>,
    dispatch_event: &BridgeEventDispatch,
) {
    let Some(handler) = handler else {
        return;
    };

    let key = format!("{node_id}:{src}:{event}");
    let fired = FIRED_IMAGE_EVENTS.get_or_init(|| Mutex::new(HashSet::new()));
    let Ok(mut guard) = fired.lock() else {
        return;
    };
    if guard.insert(key) {
        dispatch_event(handler, NodeValue::String(src.to_owned()));
    }
}

fn loading_placeholder() -> gpui::Div {
    div().bg(hsla(0., 0., 0.9, 0.08))
}

fn fallback_element(alt: &str) -> AnyElement {
    if alt.is_empty() {
        div().bg(hsla(0., 0., 0.5, 0.12)).into_any_element()
    } else {
        div().child(alt.to_owned()).into_any_element()
    }
}