use std::{cell::RefCell, rc::Rc};

use gpui::{AnyElement, AppContext, Context, Entity, IntoElement, Subscription, Window};
use gpui_component::slider::{Slider, SliderEvent, SliderState, SliderValue};

use crate::{
    common::{
        channel::{ChannelSender, RuntimeCommand},
        ids::HandlerId,
        mount::{NodeValue, RetainedNodeKind},
    },
    gpui_backend::{
        components::helper::props::{
            bool_prop, component_props, event_handler, number_prop,
        },
        render_model::{model::RenderModel, style::apply_style},
        retained_tree::node::RetainedNode,
    },
};

pub(in crate::gpui_backend) struct RasterSliderState {
    slider: Entity<SliderState>,
    bindings: Rc<RefCell<SliderEventBindings>>,
    config: SliderConfig,
    controlled_value: Option<f32>,
    _subscription: Subscription,
}

impl RasterSliderState {
    pub(in crate::gpui_backend) fn new(
        node: &RetainedNode,
        runtime_commands: ChannelSender<RuntimeCommand>,
        cx: &mut Context<crate::gpui_backend::app::NodeOwnerView>,
    ) -> Self {
        let config = SliderConfig::from_node(node);
        let bindings = Rc::new(RefCell::new(SliderEventBindings::from_node(node)));
        let initial_value = initial_value(node, &config);

        let slider = cx.new(|_| {
            SliderState::new()
                .min(config.min)
                .max(config.max)
                .step(config.step)
                .default_value(SliderValue::Single(initial_value))
        });

        let _subscription = cx.subscribe(&slider, {
            let bindings = bindings.clone();
            let runtime_commands = runtime_commands.clone();
            move |_, _, event: &SliderEvent, _cx| {
                if let SliderEvent::Change(value) = event
                    && let SliderValue::Single(value) = value
                {
                    bindings
                        .borrow()
                        .dispatch_change(*value, &runtime_commands);
                }
            }
        });

        Self {
            slider,
            bindings,
            config,
            controlled_value: controlled_value(node),
            _subscription,
        }
    }

    pub(in crate::gpui_backend) fn matches_config(&self, node: &RetainedNode) -> bool {
        self.config == SliderConfig::from_node(node)
    }

    pub(in crate::gpui_backend) fn sync_from_node(
        &mut self,
        node: &RetainedNode,
        window: &mut Window,
        cx: &mut Context<crate::gpui_backend::app::NodeOwnerView>,
    ) {
        *self.bindings.borrow_mut() = SliderEventBindings::from_node(node);
        self.config = SliderConfig::from_node(node);

        let next_controlled_value = controlled_value(node);
        if let Some(value) = next_controlled_value {
            let clamped = value.clamp(self.config.min, self.config.max);
            self.slider.update(cx, |slider, cx| {
                if let SliderValue::Single(current) = slider.value()
                    && (current - clamped).abs() > f32::EPSILON
                {
                    slider.set_value(SliderValue::Single(clamped), window, cx);
                }
            });
        }
        self.controlled_value = next_controlled_value;
    }

    pub(in crate::gpui_backend) fn slider(&self) -> &Entity<SliderState> {
        &self.slider
    }
}

pub(in crate::gpui_backend) fn render_slider_from_node(
    node: &RetainedNode,
    state: &Entity<SliderState>,
) -> Option<AnyElement> {
    if !is_slider_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let props = component_props(node);
    let mut slider = Slider::new(state);
    if let Some(disabled) = bool_prop(props, "disabled") {
        slider = slider.disabled(disabled);
    }

    Some(apply_style(slider, &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn is_slider_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Slider"
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SliderConfig {
    min: f32,
    max: f32,
    step: f32,
}

impl SliderConfig {
    fn from_node(node: &RetainedNode) -> Self {
        let props = component_props(node);
        Self {
            min: number_prop(props, "min").unwrap_or(0.0) as f32,
            max: number_prop(props, "max").unwrap_or(100.0) as f32,
            step: number_prop(props, "step").unwrap_or(1.0) as f32,
        }
    }
}

fn controlled_value(node: &RetainedNode) -> Option<f32> {
    number_prop(component_props(node), "value").map(|value| value as f32)
}

fn initial_value(node: &RetainedNode, config: &SliderConfig) -> f32 {
    controlled_value(node)
        .or_else(|| number_prop(component_props(node), "defaultValue").map(|value| value as f32))
        .unwrap_or(config.min)
        .clamp(config.min, config.max)
}

#[derive(Clone, Copy, Debug, Default)]
struct SliderEventBindings {
    on_change: Option<HandlerId>,
}

impl SliderEventBindings {
    fn from_node(node: &RetainedNode) -> Self {
        Self {
            on_change: event_handler(node, "onChange"),
        }
    }

    fn dispatch_change(&self, value: f32, runtime_commands: &ChannelSender<RuntimeCommand>) {
        if let Some(handler_id) = self.on_change {
            let _ = runtime_commands.send(RuntimeCommand::InvokeEvent {
                handler_id,
                payload: NodeValue::Object(
                    [("value".to_owned(), NodeValue::Number(value as f64))].into(),
                ),
            });
        }
    }
}