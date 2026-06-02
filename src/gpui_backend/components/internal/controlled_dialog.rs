use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use gpui::{App, Window};

use crate::{
    common::{
        channel::{ChannelSender, RuntimeCommand},
        ids::{HandlerId, NativeObjectId},
        mount::NodeValue,
        utils::logger,
    },
    gpui_backend::retained_tree::node::RetainedNode,
};

pub(in crate::gpui_backend) struct ControlledDialogState<C> {
    local_node_id: Option<NativeObjectId>,
    config: C,
    local_open: bool,
    suppressed_until_controlled_close: Rc<RefCell<bool>>,
}

impl<C> ControlledDialogState<C>
where
    C: Clone + Default + PartialEq,
{
    pub(in crate::gpui_backend) fn new() -> Self {
        Self {
            local_node_id: None,
            config: C::default(),
            local_open: false,
            suppressed_until_controlled_close: Rc::new(RefCell::new(false)),
        }
    }

    pub(in crate::gpui_backend) fn sync_closed(
        &mut self,
        close: impl FnOnce(&mut Window, &mut App),
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.local_open {
            close(window, cx);
        }
        self.reset();
    }

    pub(in crate::gpui_backend) fn sync_from_node(
        &mut self,
        node: &RetainedNode,
        active: bool,
        controlled_open: bool,
        next_config: C,
        warning: &str,
        close: impl Fn(&mut Window, &mut App),
        open: impl FnOnce(&RetainedNode, C, Rc<RefCell<bool>>, &mut Window, &mut App),
        window: &mut Window,
        cx: &mut App,
    ) {
        if !controlled_open {
            if self.local_open {
                close(window, cx);
            }
            self.local_node_id = None;
            self.local_open = false;
            *self.suppressed_until_controlled_close.borrow_mut() = false;
            self.config = next_config;
            return;
        }

        if !active {
            if self.local_open {
                close(window, cx);
                self.local_node_id = None;
                self.local_open = false;
            }
            logger::warn(warning);
            return;
        }

        if *self.suppressed_until_controlled_close.borrow() {
            return;
        }

        if self.local_open && self.local_node_id == Some(node.id) && self.config == next_config {
            return;
        }

        if self.local_open {
            close(window, cx);
        }
        open(
            node,
            next_config.clone(),
            self.suppressed_until_controlled_close.clone(),
            window,
            cx,
        );
        self.local_node_id = Some(node.id);
        self.config = next_config;
        self.local_open = true;
    }

    fn reset(&mut self) {
        self.local_node_id = None;
        self.local_open = false;
        *self.suppressed_until_controlled_close.borrow_mut() = false;
        self.config = C::default();
    }
}

pub(in crate::gpui_backend) fn dispatch_string_event(
    handler_id: HandlerId,
    runtime_commands: &ChannelSender<RuntimeCommand>,
    label: &str,
) {
    if runtime_commands
        .send(RuntimeCommand::InvokeEvent {
            handler_id,
            payload: NodeValue::String(String::new()),
        })
        .is_err()
    {
        logger::error(format!("failed to enqueue {label} event"));
    }
}

pub(in crate::gpui_backend) fn dispatch_open_change(
    handler_id: HandlerId,
    reason: &str,
    runtime_commands: &ChannelSender<RuntimeCommand>,
    label: &str,
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
        logger::error(format!("failed to enqueue {label} event"));
    }
}
