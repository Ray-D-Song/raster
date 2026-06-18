use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use gpui::{AnyElement, AppContext, Context, Entity, Hsla, IntoElement, Subscription, Window};
use gpui_component::{
    Anchor, Colorize, Sizable, Size,
    color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState},
};

use crate::{
    common::{
        channel::{ChannelSender, RuntimeCommand},
        ids::HandlerId,
        mount::{NodeValue, RetainedNodeKind},
        utils::logger,
    },
    gpui_backend::{
        components::{
            helper::props::{component_props, display_value, event_handler, string_prop},
            icon::parse_icon_name,
        },
        render_model::{
            model::RenderModel,
            style::{apply_style, parse_color},
        },
        retained_tree::node::RetainedNode,
    },
};

pub(in crate::gpui_backend) struct RasterColorPickerState {
    color_picker: Entity<ColorPickerState>,
    bindings: Rc<RefCell<ColorPickerEventBindings>>,
    controlled_value: Option<Option<Hsla>>,
    config: ColorPickerConfig,
    _subscription: Subscription,
}

impl RasterColorPickerState {
    pub(in crate::gpui_backend) fn new(
        node: &RetainedNode,
        runtime_commands: ChannelSender<RuntimeCommand>,
        window: &mut Window,
        cx: &mut Context<crate::gpui_backend::app::NodeOwnerView>,
    ) -> Self {
        let config = ColorPickerConfig::from_node(node);
        let initial_value = controlled_value(node).unwrap_or_else(|| default_value(node));
        let bindings = Rc::new(RefCell::new(ColorPickerEventBindings::from_node(node)));

        let color_picker = cx.new(|cx| {
            let mut state = ColorPickerState::new(window, cx);
            if let Some(value) = initial_value {
                state = state.default_value(value);
            }
            state
        });

        let _subscription = cx.subscribe(&color_picker, {
            let bindings = bindings.clone();
            let runtime_commands = runtime_commands.clone();
            move |_, _, event: &ColorPickerEvent, _cx| match event {
                ColorPickerEvent::Change(value) => {
                    bindings.borrow().dispatch_change(*value, &runtime_commands);
                }
            }
        });

        Self {
            color_picker,
            bindings,
            controlled_value: controlled_value(node),
            config,
            _subscription,
        }
    }

    pub(in crate::gpui_backend) fn matches_config(&self, node: &RetainedNode) -> bool {
        self.config == ColorPickerConfig::from_node(node)
            && !(controlled_value(node) == Some(None) && self.controlled_value != Some(None))
    }

    pub(in crate::gpui_backend) fn sync_from_node(
        &mut self,
        node: &RetainedNode,
        window: &mut Window,
        cx: &mut Context<crate::gpui_backend::app::NodeOwnerView>,
    ) {
        *self.bindings.borrow_mut() = ColorPickerEventBindings::from_node(node);

        let next_controlled_value = controlled_value(node);
        if next_controlled_value != self.controlled_value
            && let Some(Some(value)) = next_controlled_value
        {
            self.color_picker.update(cx, |color_picker, cx| {
                if color_picker.value() != Some(value) {
                    color_picker.set_value(value, window, cx);
                }
            });
        }
        self.controlled_value = next_controlled_value;
    }

    pub(in crate::gpui_backend) fn color_picker(&self) -> &Entity<ColorPickerState> {
        &self.color_picker
    }
}

