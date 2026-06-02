use std::{cell::RefCell, rc::Rc};

use gpui::{App, ParentElement, Window, div, px};
use gpui_component::{
    Icon, WindowExt,
    dialog::{AlertDialog, DialogButtonProps},
};

use crate::{
    common::{
        channel::{ChannelSender, RuntimeCommand},
        ids::HandlerId,
        mount::RetainedNodeKind,
    },
    gpui_backend::{
        app::{OwnerRegistry, RasterRootView, render_node_child},
        components::{
            button::parse_button_variant,
            helper::props::{bool_prop, component_props, event_handler, number_prop, string_prop},
            icon::parse_icon_name,
            internal::controlled_dialog::{
                ControlledDialogState, dispatch_open_change, dispatch_string_event,
            },
        },
        retained_tree::{node::RetainedNode, tree::RetainedTree},
    },
};

pub(in crate::gpui_backend) struct RasterAlertState {
    state: ControlledDialogState<AlertConfig>,
}

pub(in crate::gpui_backend) struct AlertRenderContext {
    pub(in crate::gpui_backend) tree: Rc<RefCell<RetainedTree>>,
    pub(in crate::gpui_backend) owners: Rc<RefCell<OwnerRegistry>>,
    pub(in crate::gpui_backend) perf: Rc<RefCell<crate::gpui_backend::perf::PerfMonitor>>,
    pub(in crate::gpui_backend) runtime_commands: ChannelSender<RuntimeCommand>,
    pub(in crate::gpui_backend) root: gpui::WeakEntity<RasterRootView>,
}

impl RasterAlertState {
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
        render_context: AlertRenderContext,
        window: &mut Window,
        cx: &mut App,
    ) {
        let next_config = AlertConfig::from_node(node);
        self.state.sync_from_node(
            node,
            active,
            alert_open(node),
            next_config,
            "multiple open Alert nodes found; only the first one is rendered",
            |window, cx| window.close_dialog(cx),
            |node, config, suppressed, window, cx| {
                open_alert(node, config, render_context, suppressed, window, cx);
            },
            window,
            cx,
        );
    }
}

pub(in crate::gpui_backend) fn is_alert_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Alert"
}

pub(in crate::gpui_backend) fn alert_open(node: &RetainedNode) -> bool {
    bool_prop(component_props(node), "open") == Some(true)
}

#[derive(Debug, Clone, PartialEq)]
struct AlertConfig {
    title: Option<String>,
    description: Option<String>,
    icon: Option<String>,
    show_cancel: bool,
    ok_text: Option<String>,
    cancel_text: Option<String>,
    ok_variant: Option<String>,
    cancel_variant: Option<String>,
    width: f64,
    overlay_closable: bool,
    keyboard: bool,
    close_button: bool,
    on_ok: Option<HandlerId>,
    on_cancel: Option<HandlerId>,
    on_close: Option<HandlerId>,
    on_open_change: Option<HandlerId>,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            icon: None,
            show_cancel: false,
            ok_text: None,
            cancel_text: None,
            ok_variant: None,
            cancel_variant: None,
            width: 420.0,
            overlay_closable: false,
            keyboard: true,
            close_button: false,
            on_ok: None,
            on_cancel: None,
            on_close: None,
            on_open_change: None,
        }
    }
}

impl AlertConfig {
    fn from_node(node: &RetainedNode) -> Self {
        let props = component_props(node);
        Self {
            title: string_prop(props, "title"),
            description: string_prop(props, "description"),
            icon: string_prop(props, "icon"),
            show_cancel: bool_prop(props, "showCancel").unwrap_or(false),
            ok_text: string_prop(props, "okText"),
            cancel_text: string_prop(props, "cancelText"),
            ok_variant: string_prop(props, "okVariant"),
            cancel_variant: string_prop(props, "cancelVariant"),
            width: number_prop(props, "width").unwrap_or(420.0),
            overlay_closable: bool_prop(props, "overlayClosable").unwrap_or(false),
            keyboard: bool_prop(props, "keyboard").unwrap_or(true),
            close_button: bool_prop(props, "closeButton").unwrap_or(false),
            on_ok: event_handler(node, "onOk"),
            on_cancel: event_handler(node, "onCancel"),
            on_close: event_handler(node, "onClose"),
            on_open_change: event_handler(node, "onOpenChange"),
        }
    }
}

