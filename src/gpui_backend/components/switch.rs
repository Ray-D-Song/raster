use std::rc::Rc;

use gpui::{AnyElement, App, IntoElement};
use gpui_component::{Disableable, Sizable, Size, switch::Switch};

use crate::{
    common::{
        channel::RuntimeCommand,
        mount::{NodeValue, RetainedNodeKind},
    },
    gpui_backend::{
        components::helper::props::{
            bool_prop, component_props, event_handler, prop_or_child_text, string_prop,
        },
        render_model::{model::RenderModel, style::apply_style},
        retained_tree::node::RetainedNode,
    },
};

pub(in crate::gpui_backend) fn render_switch_from_node<'a>(
    node: &RetainedNode,
    child_text: impl IntoIterator<Item = &'a str>,
    dispatch_event: Rc<dyn Fn(RuntimeCommand, &mut App)>,
) -> Option<AnyElement> {
    if !is_switch_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let props = component_props(node);
    let mut switch = Switch::new(("raster-switch", node.id.0));
    if let Some(label) = prop_or_child_text(props, "label", child_text) {
        switch = switch.label(label);
    }
    if let Some(size) = string_prop(props, "size").map(|value| Size::from_str(&value)) {
        switch = switch.with_size(size);
    }
    if let Some(checked) = bool_prop(props, "checked").or_else(|| bool_prop(props, "selected")) {
        switch = switch.checked(checked);
    }
    if let Some(disabled) = bool_prop(props, "disabled") {
        switch = switch.disabled(disabled);
    }
    if let Some(tooltip) = string_prop(props, "tooltip") {
        switch = switch.tooltip(tooltip);
    }

    let on_change = event_handler(node, "onChange");
    let on_click = event_handler(node, "onClick");
    if on_change.is_some() || on_click.is_some() {
        switch = switch.on_click(move |checked, _window, cx| {
            let payload = NodeValue::Bool(*checked);
            if let Some(handler_id) = on_change {
                dispatch_event(
                    RuntimeCommand::InvokeEvent {
                        handler_id,
                        payload: payload.clone(),
                    },
                    cx,
                );
            }
            if let Some(handler_id) = on_click {
                dispatch_event(
                    RuntimeCommand::InvokeEvent {
                        handler_id,
                        payload,
                    },
                    cx,
                );
            }
        });
    }

    Some(apply_style(switch, &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn is_switch_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Switch"
}
