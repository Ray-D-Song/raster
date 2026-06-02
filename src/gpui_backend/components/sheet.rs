use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use gpui::{App, ParentElement, Window, px};
use gpui_component::{Placement, WindowExt};

use crate::{
    common::{
        channel::{ChannelSender, RuntimeCommand},
        ids::HandlerId,
        mount::{NodeValue, RetainedNodeKind},
        utils::logger,
    },
    gpui_backend::{
        app::{OwnerRegistry, RasterRootView, render_node_child},
        components::helper::props::{
            bool_prop, component_props, event_handler, number_prop, string_prop,
        },
        retained_tree::{node::RetainedNode, tree::RetainedTree},
    },
};

pub(in crate::gpui_backend) struct RasterSheetState {
    local_node_id: Option<crate::common::ids::NativeObjectId>,
    config: SheetConfig,
    local_open: bool,
    suppressed_until_controlled_close: Rc<RefCell<bool>>,
}

pub(in crate::gpui_backend) struct SheetRenderContext {
    pub(in crate::gpui_backend) tree: Rc<RefCell<RetainedTree>>,
    pub(in crate::gpui_backend) owners: Rc<RefCell<OwnerRegistry>>,
    pub(in crate::gpui_backend) perf: Rc<RefCell<crate::gpui_backend::perf::PerfMonitor>>,
    pub(in crate::gpui_backend) runtime_commands: ChannelSender<RuntimeCommand>,
    pub(in crate::gpui_backend) root: gpui::WeakEntity<RasterRootView>,
}

impl RasterSheetState {
    pub(in crate::gpui_backend) fn new() -> Self {
        Self {
            local_node_id: None,
            config: SheetConfig::default(),
            local_open: false,
            suppressed_until_controlled_close: Rc::new(RefCell::new(false)),
        }
    }

    pub(in crate::gpui_backend) fn sync_closed(&mut self, window: &mut Window, cx: &mut App) {
        if self.local_open {
            window.close_sheet(cx);
        }
        self.local_node_id = None;
        self.local_open = false;
        *self.suppressed_until_controlled_close.borrow_mut() = false;
        self.config = SheetConfig::default();
    }

    pub(in crate::gpui_backend) fn sync_from_node(
        &mut self,
        node: &RetainedNode,
        active: bool,
        render_context: SheetRenderContext,
        window: &mut Window,
        cx: &mut App,
    ) {
        let controlled_open = sheet_open(node);
        if !controlled_open {
            if self.local_open {
                window.close_sheet(cx);
            }
            self.local_node_id = None;
            self.local_open = false;
            *self.suppressed_until_controlled_close.borrow_mut() = false;
            self.config = SheetConfig::from_node(node);
            return;
        }

        if !active {
            if self.local_open {
                window.close_sheet(cx);
                self.local_node_id = None;
                self.local_open = false;
            }
            logger::warn("multiple open Sheet nodes found; only the first one is rendered");
            return;
        }

        if *self.suppressed_until_controlled_close.borrow() {
            return;
        }

        let next_config = SheetConfig::from_node(node);
        if self.local_open && self.local_node_id == Some(node.id) && self.config == next_config {
            return;
        }

        if self.local_open {
            window.close_sheet(cx);
        }
        open_sheet(
            node,
            next_config.clone(),
            render_context,
            self.suppressed_until_controlled_close.clone(),
            window,
            cx,
        );
        self.local_node_id = Some(node.id);
        self.config = next_config;
        self.local_open = true;
    }
}

pub(in crate::gpui_backend) fn is_sheet_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Sheet"
}

pub(in crate::gpui_backend) fn sheet_open(node: &RetainedNode) -> bool {
    bool_prop(component_props(node), "open") == Some(true)
}

#[derive(Debug, Clone, PartialEq)]
struct SheetConfig {
    title: Option<String>,
    placement: Placement,
    size: f64,
    overlay: bool,
    overlay_closable: bool,
    resizable: bool,
    on_open_change: Option<HandlerId>,
}

impl Default for SheetConfig {
    fn default() -> Self {
        Self {
            title: None,
            placement: Placement::Right,
            size: 350.0,
            overlay: true,
            overlay_closable: true,
            resizable: true,
            on_open_change: None,
        }
    }
}

impl SheetConfig {
    fn from_node(node: &RetainedNode) -> Self {
        let props = component_props(node);
        Self {
            title: string_prop(props, "title"),
            placement: placement_prop(props).unwrap_or(Placement::Right),
            size: number_prop(props, "size").unwrap_or(350.0),
            overlay: bool_prop(props, "overlay").unwrap_or(true),
            overlay_closable: bool_prop(props, "overlayClosable").unwrap_or(true),
            resizable: bool_prop(props, "resizable").unwrap_or(true),
            on_open_change: event_handler(node, "onOpenChange"),
        }
    }
}

fn open_sheet(
    node: &RetainedNode,
    config: SheetConfig,
    render_context: SheetRenderContext,
    suppressed: Rc<RefCell<bool>>,
    window: &mut Window,
    cx: &mut App,
) {
    let children = node.children.clone();
    let placement = config.placement;
    let runtime_commands = render_context.runtime_commands.clone();

    window.open_sheet_at(placement, cx, move |sheet, _window, _cx| {
        let mut sheet = sheet
            .size(px(config.size as f32))
            .overlay(config.overlay)
            .overlay_closable(config.overlay_closable)
            .resizable(config.resizable);

        if let Some(title) = config.title.clone() {
            sheet = sheet.title(title);
        }

        if let Some(handler_id) = config.on_open_change {
            let runtime_commands = runtime_commands.clone();
            let suppressed = suppressed.clone();
            sheet = sheet.on_close(move |_, _, _| {
                *suppressed.borrow_mut() = true;
                dispatch_open_change(handler_id, "cancel", &runtime_commands);
            });
        }

        for child in &children {
            sheet = sheet.child(render_node_child(
                *child,
                &render_context.tree,
                &render_context.owners,
                &render_context.perf,
                render_context.runtime_commands.clone(),
                render_context.root.clone(),
            ));
        }

        sheet
    });
}

fn dispatch_open_change(
    handler_id: HandlerId,
    reason: &str,
    runtime_commands: &ChannelSender<RuntimeCommand>,
) {
    let mut payload = BTreeMap::new();
    payload.insert("open".to_owned(), NodeValue::Bool(false));
    payload.insert("reason".to_owned(), NodeValue::String(reason.to_owned()));
    if runtime_commands
        .send(RuntimeCommand::InvokeEvent {
            handler_id,
            payload: NodeValue::Object(payload),
        })
        .is_err()
    {
        logger::error("failed to enqueue Sheet onOpenChange event");
    }
}

fn placement_prop(props: &BTreeMap<String, NodeValue>) -> Option<Placement> {
    match string_prop(props, "placement").as_deref() {
        Some("top") => Some(Placement::Top),
        Some("bottom") => Some(Placement::Bottom),
        Some("left") => Some(Placement::Left),
        Some("right") => Some(Placement::Right),
        _ => None,
    }
}
