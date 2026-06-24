use std::{cell::RefCell, rc::Rc};

use gpui::{AnyElement, AppContext, Context, Entity, IntoElement, Subscription, Window};
use gpui_component::{
    Selectable, Sizable, Size,
    input::{Input, InputEvent, InputState},
};

use crate::{
    bridge::BridgeEventDispatch,
    common::{
        ids::HandlerId,
        mount::{NodeValue, RetainedNodeKind},
    },
    gpui_backend::{
        components::helper::props::{
            bool_prop, component_props, event_handler, number_prop, string_prop,
        },
        render_model::{model::RenderModel, style::apply_style},
        retained_tree::node::RetainedNode,
    },
};

pub(in crate::gpui_backend) struct RasterInputState {
    input: Entity<InputState>,
    event_bindings: Rc<RefCell<InputEventBindings>>,
    sync_state: Rc<RefCell<TextControlSyncState>>,
    _subscription: Subscription,
    controlled_value: Option<String>,
    config: TextControlConfig,
}

impl RasterInputState {
    pub(in crate::gpui_backend) fn new(
        node: &RetainedNode,
        dispatch_event: BridgeEventDispatch,
        window: &mut Window,
        cx: &mut Context<crate::gpui_backend::app::NodeOwnerView>,
    ) -> Self {
        let initial_value = string_prop(component_props(node), "value")
            .or_else(|| string_prop(component_props(node), "defaultValue"))
            .unwrap_or_default();
        let config = TextControlConfig::from_node(node);
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).default_value(initial_value.clone());
            if config.multiline {
                state = state.auto_grow(config.rows, config.rows);
            }
            if let Some(placeholder) = string_prop(component_props(node), "placeholder") {
                state = state.placeholder(placeholder);
            }
            if !config.multiline
                && bool_prop(component_props(node), "secureTextEntry") == Some(true)
            {
                state = state.masked(true);
            }
            state
        });

        let event_bindings = Rc::new(RefCell::new(InputEventBindings::from_node(node)));
        let sync_state = Rc::new(RefCell::new(TextControlSyncState::default()));
        let _subscription = cx.subscribe(&input, {
            let dispatch_event = dispatch_event.clone();
            let event_bindings = event_bindings.clone();
            let sync_state = sync_state.clone();
            move |_, input, event: &InputEvent, cx| {
                let value = input.read(cx).value().to_string();
                let event_bindings = event_bindings.borrow();
                let event_count = if matches!(event, InputEvent::Change) {
                    let mut sync_state = sync_state.borrow_mut();
                    sync_state.native_event_count = sync_state.native_event_count.saturating_add(1);
                    sync_state.native_event_count
                } else {
                    sync_state.borrow().native_event_count
                };
                match event {
                    InputEvent::Change => {
                        event_bindings.dispatch_change(
                            &value,
                            event_count,
                            &dispatch_event,
                        );
                    }
                    InputEvent::PressEnter { .. } => {
                        event_bindings.dispatch_submit(&value, &dispatch_event);
                    }
                    InputEvent::Focus => {
                        event_bindings.dispatch_string(
                            "onFocus",
                            &value,
                            &dispatch_event,
                        );
                    }
                    InputEvent::Blur => {
                        event_bindings.dispatch_string(
                            "onBlur",
                            &value,
                            &dispatch_event,
                        );
                    }
                }
            }
        });

        Self {
            input,
            event_bindings,
            sync_state,
            _subscription,
            controlled_value: string_prop(component_props(node), "value"),
            config,
        }
    }

    pub(in crate::gpui_backend) fn matches_config(&self, node: &RetainedNode) -> bool {
        self.config == TextControlConfig::from_node(node)
    }

    pub(in crate::gpui_backend) fn sync_from_node(
        &mut self,
        node: &RetainedNode,
        window: &mut Window,
        cx: &mut Context<crate::gpui_backend::app::NodeOwnerView>,
    ) {
        *self.event_bindings.borrow_mut() = InputEventBindings::from_node(node);

        if let Some(placeholder) = string_prop(component_props(node), "placeholder") {
            self.input.update(cx, |input, cx| {
                input.set_placeholder(placeholder, window, cx);
            });
        }

        if !self.config.multiline
            && let Some(masked) = bool_prop(component_props(node), "secureTextEntry")
        {
            self.input.update(cx, |input, cx| {
                input.set_masked(masked, window, cx);
            });
        }

        let next_controlled_value = string_prop(component_props(node), "value");
        if let Some(value) = next_controlled_value.clone() {
            let prop_event_count = text_event_count_prop(node);
            let sync_state = self.sync_state.clone();
            self.input.update(cx, |input, cx| {
                let native_value = input.value().to_string();
                let mut sync_state = sync_state.borrow_mut();
                let native_event_count = sync_state.native_event_count;
                match decide_text_control_sync(
                    prop_event_count,
                    native_event_count,
                    &value,
                    &native_value,
                ) {
                    TextControlSyncDecision::SkipStale => return,
                    TextControlSyncDecision::Ack => {
                        sync_state.last_acked_event_count = prop_event_count;
                        return;
                    }
                    TextControlSyncDecision::Apply => {
                        sync_state.last_acked_event_count = prop_event_count;
                        input.set_value(value, window, cx);
                    }
                }
            });
        }
        self.controlled_value = next_controlled_value;
    }

    pub(in crate::gpui_backend) fn input(&self) -> &Entity<InputState> {
        &self.input
    }
}

