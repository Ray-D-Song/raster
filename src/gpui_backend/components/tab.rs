use std::{cell::RefCell, rc::Rc};

use gpui::{AnyElement, IntoElement, ParentElement, Styled};
use gpui_component::{
    Selectable, Sizable, Size,
    tab::{Tab, TabBar, TabVariant},
};

use crate::{
    bridge::{SharedBridgeState, emit_handler_invoke},
    common::{
        ids::{HandlerId, NativeObjectId},
        mount::{NodeValue, RetainedNodeKind},
        utils::logger,
    },
    gpui_backend::{
        app::{OwnerRegistry, RasterRootView, render_node_child},
        components::{
            helper::props::{
                bool_prop, component_props, event_handler, number_prop, prop_or_child_text,
                string_prop,
            },
            icon::icon_from_svg,
        },
        perf::PerfMonitor,
        render_model::{model::RenderModel, style::apply_style},
        retained_tree::{node::RetainedNode, tree::RetainedTree},
    },
};

pub(in crate::gpui_backend) fn render_tab_bar_from_node(
    node: &RetainedNode,
    tree: &Rc<RefCell<RetainedTree>>,
    owners: &Rc<RefCell<OwnerRegistry>>,
    perf: &Rc<RefCell<PerfMonitor>>,
    bridge: SharedBridgeState,
    root: gpui::WeakEntity<RasterRootView>,
) -> Option<AnyElement> {
    if !is_tab_bar_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let props = component_props(node);
    let mut tab_bar = TabBar::new(("raster-tab-bar", node.id.0));
    if let Some(variant) = string_prop(props, "variant").and_then(parse_tab_variant) {
        tab_bar = tab_bar.with_variant(variant);
    }
    if let Some(size) = string_prop(props, "size").map(|value| Size::from_str(&value)) {
        tab_bar = tab_bar.with_size(size);
    }
    if let Some(selected_index) = number_prop(props, "selectedIndex") {
        tab_bar = tab_bar.selected_index(selected_index.max(0.0) as usize);
    }
    if let Some(menu) = bool_prop(props, "menu") {
        tab_bar = tab_bar.menu(menu);
    }

    let mut tab_clicks = Vec::new();
    let mut prefix_slot = None;
    let mut suffix_slot = None;
    let tree_ref = tree.borrow();
    for (index, child_id) in node.children.iter().copied().enumerate() {
        let Some(child) = tree_ref.node(child_id) else {
            continue;
        };
        if is_tab_node(child) {
            tab_clicks.push(event_handler(child, "onClick"));
            tab_bar = tab_bar.child(build_tab(child, &tree_ref, index));
        } else if is_slot_node(child) {
            match slot_name(child).as_deref() {
                Some("prefix") => prefix_slot = Some(child.children.clone()),
                Some("suffix") => suffix_slot = Some(child.children.clone()),
                Some(name) => logger::warn(format!("TabBar ignored unsupported Slot `{name}`")),
                None => logger::warn("TabBar ignored Slot without name"),
            }
        } else {
            logger::warn(format!(
                "TabBar only supports direct Tab or Slot children; ignored {}",
                child.component_name()
            ));
        }
    }
    drop(tree_ref);

    if let Some(children) = prefix_slot {
        tab_bar = tab_bar.prefix(render_slot_children(
            children,
            tree,
            owners,
            perf,
            bridge.clone(),
            root.clone(),
        ));
    }
    if let Some(children) = suffix_slot {
        tab_bar = tab_bar.suffix(render_slot_children(
            children,
            tree,
            owners,
            perf,
            bridge.clone(),
            root.clone(),
        ));
    }

    let on_click = event_handler(node, "onClick");
    if on_click.is_some() || tab_clicks.iter().any(Option::is_some) {
        tab_bar = tab_bar.on_click(move |index, _window, _cx| {
            if let Some(Some(handler_id)) = tab_clicks.get(*index) {
                send_event(
                    &bridge,
                    *handler_id,
                    NodeValue::String(index.to_string()),
                    "Tab onClick",
                );
            }
            if let Some(handler_id) = on_click {
                send_event(
                    &bridge,
                    handler_id,
                    NodeValue::String(index.to_string()),
                    "TabBar onClick",
                );
            }
        });
    }

    Some(apply_style(tab_bar, &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn render_tab_from_node(
    node: &RetainedNode,
    child_text: impl IntoIterator<Item = impl Into<String>>,
    bridge: SharedBridgeState,
) -> Option<AnyElement> {
    if !is_tab_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let mut tab = build_tab_from_text(node, child_text, 0);
    if let Some(handler_id) = event_handler(node, "onClick") {
        tab = tab.on_click(move |_, _, _| {
            send_event(
                &bridge,
                handler_id,
                NodeValue::String(String::new()),
                "Tab onClick",
            );
        });
    }

    Some(apply_style(tab, &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn is_tab_bar_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "TabBar"
}

pub(in crate::gpui_backend) fn is_tab_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Tab"
}

fn is_slot_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Slot"
}

fn build_tab(node: &RetainedNode, tree: &RetainedTree, fallback_index: usize) -> Tab {
    let child_text = node
        .children
        .iter()
        .filter_map(|child_id| tree.node(*child_id))
        .filter_map(|child| child.payload.text.as_deref());
    build_tab_from_text(node, child_text, fallback_index)
}

fn build_tab_from_text(
    node: &RetainedNode,
    child_text: impl IntoIterator<Item = impl Into<String>>,
    fallback_index: usize,
) -> Tab {
    let props = component_props(node);
    let mut tab = Tab::new();
    let child_text = child_text
        .into_iter()
        .map(Into::into)
        .collect::<Vec<String>>();
    if let Some(label) = prop_or_child_text(props, "label", child_text.iter().map(String::as_str)) {
        tab = tab.label(label);
    } else {
        tab = tab.label(fallback_index.to_string());
    }
    if let Some(icon) = string_prop(props, "iconSvg").map(|svg| icon_from_svg(&svg)) {
        tab = tab.icon(icon);
    }
    if let Some(variant) = string_prop(props, "variant").and_then(parse_tab_variant) {
        tab = tab.with_variant(variant);
    }
    if let Some(size) = string_prop(props, "size").map(|value| Size::from_str(&value)) {
        tab = tab.with_size(size);
    }
    if let Some(disabled) = bool_prop(props, "disabled") {
        tab = tab.disabled(disabled);
    }
    if bool_prop(props, "selected") == Some(true) {
        tab = tab.selected(true);
    }
    tab
}

fn render_slot_children(
    children: Vec<NativeObjectId>,
    tree: &Rc<RefCell<RetainedTree>>,
    owners: &Rc<RefCell<OwnerRegistry>>,
    perf: &Rc<RefCell<PerfMonitor>>,
    bridge: SharedBridgeState,
    root: gpui::WeakEntity<RasterRootView>,
) -> AnyElement {
    gpui::div()
        .flex()
        .items_center()
        .children(children.into_iter().map(|child_id| {
            render_node_child(
                child_id,
                tree,
                owners,
                perf,
                bridge.clone(),
                root.clone(),
            )
        }))
        .into_any_element()
}

fn slot_name(node: &RetainedNode) -> Option<String> {
    string_prop(component_props(node), "name")
}

fn parse_tab_variant(value: String) -> Option<TabVariant> {
    Some(match value.as_str() {
        "tab" => TabVariant::Tab,
        "outline" => TabVariant::Outline,
        "pill" => TabVariant::Pill,
        "segmented" => TabVariant::Segmented,
        "underline" => TabVariant::Underline,
        _ => return None,
    })
}

fn send_event(
    bridge: &SharedBridgeState,
    handler_id: HandlerId,
    payload: NodeValue,
    _label: &str,
) {
    emit_handler_invoke(bridge, handler_id, payload);
}
