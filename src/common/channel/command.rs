use std::{path::PathBuf, sync::mpsc};

use crate::common::{
    channel::queue::{ChannelReceiver, ChannelSender, channel},
    ids::HandlerId,
    mount::NodeValue,
};

pub type QueryResponder = mpsc::Sender<NodeValue>;

/// GPUI app thread -> JS runtime thread commands.
#[derive(Debug)]
pub enum RuntimeCommand {
    InvokeEvent {
        handler_id: HandlerId,
        payload: NodeValue,
    },
    InvokeQuery {
        handler_id: HandlerId,
        payload: NodeValue,
        responder: QueryResponder,
    },
    ReloadAppBundle {
        path: PathBuf,
    },
    ReloadAppBundleSource {
        name: String,
        source: String,
    },
    Shutdown,
}

/// Queue for runtime commands produced by GPUI events.
#[derive(Debug)]
pub struct RuntimeCommandQueue {
    sender: ChannelSender<RuntimeCommand>,
    receiver: ChannelReceiver<RuntimeCommand>,
}

impl RuntimeCommandQueue {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self { sender, receiver }
    }

    pub fn sender(&self) -> ChannelSender<RuntimeCommand> {
        self.sender.clone()
    }

    pub fn enqueue(
        sender: &ChannelSender<RuntimeCommand>,
        command: RuntimeCommand,
    ) -> anyhow::Result<()> {
        sender
            .send(command)
            .map_err(|_| anyhow::anyhow!("runtime command receiver has been dropped"))
    }

    pub fn drain(&self) -> Vec<RuntimeCommand> {
        self.receiver.drain()
    }

    pub fn recv(&self) -> Result<RuntimeCommand, std::sync::mpsc::RecvError> {
        self.receiver.recv()
    }

    pub fn try_recv(&self) -> Result<RuntimeCommand, mpsc::TryRecvError> {
        self.receiver.try_recv()
    }
}

impl Default for RuntimeCommandQueue {
    fn default() -> Self {
        Self::new()
    }
}
