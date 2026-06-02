use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use chrono::NaiveDate;
use gpui::{AnyElement, AppContext, Context, Entity, IntoElement, Subscription, Window};
use gpui_component::{
    Disableable, Sizable, Size,
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
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
            bool_prop, component_props, event_handler, number_prop, string_prop,
        },
        render_model::{model::RenderModel, style::apply_style},
        retained_tree::node::RetainedNode,
    },
};

use gpui_component::calendar::{Date, Matcher};

pub(in crate::gpui_backend) struct RasterDatePickerState {
    date_picker: Entity<DatePickerState>,
    bindings: Rc<RefCell<DatePickerEventBindings>>,
    controlled_value: Option<Date>,
    config: DatePickerConfig,
    _subscription: Subscription,
}

impl RasterDatePickerState {
    pub(in crate::gpui_backend) fn new(
        node: &RetainedNode,
        runtime_commands: ChannelSender<RuntimeCommand>,
        window: &mut Window,
        cx: &mut Context<crate::gpui_backend::app::NodeOwnerView>,
    ) -> Self {
        let config = DatePickerConfig::from_node(node);
        let initial_value =
            controlled_date(node, config.mode).unwrap_or_else(|| empty_date(config.mode));
        let bindings = Rc::new(RefCell::new(DatePickerEventBindings::from_node(
            node,
            config.mode,
        )));

        let date_picker = cx.new(|cx| {
            let mut state = match config.mode {
                DateSelectionMode::Single => DatePickerState::new(window, cx),
                DateSelectionMode::Range => DatePickerState::range(window, cx),
            }
            .date_format("%Y-%m-%d");

            if let Some(matcher) = parse_disabled_matcher(config.disabled_matcher.as_ref()) {
                state = state.disabled_matcher(matcher);
            }
            state.set_date(initial_value, window, cx);
            state
        });

        let _subscription = cx.subscribe(&date_picker, {
            let bindings = bindings.clone();
            let runtime_commands = runtime_commands.clone();
            move |_, _, event: &DatePickerEvent, _cx| match event {
                DatePickerEvent::Change(date) => {
                    bindings.borrow().dispatch_change(*date, &runtime_commands);
                }
            }
        });

        Self {
            date_picker,
            bindings,
            controlled_value: controlled_date(node, config.mode),
            config,
            _subscription,
        }
    }

    pub(in crate::gpui_backend) fn matches_config(&self, node: &RetainedNode) -> bool {
        self.config == DatePickerConfig::from_node(node)
    }

    pub(in crate::gpui_backend) fn sync_from_node(
        &mut self,
        node: &RetainedNode,
        window: &mut Window,
        cx: &mut Context<crate::gpui_backend::app::NodeOwnerView>,
    ) {
        *self.bindings.borrow_mut() = DatePickerEventBindings::from_node(node, self.config.mode);

        let next_controlled_value = controlled_date(node, self.config.mode);
        if next_controlled_value != self.controlled_value
            && let Some(value) = next_controlled_value
        {
            self.date_picker.update(cx, |date_picker, cx| {
                if date_picker.date() != value {
                    date_picker.set_date(value, window, cx);
                }
            });
        }
        self.controlled_value = next_controlled_value;
    }

    pub(in crate::gpui_backend) fn date_picker(&self) -> &Entity<DatePickerState> {
        &self.date_picker
    }
}

