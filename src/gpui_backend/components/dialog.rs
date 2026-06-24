use std::{cell::RefCell, rc::Rc};

use gpui::{App, ParentElement, Window, div, px};
use gpui_component::{
    WindowExt,
    button::{Button, ButtonVariants},
    dialog::{Dialog, DialogAction, DialogButtonProps, DialogClose, DialogFooter},
};

use crate::{
    bridge::SharedBridgeState,
    common::{
        ids::HandlerId,
        mount::RetainedNodeKind,
    },
    gpui_backend::{
        app::{OwnerRegistry, RasterRootView, render_node_child},
        components::helper::props::{
            bool_prop, component_props, event_handler, number_prop, string_prop,
        },
        components::internal::controlled_dialog::{
            ControlledDialogState, dispatch_open_change, dispatch_string_event,
        },
        retained_tree::{node::RetainedNode, tree::RetainedTree},
    },
};

pub(in crate::gpui_backend) struct RasterDialogState {
    state: ControlledDialogState<DialogConfig>,
}

pub(in crate::gpui_backend) struct DialogRenderContext {
    pub(in crate::gpui_backend) tree: Rc<RefCell<RetainedTree>>,
    pub(in crate::gpui_backend) owners: Rc<RefCell<OwnerRegistry>>,
    pub(in crate::gpui_backend) perf: Rc<RefCell<crate::gpui_backend::perf::PerfMonitor>>,
    pub(in crate::gpui_backend) bridge: SharedBridgeState,
    pub(in crate::gpui_backend) root: gpui::WeakEntity<RasterRootView>,
}

impl RasterDialogState {
    pub(in crate::gpui_backend) fn new() -> Self {
        Self {
            state: ControlledDialogState::new(),
        }
    }

    pub(in crate::gpui_backend) fn sync_closed(&mut self, window: &mut Window, cx: &mut App) {
        self.state
            .sync_closed(|window, cx| window.close_dialog(cx), window, cx);
    }

    pub(in crate::gpui_backend) fn sync_from_node(
        &mut self,
        node: &RetainedNode,
        active: bool,
        render_context: DialogRenderContext,
        window: &mut Window,
        cx: &mut App,
    ) {
        let next_config = DialogConfig::from_node(node);
        self.state.sync_from_node(
            node,
            active,
            dialog_open(node),
            next_config,
            "multiple open Dialog nodes found; only the first one is rendered",
            |window, cx| window.close_dialog(cx),
            |node, config, suppressed, window, cx| {
                open_dialog(node, config, render_context, suppressed, window, cx);
            },
            window,
            cx,
        );
    }
}

pub(in crate::gpui_backend) fn is_dialog_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Dialog"
}

pub(in crate::gpui_backend) fn dialog_open(node: &RetainedNode) -> bool {
    bool_prop(component_props(node), "open") == Some(true)
}

#[derive(Debug, Clone, PartialEq)]
struct DialogConfig {
    title: Option<String>,
    confirm: bool,
    ok_text: Option<String>,
    cancel_text: Option<String>,
    width: f64,
    max_width: Option<f64>,
    margin_top: Option<f64>,
    overlay: bool,
    overlay_closable: bool,
    keyboard: bool,
    close_button: bool,
    on_ok: Option<HandlerId>,
    on_cancel: Option<HandlerId>,
    on_open_change: Option<HandlerId>,
}

impl Default for DialogConfig {
    fn default() -> Self {
        Self {
            title: None,
            confirm: false,
            ok_text: None,
            cancel_text: None,
            width: 480.0,
            max_width: None,
            margin_top: None,
            overlay: true,
            overlay_closable: true,
            keyboard: true,
            close_button: true,
            on_ok: None,
            on_cancel: None,
            on_open_change: None,
        }
    }
}

impl DialogConfig {
    fn from_node(node: &RetainedNode) -> Self {
        let props = component_props(node);
        Self {
            title: string_prop(props, "title"),
            confirm: bool_prop(props, "confirm").unwrap_or(false),
            ok_text: string_prop(props, "okText"),
            cancel_text: string_prop(props, "cancelText"),
            width: number_prop(props, "width").unwrap_or(480.0),
            max_width: number_prop(props, "maxWidth"),
            margin_top: number_prop(props, "marginTop"),
            overlay: bool_prop(props, "overlay").unwrap_or(true),
            overlay_closable: bool_prop(props, "overlayClosable").unwrap_or(true),
            keyboard: bool_prop(props, "keyboard").unwrap_or(true),
            close_button: bool_prop(props, "closeButton").unwrap_or(true),
            on_ok: event_handler(node, "onOk"),
            on_cancel: event_handler(node, "onCancel"),
            on_open_change: event_handler(node, "onOpenChange"),
        }
    }
}

