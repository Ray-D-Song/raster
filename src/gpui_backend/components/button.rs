use std::rc::Rc;

use gpui::{AnyElement, App, IntoElement, px};
use gpui_component::{
    Disableable, Selectable, Sizable, Size,
    button::{Button, ButtonRounded, ButtonVariant, ButtonVariants},
};

use crate::{
    common::{
        channel::RuntimeCommand,
        mount::{NodeValue, RetainedNodeKind},
    },
    gpui_backend::{
        components::{
            helper::props::{
                bool_prop, component_props, event_handler, number_prop, prop_or_child_text,
                string_prop,
            },
            icon::icon_from_svg,
        },
        render_model::{model::RenderModel, style::apply_style},
        retained_tree::node::RetainedNode,
    },
};

pub fn render_button_from_node<'a>(
    node: &RetainedNode,
    child_text: impl IntoIterator<Item = &'a str>,
    dispatch_event: Rc<dyn Fn(RuntimeCommand, &mut App)>,
) -> Option<AnyElement> {
    if node.kind != RetainedNodeKind::Widget || node.component_name() != "Button" {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let mut button = Button::new(("raster-button", node.id.0)).secondary();
    let props = component_props(node);

    if let Some(label) = prop_or_child_text(props, "label", child_text) {
        button = button.label(label);
    }
    if let Some(size) = string_prop(props, "size").map(|value| Size::from_str(&value)) {
        button = button.with_size(size);
    }
    if let Some(variant) = string_prop(props, "variant").and_then(parse_button_variant) {
        button = button.with_variant(variant);
    }
    if let Some(disabled) = bool_prop(props, "disabled") {
        button = button.disabled(disabled);
    }
    if let Some(selected) = bool_prop(props, "selected") {
        button = button.selected(selected);
    }
    if let Some(loading) = bool_prop(props, "loading") {
        button = button.loading(loading);
    }
    if bool_prop(props, "compact") == Some(true) {
        button = button.compact();
    }
    if bool_prop(props, "outline") == Some(true) {
        button = button.outline();
    }
    if let Some(rounded) = props.get("rounded").and_then(parse_rounded) {
        button = button.rounded(rounded);
    }
    if let Some(dropdown_caret) = bool_prop(props, "dropdownCaret") {
        button = button.dropdown_caret(dropdown_caret);
    }
    if let Some(tab_index) = number_prop(props, "tabIndex") {
        button = button.tab_index(tab_index as isize);
    }
    if let Some(tab_stop) = bool_prop(props, "tabStop") {
        button = button.tab_stop(tab_stop);
    }
    if let Some(tooltip) = string_prop(props, "tooltip") {
        button = button.tooltip(tooltip);
    }
    if let Some(icon) = string_prop(props, "iconSvg").map(|svg| icon_from_svg(&svg)) {
        button = button.icon(icon);
    }
    if let Some(handler_id) = event_handler(node, "onClick") {
        button = button.on_click(move |_event, _window, _cx| {
            dispatch_event(
                RuntimeCommand::InvokeEvent {
                    handler_id,
                    payload: NodeValue::String(String::new()),
                },
                _cx,
            );
        });
    }

    Some(apply_style(button, &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn parse_button_variant(value: String) -> Option<ButtonVariant> {
    match value.as_str() {
        "primary" => Some(ButtonVariant::Primary),
        "secondary" => Some(ButtonVariant::Secondary),
        "danger" | "error" => Some(ButtonVariant::Danger),
        "info" => Some(ButtonVariant::Info),
        "success" => Some(ButtonVariant::Success),
        "warning" => Some(ButtonVariant::Warning),
        "ghost" => Some(ButtonVariant::Ghost),
        "link" => Some(ButtonVariant::Link),
        "text" => Some(ButtonVariant::Text),
        _ => None,
    }
}

fn parse_rounded(value: &NodeValue) -> Option<ButtonRounded> {
    match value {
        NodeValue::Bool(true) => Some(ButtonRounded::Medium),
        NodeValue::Bool(false) => Some(ButtonRounded::None),
        NodeValue::Number(value) => Some(ButtonRounded::Size(px(*value as f32))),
        NodeValue::String(value) => match value.as_str() {
            "none" => Some(ButtonRounded::None),
            "small" | "sm" => Some(ButtonRounded::Small),
            "medium" | "md" => Some(ButtonRounded::Medium),
            "large" | "lg" => Some(ButtonRounded::Large),
            _ => value
                .parse::<f32>()
                .ok()
                .map(|value| ButtonRounded::Size(px(value))),
        },
        _ => None,
    }
}
