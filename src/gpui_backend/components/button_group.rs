use std::{cell::RefCell, rc::Rc};

use gpui::{AnyElement, Axis, IntoElement};
use gpui_component::{
    Disableable, Selectable, Sizable, Size,
    button::{Button, ButtonGroup, ButtonVariant, ButtonVariants},
};

use crate::{
    common::{
        channel::{ChannelSender, RuntimeCommand},
        ids::HandlerId,
        mount::{NodeValue, RetainedNodeKind},
        utils::logger,
    },
    gpui_backend::{
        components::helper::props::{bool_prop, component_props, event_handler, string_prop},
        render_model::{model::RenderModel, style::apply_style},
        retained_tree::{node::RetainedNode, tree::RetainedTree},
    },
};

pub(in crate::gpui_backend) fn render_button_group_from_node(
    node: &RetainedNode,
    tree: &Rc<RefCell<RetainedTree>>,
    runtime_commands: ChannelSender<RuntimeCommand>,
) -> Option<AnyElement> {
    if !is_button_group_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let group_props = component_props(node);
    let controlled_value = group_props.get("value").cloned();
    let group_on_change = event_handler(node, "onChange");
    let group_on_click = event_handler(node, "onClick");
    let mut button_values = Vec::new();
    let mut button_clicks = Vec::new();
    let mut group = ButtonGroup::new(("raster-button-group", node.id.0));

    if bool_prop(group_props, "multiple") == Some(true) {
        group = group.multiple(true);
    }
    if let Some(disabled) = bool_prop(group_props, "disabled") {
        group = group.disabled(disabled);
    }
    if bool_prop(group_props, "compact") == Some(true) {
        group = group.compact();
    }
    if bool_prop(group_props, "outline") == Some(true) {
        group = group.outline();
    }
    if let Some(size) = string_prop(group_props, "size").map(|value| Size::from_str(&value)) {
        group = group.with_size(size);
    }
    if let Some(variant) = string_prop(group_props, "variant").and_then(parse_button_variant) {
        group = group.with_variant(variant);
    }
    if let Some(axis) = string_prop(group_props, "layout")
        .or_else(|| string_prop(group_props, "axis"))
        .as_deref()
        .map(parse_axis)
    {
        group = group.layout(axis);
    }

    let tree_ref = tree.borrow();
    for (index, child_id) in node.children.iter().copied().enumerate() {
        let Some(child) = tree_ref.node(child_id) else {
            continue;
        };
        if !is_button_node(child) {
            logger::warn(format!(
                "ButtonGroup only supports direct Button children; ignored {}",
                child.component_name()
            ));
            continue;
        }

        let child_props = component_props(child);
        let child_value = child_props
            .get("value")
            .cloned()
            .unwrap_or(NodeValue::Number(index as f64));
        let child_on_click = event_handler(child, "onClick");
        let selected = controlled_value
            .as_ref()
            .map(|value| value == &child_value)
            .unwrap_or_else(|| bool_prop(child_props, "selected") == Some(true));

        let mut button = Button::new(("raster-button-group-child", child.id.0)).secondary();
        if let Some(label) = button_label(child, &tree_ref) {
            button = button.label(label);
        }
        if let Some(size) = string_prop(child_props, "size").map(|value| Size::from_str(&value)) {
            button = button.with_size(size);
        }
        if let Some(variant) = string_prop(child_props, "variant").and_then(parse_button_variant) {
            button = button.with_variant(variant);
        }
        if let Some(disabled) = bool_prop(child_props, "disabled") {
            button = button.disabled(disabled);
        }
        if selected {
            button = button.selected(true);
        }
        if bool_prop(child_props, "compact") == Some(true) {
            button = button.compact();
        }
        if bool_prop(child_props, "outline") == Some(true) {
            button = button.outline();
        }

        button_values.push(child_value);
        button_clicks.push(child_on_click);
        group = group.child(button);
    }
    drop(tree_ref);

    if group_on_change.is_some()
        || group_on_click.is_some()
        || button_clicks.iter().any(Option::is_some)
    {
        group = group.on_click(move |selected_indices, _window, _cx| {
            let Some(index) = selected_indices.first().copied() else {
                return;
            };

            if let Some(Some(handler_id)) = button_clicks.get(index) {
                send_event(
                    &runtime_commands,
                    *handler_id,
                    NodeValue::String(String::new()),
                    "ButtonGroup child onClick",
                );
            }
            if let Some(handler_id) = group_on_change {
                if let Some(value) = button_values.get(index).cloned() {
                    send_event(&runtime_commands, handler_id, value, "ButtonGroup onChange");
                }
            }
            if let Some(handler_id) = group_on_click {
                send_event(
                    &runtime_commands,
                    handler_id,
                    NodeValue::String(index.to_string()),
                    "ButtonGroup onClick",
                );
            }
        });
    }

    Some(apply_style(group, &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn is_button_group_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "ButtonGroup"
}

fn is_button_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Button"
}

fn button_label(node: &RetainedNode, tree: &RetainedTree) -> Option<String> {
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
        "vertical" | "y" => Axis::Vertical,
        _ => Axis::Horizontal,
    }
}

fn parse_button_variant(value: String) -> Option<ButtonVariant> {
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
