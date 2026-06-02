use std::collections::BTreeSet;

use crate::common::ids::{NativeObjectId, SurfaceId};

/// A GPUI render boundary that should be notified after a retained mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OwnerId {
    Surface(SurfaceId),
    Node(NativeObjectId),
}

/// Result of applying one or more DOM-like retained tree operations.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplyOutcome {
    dirty_owners: BTreeSet<OwnerId>,
}

impl ApplyOutcome {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dirty(owner: OwnerId) -> Self {
        let mut outcome = Self::new();
        outcome.mark_dirty(owner);
        outcome
    }

    pub fn mark_dirty(&mut self, owner: OwnerId) {
        self.dirty_owners.insert(owner);
    }

    pub fn merge(&mut self, other: ApplyOutcome) {
        self.dirty_owners.extend(other.dirty_owners);
    }

    pub fn dirty_owners(&self) -> impl Iterator<Item = OwnerId> + '_ {
        self.dirty_owners.iter().copied()
    }

    pub fn is_clean(&self) -> bool {
        self.dirty_owners.is_empty()
    }
}
