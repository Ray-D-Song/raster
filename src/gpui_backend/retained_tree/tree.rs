use std::collections::BTreeMap;

use anyhow::{Result, bail};

use crate::common::{
    ids::{NativeObjectId, SurfaceId},
    mount::{MountMutation, MountMutationBatch, NodePayload, RetainedNodeKind},
};

use super::{
    diff::{NodePayloadChange, diff_node_payload},
    mutation::{ApplyOutcome, OwnerId},
    node::RetainedNode,
};

/// One GPUI window/surface worth of retained roots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedSurface {
    pub id: SurfaceId,
    pub roots: Vec<NativeObjectId>,
}

/// GPUI app-thread retained object tree.
#[derive(Debug, Default)]
pub struct RetainedTree {
    surfaces: BTreeMap<SurfaceId, RetainedSurface>,
    nodes: BTreeMap<NativeObjectId, RetainedNode>,
    next_surface_id: u64,
    next_node_id: u64,
}

impl RetainedTree {
    pub fn new() -> Self {
        Self {
            surfaces: BTreeMap::new(),
            nodes: BTreeMap::new(),
            next_surface_id: 1,
            next_node_id: 1,
        }
    }

    pub fn create_surface(&mut self) -> SurfaceId {
        let id = SurfaceId(self.next_surface_id);
        self.next_surface_id += 1;
        self.surfaces.insert(
            id,
            RetainedSurface {
                id,
                roots: Vec::new(),
            },
        );
        id
    }

    pub fn surface(&self, id: SurfaceId) -> Option<&RetainedSurface> {
        self.surfaces.get(&id)
    }

    pub fn node(&self, id: NativeObjectId) -> Option<&RetainedNode> {
        self.nodes.get(&id)
    }

    pub fn owner_node_ids(&self) -> Vec<NativeObjectId> {
        self.nodes
            .iter()
            .filter_map(|(id, node)| node.is_owner_boundary().then_some(*id))
            .collect()
    }

    pub fn create_node(
        &mut self,
        surface_id: SurfaceId,
        kind: RetainedNodeKind,
        name: impl Into<String>,
        key: Option<String>,
        payload: NodePayload,
    ) -> Result<NativeObjectId> {
        self.ensure_surface(surface_id)?;
        let id = self.allocate_node_id();
        let node = RetainedNode::new(id, surface_id, kind, name, key, payload);
        self.nodes.insert(id, node);
        Ok(id)
    }

    pub fn apply_batch(&mut self, batch: MountMutationBatch) -> Result<ApplyOutcome> {
        self.ensure_surface(batch.surface_id)?;
        let mut outcome = ApplyOutcome::new();

        for mutation in batch.mutations {
            let mutation_outcome = self.apply_mutation(batch.surface_id, mutation)?;
            outcome.merge(mutation_outcome);
        }

        Ok(outcome)
    }

    pub fn create_text(
        &mut self,
        surface_id: SurfaceId,
        text: impl Into<String>,
    ) -> Result<NativeObjectId> {
        let payload = NodePayload {
            text: Some(text.into()),
            ..NodePayload::default()
        };
        self.create_node(surface_id, RetainedNodeKind::Text, "#text", None, payload)
    }

    pub fn set_roots(
        &mut self,
        surface_id: SurfaceId,
        roots: impl IntoIterator<Item = NativeObjectId>,
    ) -> Result<ApplyOutcome> {
        let roots = roots.into_iter().collect::<Vec<_>>();
        for root in &roots {
            self.ensure_node_in_surface(*root, surface_id)?;
        }

        let old_roots = self
            .surface(surface_id)
            .ok_or_else(|| anyhow::anyhow!("unknown retained surface {:?}", surface_id))?
            .roots
            .clone();
        for old_root in old_roots {
            if !roots.contains(&old_root) {
                self.detach_node(old_root)?;
            }
        }
        for root in &roots {
            self.detach_node(*root)?;
        }

        let surface = self
            .surfaces
            .get_mut(&surface_id)
            .expect("surface checked before mutation");
        surface.roots = roots;

        Ok(ApplyOutcome::dirty(OwnerId::Surface(surface_id)))
    }

    pub fn append_child(
        &mut self,
        parent: NativeObjectId,
        child: NativeObjectId,
    ) -> Result<ApplyOutcome> {
        self.insert_child_before(parent, child, None)
    }