fn open_dialog(
    node: &RetainedNode,
    config: DialogConfig,
    render_context: DialogRenderContext,
    suppressed: Rc<RefCell<bool>>,
    window: &mut Window,
    cx: &mut App,
) {
    let children = node.children.clone();
    let bridge = render_context.bridge.clone();

    window.open_dialog(cx, move |dialog, _window, _cx| {
        let mut dialog = apply_config(dialog, &config);

        if config.confirm {
            if let Some(handler_id) = config.on_ok {
                let bridge = bridge.clone();
                let on_open_change = config.on_open_change;
                let suppressed = suppressed.clone();
                dialog = dialog.on_ok(move |_, _, _| {
                    *suppressed.borrow_mut() = true;
                    dispatch_string_event(handler_id, &bridge, "Dialog onOk");
                    if let Some(handler_id) = on_open_change {
                        dispatch_open_change(
                            handler_id,
                            "ok",
                            &bridge,
                            "Dialog onOpenChange",
                        );
                    }
                    true
                });
            } else if let Some(handler_id) = config.on_open_change {
                let bridge = bridge.clone();
                let suppressed = suppressed.clone();
                dialog = dialog.on_ok(move |_, _, _| {
                    *suppressed.borrow_mut() = true;
                    dispatch_open_change(
                        handler_id,
                        "ok",
                        &bridge,
                        "Dialog onOpenChange",
                    );
                    true
                });
            }
        }

        if config.on_cancel.is_some() || config.on_open_change.is_some() {
            let bridge = bridge.clone();
            let on_cancel = config.on_cancel;
            let on_open_change = config.on_open_change;
            let suppressed = suppressed.clone();
            dialog = dialog.on_cancel(move |_, _, _| {
                *suppressed.borrow_mut() = true;
                if let Some(handler_id) = on_cancel {
                    dispatch_string_event(handler_id, &bridge, "Dialog onCancel");
                }
                if let Some(handler_id) = on_open_change {
                    dispatch_open_change(
                        handler_id,
                        "cancel",
                        &bridge,
                        "Dialog onOpenChange",
                    );
                }
                true
            });
        }

        for child in &children {
            dialog = dialog.child(render_node_child(
                *child,
                &render_context.tree,
                &render_context.owners,
                &render_context.perf,
                render_context.bridge.clone(),
                render_context.root.clone(),
            ));
        }

        dialog
    });
}

fn apply_config(mut dialog: Dialog, config: &DialogConfig) -> Dialog {
    if config.confirm {
        dialog = dialog
            .footer(
                DialogFooter::new()
                    .child(
                        DialogClose::new().child(
                            Button::new("cancel")
                                .label(config.cancel_text.clone().unwrap_or("Cancel".to_owned())),
                        ),
                    )
                    .child(
                        DialogAction::new().child(
                            Button::new("ok")
                                .primary()
                                .label(config.ok_text.clone().unwrap_or("OK".to_owned())),
                        ),
                    ),
            )
            .overlay_closable(false)
            .close_button(false);
    }
    if let Some(title) = config.title.clone() {
        dialog = dialog.title(div().child(title));
    }
    dialog = dialog
        .width(px(config.width as f32))
        .overlay(config.overlay)
        .overlay_closable(config.overlay_closable)
        .keyboard(config.keyboard)
        .close_button(config.close_button);
    if let Some(max_width) = config.max_width {
        dialog = dialog.max_w(px(max_width as f32));
    }
    if let Some(margin_top) = config.margin_top {
        dialog = dialog.margin_top(px(margin_top as f32));
    }
    if config.ok_text.is_some() || config.cancel_text.is_some() {
        let mut button_props = DialogButtonProps::default();
        if let Some(ok_text) = config.ok_text.clone() {
            button_props = button_props.ok_text(ok_text);
        }
        if let Some(cancel_text) = config.cancel_text.clone() {
            button_props = button_props.cancel_text(cancel_text);
        }
        dialog = dialog.button_props(button_props);
    }
    dialog
}