pub(in crate::gpui_backend) fn render_input_from_node(
    node: &RetainedNode,
    state: &Entity<InputState>,
) -> Option<AnyElement> {
    if !is_text_control_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let mut input = Input::new(state);
    if let Some(size) = string_prop(component_props(node), "size") {
        input = input.with_size(Size::from_str(&size));
    }
    if let Some(disabled) = disabled_prop(node) {
        input = input.disabled(disabled);
    }
    if let Some(selected) = bool_prop(component_props(node), "selected") {
        input = input.selected(selected);
    }
    if let Some(appearance) = bool_prop(component_props(node), "appearance") {
        input = input.appearance(appearance);
    }
    if let Some(bordered) = bool_prop(component_props(node), "bordered") {
        input = input.bordered(bordered);
    }
    if let Some(focus_bordered) = bool_prop(component_props(node), "focusBordered") {
        input = input.focus_bordered(focus_bordered);
    }
    if let Some(cleanable) = bool_prop(component_props(node), "cleanable") {
        input = input.cleanable(cleanable);
    }
    if bool_prop(component_props(node), "maskToggle") == Some(true) {
        input = input.mask_toggle();
    }
    if let Some(tab_index) = number_prop(component_props(node), "tabIndex") {
        input = input.tab_index(tab_index as isize);
    }
    Some(apply_style(input, &model.style).into_any_element())
}

#[derive(Clone, Copy, Debug, Default)]
struct InputEventBindings {
    on_change: Option<HandlerId>,
    on_change_text: Option<HandlerId>,
    on_submit_editing: Option<HandlerId>,
    on_focus: Option<HandlerId>,
    on_blur: Option<HandlerId>,
}

impl InputEventBindings {
    fn from_node(node: &RetainedNode) -> Self {
        Self {
            on_change: event_handler(node, "onChange"),
            on_change_text: event_handler(node, "onChangeText"),
            on_submit_editing: event_handler(node, "onSubmitEditing"),
            on_focus: event_handler(node, "onFocus"),
            on_blur: event_handler(node, "onBlur"),
        }
    }

    fn dispatch_change(
        &self,
        value: &str,
        event_count: u64,
        dispatch_event: &BridgeEventDispatch,
    ) {
        let payload = NodeValue::Object(
            [
                ("value".to_owned(), NodeValue::String(value.to_owned())),
                (
                    "eventCount".to_owned(),
                    NodeValue::Number(event_count as f64),
                ),
            ]
            .into(),
        );
        if let Some(handler_id) = self.on_change {
            dispatch_event(handler_id, payload.clone());
        }
        if let Some(handler_id) = self.on_change_text {
            dispatch_event(handler_id, payload);
        }
    }

    fn dispatch_submit(&self, value: &str, dispatch_event: &BridgeEventDispatch) {
        if let Some(handler_id) = self.on_submit_editing {
            dispatch_event(handler_id, NodeValue::String(value.to_owned()));
        }
    }

    fn dispatch_string(
        &self,
        property: &str,
        value: &str,
        dispatch_event: &BridgeEventDispatch,
    ) {
        let handler_id = match property {
            "onFocus" => self.on_focus,
            "onBlur" => self.on_blur,
            _ => None,
        };
        if let Some(handler_id) = handler_id {
            dispatch_event(handler_id, NodeValue::String(value.to_owned()));
        }
    }
}

#[derive(Debug, Default)]
struct TextControlSyncState {
    native_event_count: u64,
    last_acked_event_count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TextControlSyncDecision {
    SkipStale,
    Ack,
    Apply,
}

pub(super) fn decide_text_control_sync(
    prop_event_count: u64,
    native_event_count: u64,
    prop_value: &str,
    native_value: &str,
) -> TextControlSyncDecision {
    if prop_event_count < native_event_count {
        return TextControlSyncDecision::SkipStale;
    }
    if prop_value == native_value {
        return TextControlSyncDecision::Ack;
    }
    TextControlSyncDecision::Apply
}

fn text_event_count_prop(node: &RetainedNode) -> u64 {
    number_prop(component_props(node), "__rasterTextEventCount")
        .unwrap_or(0.0)
        .max(0.0) as u64
}

fn disabled_prop(node: &RetainedNode) -> Option<bool> {
    let props = component_props(node);
    bool_prop(props, "disabled")
        .or_else(|| bool_prop(props, "readOnly"))
        .or_else(|| bool_prop(props, "editable").map(|editable| !editable))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TextControlConfig {
    multiline: bool,
    rows: usize,
}

impl TextControlConfig {
    fn from_node(node: &RetainedNode) -> Self {
        let multiline = is_multiline(node);
        let rows = if multiline {
            number_prop(component_props(node), "rows")
                .unwrap_or(2.0)
                .max(1.0) as usize
        } else {
            1
        };
        Self { multiline, rows }
    }
}

pub(in crate::gpui_backend) fn is_text_control_node(node: &RetainedNode) -> bool {
    matches!(
        node.kind,
        RetainedNodeKind::Input | RetainedNodeKind::Textarea
    ) && matches!(node.component_name(), "Input" | "Textarea")
}

fn is_multiline(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Textarea
        || bool_prop(component_props(node), "multiline") == Some(true)
}
