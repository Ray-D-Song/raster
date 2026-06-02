use crate::common::{
    channel::{ChannelSender, WakeSignal},
    ids::SurfaceId,
    mount::{MountMutation, MountMutationBatch},
};

/// Collects one React commit worth of host mutations before sending it to GPUI.
///
/// React mutation mode calls `prepareForCommit` before visible tree mutations
/// and `resetAfterCommit` after them. This batcher follows that boundary: it
/// does not merge across React commits.
#[derive(Debug)]
pub struct HostMutationBatcher {
    next_sequence: u64,
    pending: Option<PendingMountBatch>,
    commit_sender: ChannelSender<MountMutationBatch>,
}

#[derive(Debug)]
struct PendingMountBatch {
    surface_id: SurfaceId,
    mutations: Vec<MountMutation>,
}

impl HostMutationBatcher {
    pub fn new(commit_sender: ChannelSender<MountMutationBatch>) -> Self {
        Self {
            next_sequence: 1,
            pending: None,
            commit_sender,
        }
    }

    pub fn prepare_for_commit(&mut self, surface_id: SurfaceId) {
        self.pending = Some(PendingMountBatch {
            surface_id,
            mutations: Vec::new(),
        });
    }

    pub fn push(&mut self, mutation: MountMutation) {
        let Some(pending) = &mut self.pending else {
            return;
        };
        pending.mutations.push(mutation);
    }

    pub fn reset_after_commit<W: WakeSignal + ?Sized>(&mut self, wake: &W) -> anyhow::Result<()> {
        let Some(pending) = self.pending.take() else {
            return Ok(());
        };

        if pending.mutations.is_empty() {
            return Ok(());
        }

        let batch = MountMutationBatch {
            surface_id: pending.surface_id,
            sequence: self.next_sequence,
            mutations: pending.mutations,
        };
        self.next_sequence += 1;
        self.commit_sender
            .send(batch)
            .map_err(|_| anyhow::anyhow!("commit receiver has been dropped"))?;
        wake.wake();
        Ok(())
    }
}