    pub fn insert_child_before(
        &mut self,
        parent: NativeObjectId,
        child: NativeObjectId,
        before: Option<NativeObjectId>,
    ) -> Result<ApplyOutcome> {
        let surface_id = self.ensure_same_surface(parent, child)?;
        self.ensure_not_ancestor(child, parent)?;
        if let Some(before) = before {
            self.ensure_same_surface(parent, before)?;
            let parent_node = self.expect_node(parent)?;
            if !parent_node.children.contains(&before) {
                bail!(
                    "before node {:?} is not a child of parent {:?}",
                    before,
                    parent
                );
            }
        }

        let mut outcome = self.detach_node(child)?;
        {
            let parent_node = self.expect_node_mut(parent)?;
            let index = before
                .and_then(|before| parent_node.children.iter().position(|id| *id == before))
                .unwrap_or(parent_node.children.len());
            parent_node.children.insert(index, child);
        }
        self.expect_node_mut(child)?.parent = Some(parent);
        self.remove_surface_root(surface_id, child);
        outcome.mark_dirty(self.owner_for_node(parent)?);
        Ok(outcome)
    }

    pub fn remove_child(
        &mut self,
        parent: NativeObjectId,
        child: NativeObjectId,
    ) -> Result<ApplyOutcome> {
        self.ensure_same_surface(parent, child)?;
        let removed = {
            let parent_node = self.expect_node_mut(parent)?;
            if let Some(index) = parent_node.children.iter().position(|id| *id == child) {
                parent_node.children.remove(index);
                true
            } else {
                false
            }
        };
        if !removed {
            bail!("node {:?} is not a child of {:?}", child, parent);
        }
        self.expect_node_mut(child)?.parent = None;
        Ok(ApplyOutcome::dirty(self.owner_for_node(parent)?))
    }

    pub fn update_node(
        &mut self,
        id: NativeObjectId,
        payload: NodePayload,
    ) -> Result<ApplyOutcome> {
        let change = {
            let node = self.expect_node(id)?;
            diff_node_payload(&node.payload, &payload)
        };

        match change {
            NodePayloadChange::Noop => Ok(ApplyOutcome::new()),
            NodePayloadChange::HandlerOnly => {
                self.expect_node_mut(id)?.payload = payload;
                Ok(ApplyOutcome::new())
            }
            NodePayloadChange::Visual => {
                let owner = self.owner_for_node(id)?;
                self.expect_node_mut(id)?.replace_payload(payload);
                Ok(ApplyOutcome::dirty(owner))
            }
        }
    }

    pub fn update_text(
        &mut self,
        id: NativeObjectId,
        text: impl Into<String>,
    ) -> Result<ApplyOutcome> {
        let text = text.into();
        let owner = self.owner_for_node(id)?;
        let node = self.expect_node_mut(id)?;
        if node.kind != RetainedNodeKind::Text {
            bail!("node {:?} is not a text node", id);
        }
        if node.payload.text.as_deref() == Some(text.as_str()) {
            return Ok(ApplyOutcome::new());
        }
        node.update_text(text);
        Ok(ApplyOutcome::dirty(owner))
    }

    pub fn delete_node(&mut self, id: NativeObjectId) -> Result<ApplyOutcome> {
        let mut outcome = self.detach_node(id)?;
        let owner = self.owner_for_node(id)?;
        outcome.mark_dirty(owner);
        self.delete_subtree(id);
        Ok(outcome)
    }

    pub fn owner_for_node(&self, id: NativeObjectId) -> Result<OwnerId> {
        let node = self.expect_node(id)?;
        if node.is_owner_boundary() {
            return Ok(OwnerId::Node(id));
        }

        let mut current = node.parent;
        while let Some(parent_id) = current {
            let parent = self.expect_node(parent_id)?;
            if parent.is_owner_boundary() {
                return Ok(OwnerId::Node(parent_id));
            }
            current = parent.parent;
        }

        Ok(OwnerId::Surface(node.surface_id))
    }

    fn allocate_node_id(&mut self) -> NativeObjectId {
        let id = NativeObjectId(self.next_node_id);
        self.next_node_id += 1;
        id
    }

    fn create_node_with_id(
        &mut self,
        surface_id: SurfaceId,
        id: NativeObjectId,
        kind: RetainedNodeKind,
        name: impl Into<String>,
        key: Option<String>,
        payload: NodePayload,
    ) -> Result<ApplyOutcome> {
        self.ensure_surface(surface_id)?;
        if self.nodes.contains_key(&id) {
            bail!("retained node {:?} already exists", id);
        }
        self.next_node_id = self.next_node_id.max(id.0 + 1);
        self.nodes.insert(
            id,
            RetainedNode::new(id, surface_id, kind, name, key, payload),
        );
        Ok(ApplyOutcome::new())
    }