fn open_alert(
    node: &RetainedNode,
    config: AlertConfig,
    render_context: AlertRenderContext,
    suppressed: Rc<RefCell<bool>>,
    window: &mut Window,
    cx: &mut App,
) {
    let children = node.children.clone();
    let runtime_commands = render_context.runtime_commands.clone();

    window.open_alert_dialog(cx, move |alert, _window, _cx| {
        let mut alert = apply_config(alert, &config);

        if config.on_ok.is_some() || config.on_open_change.is_some() {
            let runtime_commands = runtime_commands.clone();
            let on_ok = config.on_ok;
            let on_open_change = config.on_open_change;
            let suppressed = suppressed.clone();
            alert = alert.on_ok(move |_, _, _| {
                *suppressed.borrow_mut() = true;
                if let Some(handler_id) = on_ok {
                    dispatch_string_event(handler_id, &runtime_commands, "Alert onOk");
                }
                if let Some(handler_id) = on_open_change {
                    dispatch_open_change(handler_id, "ok", &runtime_commands, "Alert onOpenChange");
                }
                true
            });
        }

        if config.on_cancel.is_some() || config.on_open_change.is_some() {
            let runtime_commands = runtime_commands.clone();
            let on_cancel = config.on_cancel;
            let on_open_change = config.on_open_change;
            let suppressed = suppressed.clone();
            alert = alert.on_cancel(move |_, _, _| {
                *suppressed.borrow_mut() = true;
                if let Some(handler_id) = on_cancel {
                    dispatch_string_event(handler_id, &runtime_commands, "Alert onCancel");
                }
                if let Some(handler_id) = on_open_change {
                    dispatch_open_change(
                        handler_id,
                        "cancel",
                        &runtime_commands,
                        "Alert onOpenChange",
                    );
                }
                true
            });
        }

        if let Some(handler_id) = config.on_close {
            let runtime_commands = runtime_commands.clone();
            alert = alert.on_close(move |_, _, _| {
                dispatch_string_event(handler_id, &runtime_commands, "Alert onClose");
            });
        }

        for child in &children {
            alert = alert.child(render_node_child(
                *child,
                &render_context.tree,
                &render_context.owners,
                &render_context.perf,
                render_context.runtime_commands.clone(),
                render_context.root.clone(),
            ));
        }

        alert
    });
}

fn apply_config(mut alert: AlertDialog, config: &AlertConfig) -> AlertDialog {
    if let Some(title) = config.title.clone() {
        alert = alert.title(div().child(title));
    }
    if let Some(description) = config.description.clone() {
        alert = alert.description(div().child(description));
    }
    if let Some(icon) = config
        .icon
        .as_deref()
        .and_then(parse_icon_name)
        .map(Icon::new)
    {
        alert = alert.icon(icon);
    }

    let mut button_props = DialogButtonProps::default().show_cancel(config.show_cancel);
    if let Some(ok_text) = config.ok_text.clone() {
        button_props = button_props.ok_text(ok_text);
    }
    if let Some(cancel_text) = config.cancel_text.clone() {
        button_props = button_props.cancel_text(cancel_text);
    }
    if let Some(variant) = config.ok_variant.clone().and_then(parse_button_variant) {
        button_props = button_props.ok_variant(variant);
    }
    if let Some(variant) = config.cancel_variant.clone().and_then(parse_button_variant) {
        button_props = button_props.cancel_variant(variant);
    }

    alert
        .button_props(button_props)
        .width(px(config.width as f32))
        .overlay_closable(config.overlay_closable)
        .keyboard(config.keyboard)
        .close_button(config.close_button)
}
