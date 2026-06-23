use std::{cell::RefCell, rc::Rc};

use chrono::NaiveTime;
use gpui::{AnyElement, AppContext, Context, Entity, IntoElement, Subscription, Window};
use gpui_component::{
    Disableable, Sizable, Size,
    time_picker::{TimeFormat, TimePicker, TimePickerEvent, TimePickerState},
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
        retained_tree::node::RetainedNode,
    },
};

pub(in crate::gpui_backend) struct RasterTimePickerState {
    time_picker: Entity<TimePickerState>,
    bindings: Rc<RefCell<TimePickerEventBindings>>,
    controlled_value: Option<Option<NaiveTime>>,
    config: TimePickerConfig,
    _subscription: Subscription,
}

impl RasterTimePickerState {
    pub(in crate::gpui_backend) fn new(
        node: &RetainedNode,
        runtime_commands: ChannelSender<RuntimeCommand>,
        window: &mut Window,
        cx: &mut Context<crate::gpui_backend::app::NodeOwnerView>,
    ) -> Self {
        let config = TimePickerConfig::from_node(node);
        let bindings = Rc::new(RefCell::new(TimePickerEventBindings::from_node(node)));
        let initial_value = controlled_time(node, config.format).flatten();

        let time_picker = cx.new(|cx| {
            let mut state = TimePickerState::new(cx).format(config.format);
            state.set_time(initial_value, false, window, cx);
            state
        });

        let _subscription = cx.subscribe(&time_picker, {
            let bindings = bindings.clone();
            let runtime_commands = runtime_commands.clone();
            let format = config.format;
            move |_, _, event: &TimePickerEvent, _cx| match event {
                TimePickerEvent::Change(time) => {
                    bindings
                        .borrow()
                        .dispatch_change(*time, format, &runtime_commands);
                }
            }
        });

        Self {
            time_picker,
            bindings,
            controlled_value: controlled_time(node, config.format),
            config,
            _subscription,
        }
    }

    pub(in crate::gpui_backend) fn matches_config(&self, node: &RetainedNode) -> bool {
        self.config == TimePickerConfig::from_node(node)
    }

    pub(in crate::gpui_backend) fn sync_from_node(
        &mut self,
        node: &RetainedNode,
        window: &mut Window,
        cx: &mut Context<crate::gpui_backend::app::NodeOwnerView>,
    ) {
        *self.bindings.borrow_mut() = TimePickerEventBindings::from_node(node);

        let next_controlled_value = controlled_time(node, self.config.format);
        if next_controlled_value != self.controlled_value {
            self.time_picker.update(cx, |time_picker, cx| {
                if time_picker.time() != next_controlled_value.flatten() {
                    time_picker.set_time(next_controlled_value.flatten(), false, window, cx);
                }
            });
            self.controlled_value = next_controlled_value;
        }
    }

    pub(in crate::gpui_backend) fn time_picker(&self) -> &Entity<TimePickerState> {
        &self.time_picker
    }
}

