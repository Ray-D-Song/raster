use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use gpui::{
    AnyElement, App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    SharedString, Styled, Subscription, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    IndexPath, Sizable, Size,
    select::{Select, SelectDelegate, SelectEvent, SelectItem, SelectState},
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
            bool_prop, component_props, display_value, event_handler, string_prop,
        },
        render_model::{model::RenderModel, style::apply_style},
        retained_tree::node::RetainedNode,
    },
};

pub(in crate::gpui_backend) struct RasterSelectState {
    select: Entity<SelectState<RasterSelectDelegate>>,
    model: Rc<RefCell<RasterSelectModel>>,
    bindings: Rc<RefCell<SelectEventBindings>>,
    searchable: bool,
    controlled_value: Option<Option<NodeValue>>,
    runtime_commands: ChannelSender<RuntimeCommand>,
    _subscription: Subscription,
}

impl RasterSelectState {
    pub(in crate::gpui_backend) fn new(
        node: &RetainedNode,
        runtime_commands: ChannelSender<RuntimeCommand>,
        window: &mut Window,
        cx: &mut Context<crate::gpui_backend::app::NodeOwnerView>,
    ) -> Self {
        let model = Rc::new(RefCell::new(RasterSelectModel::from_node(node)));
        let bindings = Rc::new(RefCell::new(SelectEventBindings::from_node(node)));
        let searchable = bool_prop(component_props(node), "searchable") == Some(true);
        let controlled_value = controlled_value(node);
        let selected_index = controlled_value
            .clone()
            .and_then(|value| value.and_then(|value| model.borrow().position(&value)));

        let delegate = RasterSelectDelegate::new(
            model.borrow().sections.clone(),
            bindings.borrow().on_search_change,
            runtime_commands.clone(),
        );
        let select = cx.new(|cx| {
            SelectState::new(delegate, selected_index, window, cx).searchable(searchable)
        });

        let _subscription = cx.subscribe(&select, {
            let model = model.clone();
            let bindings = bindings.clone();
            let runtime_commands = runtime_commands.clone();
            move |_, _, event: &SelectEvent<RasterSelectDelegate>, _cx| match event {
                SelectEvent::Confirm(value) => {
                    let index = value
                        .as_ref()
                        .and_then(|value| model.borrow().position(value));
                    bindings.borrow().dispatch_change(
                        value.as_ref(),
                        index,
                        &model.borrow(),
                        &runtime_commands,
                    );
                }
            }
        });

        Self {
            select,
            model,
            bindings,
            searchable,
            controlled_value,
            runtime_commands,
            _subscription,
        }
    }

    pub(in crate::gpui_backend) fn matches_config(&self, node: &RetainedNode) -> bool {
        self.searchable == (bool_prop(component_props(node), "searchable") == Some(true))
    }

    pub(in crate::gpui_backend) fn sync_from_node(
        &mut self,
        node: &RetainedNode,
        window: &mut Window,
        cx: &mut Context<crate::gpui_backend::app::NodeOwnerView>,
    ) {
        let bindings = SelectEventBindings::from_node(node);
        *self.bindings.borrow_mut() = bindings;

        let next_model = RasterSelectModel::from_node(node);
        let model_changed = *self.model.borrow() != next_model;
        if model_changed {
            *self.model.borrow_mut() = next_model.clone();
            let delegate = RasterSelectDelegate::new(
                next_model.sections,
                bindings.on_search_change,
                self.runtime_commands.clone(),
            );
            self.select.update(cx, |select, cx| {
                select.set_items(delegate, window, cx);
            });
        }

        let next_controlled_value = controlled_value(node);
        if (model_changed || next_controlled_value != self.controlled_value)
            && let Some(value) = next_controlled_value.clone()
        {
            self.select.update(cx, |select, cx| match value {
                Some(value) => select.set_selected_value(&value, window, cx),
                None => select.set_selected_index(None, window, cx),
            });
        }
        self.controlled_value = next_controlled_value;
    }

    pub(in crate::gpui_backend) fn select(&self) -> &Entity<SelectState<RasterSelectDelegate>> {
        &self.select
    }
}

