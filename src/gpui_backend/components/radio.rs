use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use gpui::{AnyElement, App, Axis, IntoElement};
use gpui_component::{
    Sizable, Size,
    radio::{Radio, RadioGroup},
};

use crate::{
    common::{
        channel::{ChannelSender, RuntimeCommand},
        ids::HandlerId,
        mount::{NodeValue, RetainedNodeKind},
        utils::logger,
    },
    gpui_backend::{
        components::helper::props::{
            bool_prop, component_props, event_handler, number_prop, prop_or_child_text, string_prop,
        },
        render_model::{model::RenderModel, style::apply_style},
        retained_tree::{node::RetainedNode, tree::RetainedTree},
    },
};

pub(in crate::gpui_backend) fn render_radio_from_node<'a>(
    node: &RetainedNode,
    child_text: impl IntoIterator<Item = &'a str>,
    dispatch_event: Rc<dyn Fn(RuntimeCommand, &mut App)>,
) -> Option<AnyElement> {
    if !is_radio_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let props = component_props(node);
    let mut radio = build_radio(node, prop_or_child_text(props, "label", child_text), props);

    let on_change = event_handler(node, "onChange");
    let on_click = event_handler(node, "onClick");
    if on_change.is_some() || on_click.is_some() {
        radio = radio.on_click(move |checked, _window, cx| {
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

    Some(apply_style(radio, &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn render_radio_group_from_node(
    node: &RetainedNode,
    tree: &Rc<RefCell<RetainedTree>>,
    runtime_commands: ChannelSender<RuntimeCommand>,
) -> Option<AnyElement> {
    if !is_radio_group_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let props = component_props(node);
    let mut group = RadioGroup::vertical(("raster-radio-group", node.id.0));
    if let Some(axis) = string_prop(props, "layout")
        .or_else(|| string_prop(props, "axis"))
        .as_deref()
        .map(parse_axis)
    {
        group = group.layout(axis);
    }
    if let Some(disabled) = bool_prop(props, "disabled") {
        group = group.disabled(disabled);
    }
    if let Some(selected_index) = number_prop(props, "selectedIndex") {
        group = group.selected_index(Some(selected_index.max(0.0) as usize));
    }

    let group_on_change = event_handler(node, "onChange");
    let group_on_click = event_handler(node, "onClick");
    let mut radio_clicks = Vec::new();

    let tree_ref = tree.borrow();
    for child_id in node.children.iter().copied() {
        let Some(child) = tree_ref.node(child_id) else {
            continue;
        };
        if !is_radio_node(child) {
            logger::warn(format!(
                "RadioGroup only supports direct Radio children; ignored {}",
                child.component_name()
            ));
            continue;
        }

        let child_props = component_props(child);
        let radio = build_radio(child, radio_label(child, &tree_ref), child_props);
        radio_clicks.push(event_handler(child, "onClick"));
        group = group.child(radio);
    }
    drop(tree_ref);

    if group_on_change.is_some()
        || group_on_click.is_some()
        || radio_clicks.iter().any(Option::is_some)
    {
        group = group.on_click(move |index, _window, _cx| {
            if let Some(Some(handler_id)) = radio_clicks.get(*index) {
                send_event(
                    &runtime_commands,
                    *handler_id,
                    NodeValue::String(String::new()),
                    "RadioGroup child onClick",
                );
            }
            if let Some(handler_id) = group_on_change {
                send_event(
                    &runtime_commands,
                    handler_id,
                    NodeValue::String(index.to_string()),
                    "RadioGroup onChange",
                );
            }
            if let Some(handler_id) = group_on_click {
                send_event(
                    &runtime_commands,
                    handler_id,
                    NodeValue::String(index.to_string()),
                    "RadioGroup onClick",
                );
            }
        });
    }

    Some(apply_style(group, &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn is_radio_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Radio"
}

pub(in crate::gpui_backend) fn is_radio_group_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "RadioGroup"
}

fn build_radio(
    node: &RetainedNode,
    label: Option<String>,
    props: &BTreeMap<String, NodeValue>,
) -> Radio {
    let mut radio = Radio::new(("raster-radio", node.id.0));
    if let Some(label) = label {
        radio = radio.label(label);
    }
    if let Some(size) = string_prop(props, "size").map(|value| Size::from_str(&value)) {
        radio = radio.with_size(size);
    }
    if let Some(checked) = bool_prop(props, "checked").or_else(|| bool_prop(props, "selected")) {
        radio = radio.checked(checked);
    }
    if let Some(disabled) = bool_prop(props, "disabled") {
        radio = radio.disabled(disabled);
    }
    if let Some(tab_index) = number_prop(props, "tabIndex") {
        radio = radio.tab_index(tab_index as isize);
    }
    if let Some(tab_stop) = bool_prop(props, "tabStop") {
        radio = radio.tab_stop(tab_stop);
    }
    radio
}

fn radio_label(node: &RetainedNode, tree: &RetainedTree) -> Option<String> {
    string_prop(component_props(node), "label").or_else(|| {
        let text = node
            .children
            .iter()
            .filter_map(|child_id| tree.node(*child_id))
            .filter_map(|child| child.payload.text.as_deref())
            .collect::<String>();
        (!text.is_empty()).then_some(text)
    })
}

fn send_event(
    runtime_commands: &ChannelSender<RuntimeCommand>,
    handler_id: HandlerId,
    payload: NodeValue,
    label: &str,
) {
    if runtime_commands
        .send(RuntimeCommand::InvokeEvent {
            handler_id,
            payload,
        })
        .is_err()
    {
        logger::error(format!("failed to enqueue {label} event"));
    }
}

fn parse_axis(value: &str) -> Axis {
    match value {
        "horizontal" | "row" => Axis::Horizontal,
        _ => Axis::Vertical,
    }
}