pub(in crate::gpui_backend) fn render_time_picker_from_node(
    node: &RetainedNode,
    state: &Entity<TimePickerState>,
) -> Option<AnyElement> {
    if !is_time_picker_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let props = component_props(node);
    let mut time_picker = TimePicker::new(state);
    if let Some(placeholder) = string_prop(props, "placeholder") {
        time_picker = time_picker.placeholder(placeholder);
    }
    if let Some(cleanable) = bool_prop(props, "cleanable") {
        time_picker = time_picker.cleanable(cleanable);
    }
    if let Some(appearance) = bool_prop(props, "appearance") {
        time_picker = time_picker.appearance(appearance);
    }
    if let Some(size) = string_prop(props, "size") {
        time_picker = time_picker.with_size(Size::from_str(&size));
    }
    if bool_prop(props, "disabled") == Some(true) {
        time_picker = time_picker.disabled(true);
    }

    Some(apply_style(time_picker, &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn is_time_picker_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "TimePicker"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TimePickerConfig {
    format: TimeFormat,
}

impl TimePickerConfig {
    fn from_node(node: &RetainedNode) -> Self {
        let props = component_props(node);
        let format = string_prop(props, "format")
            .map(|value| TimeFormat::from_str(&value))
            .unwrap_or_default();
        Self { format }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct TimePickerEventBindings {
    on_change: Option<HandlerId>,
    on_value_change: Option<HandlerId>,
}

impl TimePickerEventBindings {
    fn from_node(node: &RetainedNode) -> Self {
        Self {
            on_change: event_handler(node, "onChange"),
            on_value_change: event_handler(node, "onValueChange"),
        }
    }

    fn dispatch_change(
        &self,
        time: Option<NaiveTime>,
        format: TimeFormat,
        runtime_commands: &ChannelSender<RuntimeCommand>,
    ) {
        let value_payload = time_to_value(time, format);

        if let Some(handler_id) = self.on_change {
            let payload = NodeValue::Object(
                [("value".to_owned(), value_payload.clone())].into(),
            );
            if runtime_commands
                .send(RuntimeCommand::InvokeEvent {
                    handler_id,
                    payload,
                })
                .is_err()
            {
                logger::error("failed to enqueue TimePicker onChange event");
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
                logger::error("failed to enqueue TimePicker onValueChange event");
            }
        }
    }
}

fn controlled_time(node: &RetainedNode, format: TimeFormat) -> Option<Option<NaiveTime>> {
    component_props(node)
        .get("value")
        .map(|value| value_to_time(value, format))
}

fn value_to_time(value: &NodeValue, format: TimeFormat) -> Option<NaiveTime> {
    match value {
        NodeValue::Null => None,
        NodeValue::String(value) if value.is_empty() => None,
        NodeValue::String(value) => parse_time(value, format),
        _ => None,
    }
}

fn parse_time(value: &str, format: TimeFormat) -> Option<NaiveTime> {
    let patterns = match format {
        TimeFormat::Hms => ["%H:%M:%S", "%H:%M"],
        TimeFormat::Hm => ["%H:%M", "%H:%M:%S"],
    };
    for pattern in patterns {
        if let Ok(time) = NaiveTime::parse_from_str(value, pattern) {
            return Some(time);
        }
    }
    logger::warn(format!("invalid TimePicker time value: {value}"));
    None
}

fn time_to_value(time: Option<NaiveTime>, format: TimeFormat) -> NodeValue {
    time.map(|value| NodeValue::String(value.format(format.chrono_format()).to_string()))
        .unwrap_or(NodeValue::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::channel::channel;

    #[test]
    fn hm_value_round_trips() {
        let time = value_to_time(
            &NodeValue::String("08:15".to_owned()),
            TimeFormat::Hm,
        )
        .unwrap();
        assert_eq!(
            time_to_value(Some(time), TimeFormat::Hm),
            NodeValue::String("08:15".to_owned())
        );
    }

    #[test]
    fn hms_value_round_trips() {
        let time = value_to_time(
            &NodeValue::String("08:15:30".to_owned()),
            TimeFormat::Hms,
        )
        .unwrap();
        assert_eq!(
            time_to_value(Some(time), TimeFormat::Hms),
            NodeValue::String("08:15:30".to_owned())
        );
    }

    #[test]
    fn dispatch_change_sends_payload_and_value_events() {
        let (sender, receiver) = channel();
        let bindings = TimePickerEventBindings {
            on_change: Some(HandlerId(1)),
            on_value_change: Some(HandlerId(2)),
        };
        let time = NaiveTime::parse_from_str("08:15", "%H:%M").unwrap();

        bindings.dispatch_change(Some(time), TimeFormat::Hm, &sender);

        let events = receiver.drain();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            RuntimeCommand::InvokeEvent {
                handler_id: HandlerId(1),
                payload: NodeValue::Object(payload),
            } if payload.get("value") == Some(&NodeValue::String("08:15".to_owned()))
        ));
        assert!(matches!(
            &events[1],
            RuntimeCommand::InvokeEvent {
                handler_id: HandlerId(2),
                payload: NodeValue::String(value),
            } if value == "08:15"
        ));
    }
}