pub(in crate::gpui_backend) fn render_color_picker_from_node(
    node: &RetainedNode,
    state: &Entity<ColorPickerState>,
) -> Option<AnyElement> {
    if !is_color_picker_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let props = component_props(node);
    let mut color_picker = ColorPicker::new(state);

    if let Some(colors) = parse_featured_colors(props.get("featuredColors")) {
        color_picker = color_picker.featured_colors(colors);
    }
    if let Some(label) = props.get("label").map(display_value) {
        color_picker = color_picker.label(label);
    }
    if let Some(icon) = string_prop(props, "icon").and_then(|value| parse_icon_name(&value)) {
        color_picker = color_picker.icon(icon);
    }
    if let Some(size) = string_prop(props, "size") {
        color_picker = color_picker.with_size(Size::from_str(&size));
    }
    if let Some(anchor) = string_prop(props, "anchor").and_then(|value| parse_anchor(&value)) {
        color_picker = color_picker.anchor(anchor);
    }

    Some(apply_style(color_picker, &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn is_color_picker_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "ColorPicker"
}

#[derive(Clone, Debug, PartialEq)]
struct ColorPickerConfig {
    featured_colors: Option<Vec<Hsla>>,
    icon: Option<String>,
    anchor: Option<String>,
}

impl ColorPickerConfig {
    fn from_node(node: &RetainedNode) -> Self {
        let props = component_props(node);
        Self {
            featured_colors: parse_featured_colors(props.get("featuredColors")),
            icon: string_prop(props, "icon"),
            anchor: string_prop(props, "anchor"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ColorPickerEventBindings {
    on_change: Option<HandlerId>,
    on_value_change: Option<HandlerId>,
}

impl ColorPickerEventBindings {
    fn from_node(node: &RetainedNode) -> Self {
        Self {
            on_change: event_handler(node, "onChange"),
            on_value_change: event_handler(node, "onValueChange"),
        }
    }

    fn dispatch_change(
        &self,
        value: Option<Hsla>,
        runtime_commands: &ChannelSender<RuntimeCommand>,
    ) {
        let value_payload = value
            .map(|color| NodeValue::String(color.to_hex().to_string()))
            .unwrap_or(NodeValue::Null);

        if let Some(handler_id) = self.on_change {
            let payload = NodeValue::Object(
                [("value".to_owned(), value_payload.clone())]
                    .into_iter()
                    .collect::<BTreeMap<_, _>>(),
            );

            if runtime_commands
                .send(RuntimeCommand::InvokeEvent {
                    handler_id,
                    payload,
                })
                .is_err()
            {
                logger::error("failed to enqueue ColorPicker onChange event");
            }
        }

        if let Some(handler_id) = self.on_value_change {
            if runtime_commands
                .send(RuntimeCommand::InvokeEvent {
                    handler_id,
                    payload: value_payload,
                })
                .is_err()
            {
                logger::error("failed to enqueue ColorPicker onValueChange event");
            }
        }
    }
}

fn controlled_value(node: &RetainedNode) -> Option<Option<Hsla>> {
    let props = component_props(node);
    match props.get("value") {
        Some(NodeValue::Null) => Some(None),
        Some(value) => Some(parse_color_prop(value, "ColorPicker value")),
        None => None,
    }
}

fn default_value(node: &RetainedNode) -> Option<Hsla> {
    component_props(node)
        .get("defaultValue")
        .and_then(|value| parse_color_prop(value, "ColorPicker defaultValue"))
}

fn parse_color_prop(value: &NodeValue, context: &str) -> Option<Hsla> {
    let color = display_value(value);
    let parsed = parse_color(&color);
    if parsed.is_none() && !color.is_empty() {
        logger::warn(format!("invalid {context}: {color}"));
    }
    parsed
}

fn parse_featured_colors(value: Option<&NodeValue>) -> Option<Vec<Hsla>> {
    let NodeValue::Array(values) = value? else {
        return None;
    };

    Some(
        values
            .iter()
            .filter_map(|value| parse_color_prop(value, "ColorPicker featuredColors item"))
            .collect(),
    )
}

fn parse_anchor(value: &str) -> Option<Anchor> {
    Some(match value {
        "topLeft" => Anchor::TopLeft,
        "topCenter" => Anchor::TopCenter,
        "topRight" => Anchor::TopRight,
        "bottomLeft" => Anchor::BottomLeft,
        "bottomCenter" => Anchor::BottomCenter,
        "bottomRight" => Anchor::BottomRight,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::channel::channel;

    #[test]
    fn dispatch_change_sends_payload_and_value_events() {
        let (sender, receiver) = channel();
        let bindings = ColorPickerEventBindings {
            on_change: Some(HandlerId(1)),
            on_value_change: Some(HandlerId(2)),
        };
        let color = parse_color("#ff0000").expect("valid color");
        let expected = NodeValue::String(color.to_hex().to_string());

        bindings.dispatch_change(Some(color), &sender);

        let events = receiver.drain();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            RuntimeCommand::InvokeEvent {
                handler_id: HandlerId(1),
                payload: NodeValue::Object(payload),
            } if payload.get("value") == Some(&expected)
        ));
        assert!(matches!(
            &events[1],
            RuntimeCommand::InvokeEvent {
                handler_id: HandlerId(2),
                payload,
            } if payload == &expected
        ));
    }

    #[test]
    fn dispatch_change_sends_null_value_when_cleared() {
        let (sender, receiver) = channel();
        let bindings = ColorPickerEventBindings {
            on_change: None,
            on_value_change: Some(HandlerId(2)),
        };

        bindings.dispatch_change(None, &sender);

        let events = receiver.drain();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            RuntimeCommand::InvokeEvent {
                handler_id: HandlerId(2),
                payload: NodeValue::Null,
            }
        ));
    }
}