pub(in crate::gpui_backend) fn render_select_from_node(
    node: &RetainedNode,
    state: &Entity<SelectState<RasterSelectDelegate>>,
) -> Option<AnyElement> {
    if !is_select_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let mut select = Select::new(state);
    let props = component_props(node);
    if let Some(placeholder) = string_prop(props, "placeholder") {
        select = select.placeholder(placeholder);
    }
    if let Some(size) = string_prop(props, "size") {
        select = select.with_size(Size::from_str(&size));
    }
    if let Some(cleanable) = bool_prop(props, "cleanable") {
        select = select.cleanable(cleanable);
    }
    if let Some(disabled) = bool_prop(props, "disabled") {
        select = select.disabled(disabled);
    }
    if let Some(appearance) = bool_prop(props, "appearance") {
        select = select.appearance(appearance);
    }

    let wrapper = apply_style(
        div()
            .id(("raster-select-wrapper", node.id.0))
            .flex()
            .flex_col()
            .when(model.style.height.is_none(), |this| this.h(px(36.0))),
        &model.style,
    )
    .child(select);

    Some(wrapper.into_any_element())
}

pub(in crate::gpui_backend) fn is_select_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Select"
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RasterSelectModel {
    pub(super) sections: Vec<RasterSelectSection>,
}

impl RasterSelectModel {
    pub(super) fn from_node(node: &RetainedNode) -> Self {
        let props = component_props(node);
        if props.contains_key("options") && props.contains_key("sections") {
            logger::warn("Select received both options and sections; sections takes precedence");
        }

        let sections = props
            .get("sections")
            .and_then(parse_sections)
            .or_else(|| props.get("options").and_then(parse_options))
            .unwrap_or_default();

        Self { sections }
    }

    pub(super) fn position(&self, value: &NodeValue) -> Option<IndexPath> {
        self.sections
            .iter()
            .enumerate()
            .find_map(|(section, group)| {
                group.items.iter().enumerate().find_map(|(row, item)| {
                    (item.value == *value).then_some(IndexPath::default().section(section).row(row))
                })
            })
    }

    pub(super) fn item(&self, index: IndexPath) -> Option<&RasterSelectItem> {
        self.sections
            .get(index.section)
            .and_then(|section| section.items.get(index.row))
    }