    fn apply_mutation(
        &mut self,
        batch_surface_id: SurfaceId,
        mutation: MountMutation,
    ) -> Result<ApplyOutcome> {
        match mutation {
            MountMutation::CreateNode {
                id,
                kind,
                name,
                key,
                payload,
            } => self.create_node_with_id(batch_surface_id, id, kind, name, key, payload),
            MountMutation::CreateText {
                id,
                text,
                mut payload,
            } => {
                payload.text = Some(text);
                self.create_node_with_id(
                    batch_surface_id,
                    id,
                    RetainedNodeKind::Text,
                    "#text",
                    None,
                    payload,
                )
            }
            MountMutation::UpdateNode { id, payload } => self.update_node(id, payload),
            MountMutation::UpdateText { id, text } => self.update_text(id, text),
            MountMutation::AppendChild { parent, child } => self.append_child(parent, child),
            MountMutation::InsertBefore {
                parent,
                child,
                before,
            } => self.insert_child_before(parent, child, Some(before)),
            MountMutation::RemoveChild { parent, child } => self.remove_child(parent, child),
            MountMutation::DeleteNode { id } => self.delete_node(id),
            MountMutation::SetRootChildren {
                surface_id,
                children,
            } => {
                if surface_id != batch_surface_id {
                    bail!(
                        "root mutation targets surface {:?}, but batch targets {:?}",
                        surface_id,
                        batch_surface_id
                    );
                }
                self.set_roots(surface_id, children)
            }
        }
    }

    fn ensure_surface(&self, surface_id: SurfaceId) -> Result<()> {
        if !self.surfaces.contains_key(&surface_id) {
            bail!("unknown retained surface {:?}", surface_id);
        }
        Ok(())
    }

    fn ensure_node_in_surface(&self, id: NativeObjectId, surface_id: SurfaceId) -> Result<()> {
        let node = self.expect_node(id)?;
        if node.surface_id != surface_id {
            bail!(
                "retained node {:?} belongs to surface {:?}, not {:?}",
                id,
                node.surface_id,
                surface_id
            );
        }
        Ok(())
    }

    fn ensure_same_surface(&self, a: NativeObjectId, b: NativeObjectId) -> Result<SurfaceId> {
        let a_node = self.expect_node(a)?;
        let b_node = self.expect_node(b)?;
        if a_node.surface_id != b_node.surface_id {
            bail!(
                "retained nodes {:?} and {:?} belong to different surfaces",
                a,
                b
            );
        }
        Ok(a_node.surface_id)
    }

    fn ensure_not_ancestor(
        &self,
        possible_ancestor: NativeObjectId,
        node: NativeObjectId,
    ) -> Result<()> {
        let mut current = Some(node);
        while let Some(current_id) = current {
            if current_id == possible_ancestor {
                bail!(
                    "cannot insert ancestor {:?} into its descendant",
                    possible_ancestor
                );
            }
            current = self.expect_node(current_id)?.parent;
        }
        Ok(())
    }

    fn detach_node(&mut self, id: NativeObjectId) -> Result<ApplyOutcome> {
        let node = self.expect_node(id)?.clone();
        let mut outcome = ApplyOutcome::new();

        if let Some(parent_id) = node.parent {
            let parent = self.expect_node_mut(parent_id)?;
            parent.children.retain(|child| *child != id);
            self.expect_node_mut(id)?.parent = None;
            outcome.mark_dirty(self.owner_for_node(parent_id)?);
            return Ok(outcome);
        }

        if self.remove_surface_root(node.surface_id, id) {
            outcome.mark_dirty(OwnerId::Surface(node.surface_id));
        }
        Ok(outcome)
    }

    fn remove_surface_root(&mut self, surface_id: SurfaceId, node_id: NativeObjectId) -> bool {
        let Some(surface) = self.surfaces.get_mut(&surface_id) else {
            return false;
        };
        let original_len = surface.roots.len();
        surface.roots.retain(|root| *root != node_id);
        surface.roots.len() != original_len
    }

    fn delete_subtree(&mut self, id: NativeObjectId) {
        let Some(node) = self.nodes.remove(&id) else {
            return;
        };
        for child in node.children {
            self.delete_subtree(child);
        }
    }

    fn expect_node(&self, id: NativeObjectId) -> Result<&RetainedNode> {
        self.nodes
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("unknown retained node {:?}", id))
    }

    fn expect_node_mut(&mut self, id: NativeObjectId) -> Result<&mut RetainedNode> {
        self.nodes
            .get_mut(&id)
            .ok_or_else(|| anyhow::anyhow!("unknown retained node {:?}", id))
    }
}