pub(in crate::gpui_backend) fn render_date_picker_from_node(
    node: &RetainedNode,
    state: &Entity<DatePickerState>,
) -> Option<AnyElement> {
    if !is_date_picker_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let props = component_props(node);
    let mut date_picker = DatePicker::new(state).number_of_months(
        number_prop(props, "numberOfMonths")
            .map(|value| value.max(1.0) as usize)
            .unwrap_or(1),
    );
    if let Some(placeholder) = string_prop(props, "placeholder") {
        date_picker = date_picker.placeholder(placeholder);
    }
    if let Some(cleanable) = bool_prop(props, "cleanable") {
        date_picker = date_picker.cleanable(cleanable);
    }
    if let Some(appearance) = bool_prop(props, "appearance") {
        date_picker = date_picker.appearance(appearance);
    }
    if let Some(size) = string_prop(props, "size") {
        date_picker = date_picker.with_size(Size::from_str(&size));
    }
    if bool_prop(props, "disabled") == Some(true) {
        date_picker = date_picker.disabled(true);
    }

    Some(apply_style(date_picker, &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn is_date_picker_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "DatePicker"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DateSelectionMode {
    Single,
    Range,
}

impl DateSelectionMode {
    fn as_str(self) -> &'static str {
        match self {
            DateSelectionMode::Single => "single",
            DateSelectionMode::Range => "range",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DatePickerConfig {
    mode: DateSelectionMode,
    disabled_matcher: Option<NodeValue>,
}

impl DatePickerConfig {
    fn from_node(node: &RetainedNode) -> Self {
        let props = component_props(node);
        let mode = match string_prop(props, "mode").as_deref() {
            Some("range") => DateSelectionMode::Range,
            _ => DateSelectionMode::Single,
        };
        let disabled_matcher = props
            .get("disabled")
            .filter(|value| !matches!(value, NodeValue::Bool(_)))
            .cloned();

        Self {
            mode,
            disabled_matcher,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct DatePickerEventBindings {
    mode: DateSelectionMode,
    on_change: Option<HandlerId>,
}

impl DatePickerEventBindings {
    fn from_node(node: &RetainedNode, mode: DateSelectionMode) -> Self {
        Self {
            mode,
            on_change: event_handler(node, "onChange"),
        }
    }

    fn dispatch_change(&self, date: Date, runtime_commands: &ChannelSender<RuntimeCommand>) {
        let Some(handler_id) = self.on_change else {
            return;
        };

        let payload = NodeValue::Object(
            [
                (
                    "mode".to_owned(),
                    NodeValue::String(self.mode.as_str().to_owned()),
                ),
                ("value".to_owned(), date_to_value(date, self.mode)),
            ]
            .into(),
        );
        if runtime_commands
            .send(RuntimeCommand::InvokeEvent {
                handler_id,
                payload,
            })
            .is_err()
        {
            logger::error("failed to enqueue DatePicker onChange event");
        }
    }
}

impl Default for DateSelectionMode {
    fn default() -> Self {
        Self::Single
    }
}

fn controlled_date(node: &RetainedNode, mode: DateSelectionMode) -> Option<Date> {
    component_props(node)
        .get("value")
        .map(|value| value_to_date(value, mode))
        .unwrap_or_else(|| Some(empty_date(mode)))
}

fn empty_date(mode: DateSelectionMode) -> Date {
    match mode {
        DateSelectionMode::Single => Date::Single(None),
        DateSelectionMode::Range => Date::Range(None, None),
    }
}

fn value_to_date(value: &NodeValue, mode: DateSelectionMode) -> Option<Date> {
    match mode {
        DateSelectionMode::Single => match value {
            NodeValue::Null => Some(Date::Single(None)),
            NodeValue::String(value) if value.is_empty() => Some(Date::Single(None)),
            NodeValue::String(value) => parse_iso_date(value).map(|date| Date::Single(Some(date))),
            _ => Some(Date::Single(None)),
        },
        DateSelectionMode::Range => match value {
            NodeValue::Null => Some(Date::Range(None, None)),
            NodeValue::Array(values) => {
                let start = values.first().and_then(parse_optional_date);
                let end = values.get(1).and_then(parse_optional_date);
                Some(Date::Range(start, end))
            }
            _ => Some(Date::Range(None, None)),
        },
    }
}

fn date_to_value(date: Date, mode: DateSelectionMode) -> NodeValue {
    match (mode, date) {
        (DateSelectionMode::Single, Date::Single(value)) => optional_date_to_value(value),
        (DateSelectionMode::Single, Date::Range(start, _)) => optional_date_to_value(start),
        (DateSelectionMode::Range, Date::Range(start, end)) => NodeValue::Array(vec![
            optional_date_to_value(start),
            optional_date_to_value(end),
        ]),
        (DateSelectionMode::Range, Date::Single(value)) => {
            NodeValue::Array(vec![optional_date_to_value(value), NodeValue::Null])
        }
    }
}

fn optional_date_to_value(value: Option<NaiveDate>) -> NodeValue {
    value
        .map(|date| NodeValue::String(date.format("%Y-%m-%d").to_string()))
        .unwrap_or(NodeValue::Null)
}

fn parse_optional_date(value: &NodeValue) -> Option<NaiveDate> {
    match value {
        NodeValue::String(value) if !value.is_empty() => parse_iso_date(value),
        _ => None,
    }
}

fn parse_iso_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| logger::warn(format!("invalid DatePicker ISO date: {value}")))
        .ok()
}

fn parse_disabled_matcher(value: Option<&NodeValue>) -> Option<Matcher> {
    let value = value?;
    match value {
        NodeValue::String(value) => {
            let date = parse_iso_date(value)?;
            Some(Matcher::custom(move |target| *target == date))
        }
        NodeValue::Object(object) => parse_disabled_object(object),
        NodeValue::Array(values) => {
            let matchers = values
                .iter()
                .filter_map(|value| parse_disabled_matcher(Some(value)))
                .collect::<Vec<_>>();
            if matchers.is_empty() {
                None
            } else {
                Some(Matcher::custom(move |date| {
                    matchers
                        .iter()
                        .any(|matcher| matcher.is_match(&Date::Single(Some(*date))))
                }))
            }
        }
        _ => None,
    }
}

fn parse_disabled_object(object: &BTreeMap<String, NodeValue>) -> Option<Matcher> {
    if let Some(days) = object.get("dayOfWeek").and_then(parse_day_of_week) {
        return Some(Matcher::from(days));
    }

    let before = object
        .get("before")
        .and_then(NodeValue::as_str)
        .and_then(parse_iso_date);
    let after = object
        .get("after")
        .and_then(NodeValue::as_str)
        .and_then(parse_iso_date);
    if before.is_some() || after.is_some() {
        return Some(Matcher::interval(before, after));
    }

    let from = object
        .get("from")
        .and_then(NodeValue::as_str)
        .and_then(parse_iso_date);
    let to = object
        .get("to")
        .and_then(NodeValue::as_str)
        .and_then(parse_iso_date);
    if from.is_some() || to.is_some() {
        return Some(Matcher::range(from, to));
    }

    None
}

fn parse_day_of_week(value: &NodeValue) -> Option<Vec<u32>> {
    let NodeValue::Array(values) = value else {
        return None;
    };
    let days = values
        .iter()
        .filter_map(|value| match value {
            NodeValue::Number(value) => Some((*value).clamp(0.0, 6.0) as u32),
            NodeValue::String(value) => value.parse::<u32>().ok().map(|value| value.min(6)),
            _ => None,
        })
        .collect::<Vec<_>>();
    (!days.is_empty()).then_some(days)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_value_round_trips_iso_date() {
        let date = value_to_date(
            &NodeValue::String("2026-05-23".to_owned()),
            DateSelectionMode::Single,
        )
        .unwrap();

        assert_eq!(
            date_to_value(date, DateSelectionMode::Single),
            NodeValue::String("2026-05-23".to_owned())
        );
    }

    #[test]
    fn range_value_round_trips_partial_dates() {
        let date = value_to_date(
            &NodeValue::Array(vec![
                NodeValue::String("2026-05-01".to_owned()),
                NodeValue::Null,
            ]),
            DateSelectionMode::Range,
        )
        .unwrap();

        assert_eq!(
            date_to_value(date, DateSelectionMode::Range),
            NodeValue::Array(vec![
                NodeValue::String("2026-05-01".to_owned()),
                NodeValue::Null,
            ])
        );
    }

    #[test]
    fn disabled_matcher_supports_array_union() {
        let matcher = parse_disabled_matcher(Some(&NodeValue::Array(vec![
            NodeValue::Object(BTreeMap::from([(
                "dayOfWeek".to_owned(),
                NodeValue::Array(vec![NodeValue::Number(0.0)]),
            )])),
            NodeValue::String("2026-05-23".to_owned()),
        ])))
        .unwrap();

        let sunday = NaiveDate::parse_from_str("2026-05-24", "%Y-%m-%d").unwrap();
        let exact = NaiveDate::parse_from_str("2026-05-23", "%Y-%m-%d").unwrap();
        let allowed = NaiveDate::parse_from_str("2026-05-25", "%Y-%m-%d").unwrap();

        assert!(matcher.is_match(&Date::Single(Some(sunday))));
        assert!(matcher.is_match(&Date::Single(Some(exact))));
        assert!(!matcher.is_match(&Date::Single(Some(allowed))));
    }
}