    fn first_item_with_value(&self, value: &NodeValue) -> Option<&RasterSelectItem> {
        self.sections
            .iter()
            .flat_map(|section| section.items.iter())
            .find(|item| item.value == *value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RasterSelectSection {
    pub(super) label: Option<String>,
    pub(super) items: Vec<RasterSelectItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RasterSelectItem {
    pub(super) id: Option<NodeValue>,
    pub(super) label: Option<NodeValue>,
    pub(super) title: String,
    pub(super) description: Option<String>,
    pub(super) value: NodeValue,
    pub(super) disabled: bool,
}

impl SelectItem for RasterSelectItem {
    type Value = NodeValue;

    fn title(&self) -> SharedString {
        SharedString::from(self.title.clone())
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .child(self.title.clone())
            .when_some(self.description.clone(), |this, description| {
                this.child(div().text_sm().opacity(0.65).child(description))
            })
            .into_any_element()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }

    fn disabled(&self) -> bool {
        self.disabled
    }

    fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.title.to_lowercase().contains(&query)
            || self
                .description
                .as_ref()
                .is_some_and(|description| description.to_lowercase().contains(&query))
    }
}

#[derive(Clone)]
pub struct RasterSelectDelegate {
    sections: Vec<RasterSelectSection>,
    on_search_change: Option<HandlerId>,
    runtime_commands: ChannelSender<RuntimeCommand>,
}

impl RasterSelectDelegate {
    fn new(
        sections: Vec<RasterSelectSection>,
        on_search_change: Option<HandlerId>,
        runtime_commands: ChannelSender<RuntimeCommand>,
    ) -> Self {
        Self {
            sections,
            on_search_change,
            runtime_commands,
        }
    }
}

impl SelectDelegate for RasterSelectDelegate {
    type Item = RasterSelectItem;

    fn sections_count(&self, _: &App) -> usize {
        self.sections.len().max(1)
    }

    fn section(&self, section: usize) -> Option<AnyElement> {
        self.sections
            .get(section)
            .and_then(|section| section.label.clone())
            .map(IntoElement::into_any_element)
    }

    fn items_count(&self, section: usize) -> usize {
        self.sections
            .get(section)
            .map(|section| section.items.len())
            .unwrap_or(0)
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        self.sections
            .get(ix.section)
            .and_then(|section| section.items.get(ix.row))
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SelectItem<Value = V>,
        V: PartialEq,
    {
        self.sections
            .iter()
            .enumerate()
            .find_map(|(section, group)| {
                group.items.iter().enumerate().find_map(|(row, item)| {
                    (item.value() == value)
                        .then_some(IndexPath::default().section(section).row(row))
                })
            })
    }

    fn perform_search(&mut self, query: &str, _window: &mut Window, _: &mut App) -> gpui::Task<()> {
        if let Some(handler_id) = self.on_search_change {
            if self
                .runtime_commands
                .send(RuntimeCommand::InvokeEvent {
                    handler_id,
                    payload: NodeValue::String(query.to_owned()),
                })
                .is_err()
            {
                logger::error("failed to enqueue Select onSearchChange event");
            }
        }
        gpui::Task::ready(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct SelectEventBindings {
    on_change: Option<HandlerId>,
    on_open_change: Option<HandlerId>,
    on_search_change: Option<HandlerId>,
}

impl SelectEventBindings {
    fn from_node(node: &RetainedNode) -> Self {
        Self {
            on_change: event_handler(node, "onChange"),
            on_open_change: event_handler(node, "onOpenChange"),
            on_search_change: event_handler(node, "onSearchChange"),
        }
    }

    fn dispatch_change(
        &self,
        value: Option<&NodeValue>,
        index: Option<IndexPath>,
        model: &RasterSelectModel,
        runtime_commands: &ChannelSender<RuntimeCommand>,
    ) {
        let Some(handler_id) = self.on_change else {
            return;
        };

        let mut payload = BTreeMap::new();
        match value {
            Some(value) => {
                payload.insert("value".to_owned(), value.clone());
                let item = index
                    .and_then(|index| model.item(index))
                    .or_else(|| model.first_item_with_value(value));
                if let Some(item) = item {
                    if let Some(id) = &item.id {
                        payload.insert("id".to_owned(), id.clone());
                    }
                    if let Some(label) = &item.label {
                        payload.insert("label".to_owned(), label.clone());
                    }
                }
            }
            None => {
                payload.insert("value".to_owned(), NodeValue::Null);
            }
        }

        if runtime_commands
            .send(RuntimeCommand::InvokeEvent {
                handler_id,
                payload: NodeValue::Object(payload),
            })
            .is_err()
        {
            logger::error("failed to enqueue Select onChange event");
        }
    }

    fn dispatch_open_change(
        &self,
        open: bool,
        reason: &str,
        runtime_commands: &ChannelSender<RuntimeCommand>,
    ) {
        let Some(handler_id) = self.on_open_change else {
            return;
        };

        let payload = NodeValue::Object(
            [
                ("open".to_owned(), NodeValue::Bool(open)),
                ("reason".to_owned(), NodeValue::String(reason.to_owned())),
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
            logger::error("failed to enqueue Select onOpenChange event");
        }
    }
}

fn parse_options(value: &NodeValue) -> Option<Vec<RasterSelectSection>> {
    let NodeValue::Array(items) = value else {
        return None;
    };
    Some(vec![RasterSelectSection {
        label: None,
        items: items.iter().filter_map(parse_item).collect(),
    }])
}

fn parse_sections(value: &NodeValue) -> Option<Vec<RasterSelectSection>> {
    let NodeValue::Array(sections) = value else {
        return None;
    };
    Some(
        sections
            .iter()
            .filter_map(|section| {
                let NodeValue::Object(section) = section else {
                    return None;
                };
                let items = section.get("items")?;
                let NodeValue::Array(items) = items else {
                    return None;
                };
                Some(RasterSelectSection {
                    label: section.get("label").map(display_value),
                    items: items.iter().filter_map(parse_item).collect(),
                })
            })
            .collect(),
    )
}

fn parse_item(value: &NodeValue) -> Option<RasterSelectItem> {
    let NodeValue::Object(item) = value else {
        return None;
    };

    let id = item.get("id").cloned();
    let label = item.get("label").cloned();
    let value = item
        .get("value")
        .cloned()
        .or_else(|| id.clone())
        .or_else(|| label.clone())
        .unwrap_or(NodeValue::Null);
    let title = label
        .as_ref()
        .map(display_value)
        .or_else(|| item.get("value").map(display_value))
        .or_else(|| id.as_ref().map(display_value))
        .unwrap_or_default();

    Some(RasterSelectItem {
        id,
        label,
        title,
        description: item.get("description").map(display_value),
        value,
        disabled: item_disabled(item),
    })
}

fn controlled_value(node: &RetainedNode) -> Option<Option<NodeValue>> {
    component_props(node).get("value").map(|value| match value {
        NodeValue::Null => None,
        value => Some(value.clone()),
    })
}

fn item_disabled(item: &std::collections::BTreeMap<String, NodeValue>) -> bool {
    match item.get("disabled") {
        Some(NodeValue::Bool(value)) => *value,
        _ => false,
    }
}
