use crate::common::{
    channel::{
        queue::{ChannelReceiver, ChannelSender, channel},
        wake::WakeSignal,
    },
    mount::MountMutationBatch,
};

/// JS runtime -> GPUI app thread commit queue.
#[derive(Debug)]
pub struct CommitQueue {
    sender: ChannelSender<MountMutationBatch>,
    receiver: ChannelReceiver<MountMutationBatch>,
}

impl CommitQueue {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self { sender, receiver }
    }

    pub fn sender(&self) -> ChannelSender<MountMutationBatch> {
        self.sender.clone()
    }

    pub fn submit<W: WakeSignal + ?Sized>(
        sender: &ChannelSender<MountMutationBatch>,
        batch: MountMutationBatch,
        wake: &W,
    ) -> anyhow::Result<()> {
        sender
            .send(batch)
            .map_err(|_| anyhow::anyhow!("commit receiver has been dropped"))?;
        wake.wake();
        Ok(())
    }

    pub fn drain(&self) -> Vec<MountMutationBatch> {
        self.receiver.drain()
    }
}

impl Default for CommitQueue {
    fn default() -> Self {
        Self::new()
    }
}
