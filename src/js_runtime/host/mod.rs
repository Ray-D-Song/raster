use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

pub mod batch;

use llrt_core::{Ctx, Exception, Function, Object, Result as JsResult, Value};

use crate::common::{
    channel::{
        ChannelReceiver, ChannelSender, CommitQueue, NoopWakeSignal, NotificationCommandPayload,
        NotificationType, UiCommand, WakeSignal, channel,
    },
    ids::{HandlerId, NativeObjectId, SurfaceId},
    mount::{HandlerBinding, MountMutation, NodePayload, NodeValue, RetainedNodeKind},
};

use self::batch::HostMutationBatcher;

pub type NativeBindingState = Arc<NativeBinding>;

pub struct NativeBinding {
    inner: Mutex<NativeHostState>,
    commits: Mutex<CommitQueue>,
    ui_commands: Mutex<UiCommandQueue>,
    wake: Mutex<Arc<dyn WakeSignal>>,
    theme_snapshot_json: Mutex<String>,
}

#[derive(Debug)]
struct UiCommandQueue {
    sender: ChannelSender<UiCommand>,
    receiver: ChannelReceiver<UiCommand>,
}

#[derive(Debug)]
struct NativeHostState {
    next_surface_id: u64,
    next_node_id: u64,
    surfaces: BTreeMap<SurfaceId, SurfaceOptions>,
    nodes: BTreeMap<NativeObjectId, HostNode>,
    roots: BTreeMap<SurfaceId, Vec<NativeObjectId>>,
    batcher: HostMutationBatcher,
    next_handler_id: u64,
    handler_slots: BTreeMap<HandlerSlotKey, HandlerId>,
}

#[derive(Debug, Clone, Default)]
pub struct SurfaceOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub perfdetect: bool,
}

#[derive(Debug, Clone)]
struct HostNode {
    id: NativeObjectId,
    surface_id: SurfaceId,
    kind: RetainedNodeKind,
    name: String,
    key: Option<String>,
    payload: NodePayload,
    children: Vec<NativeObjectId>,
    materialized: bool,
}

#[derive(Debug, Clone, Copy)]
struct NativeHandle {
    surface_id: SurfaceId,
    node_id: NativeObjectId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct HandlerSlotKey {
    surface_id: SurfaceId,
    node_id: NativeObjectId,
    kind: String,
    property: String,
}

pub fn new_native_binding_state() -> NativeBindingState {
    let commits = CommitQueue::new();
    let ui_commands = UiCommandQueue::new();
    Arc::new(NativeBinding {
        inner: Mutex::new(NativeHostState::new(commits.sender())),
        commits: Mutex::new(commits),
        ui_commands: Mutex::new(ui_commands),
        wake: Mutex::new(Arc::new(NoopWakeSignal)),
        theme_snapshot_json: Mutex::new("{}".to_owned()),
    })
}

impl UiCommandQueue {
    fn new() -> Self {
        let (sender, receiver) = channel();
        Self { sender, receiver }
    }

    fn sender(&self) -> ChannelSender<UiCommand> {
        self.sender.clone()
    }

    fn drain(&self) -> Vec<UiCommand> {
        self.receiver.drain()
    }
}

impl NativeBinding {
    pub fn set_commit_wake(&self, wake: Arc<dyn WakeSignal>) {
        if let Ok(mut current) = self.wake.lock() {
            *current = wake;
        }
    }

    fn commit_wake(&self) -> Arc<dyn WakeSignal> {
        self.wake
            .lock()
            .map(|wake| wake.clone())
            .unwrap_or_else(|_| Arc::new(NoopWakeSignal))
    }

    #[allow(dead_code)]
    pub fn drain_commits(&self) -> Vec<crate::common::mount::MountMutationBatch> {
        self.commits
            .lock()
            .map(|commits| commits.drain())
            .unwrap_or_default()
    }

    pub fn surface_options(&self, surface_id: SurfaceId) -> SurfaceOptions {
        self.inner
            .lock()
            .ok()
            .and_then(|state| state.surfaces.get(&surface_id).cloned())
            .unwrap_or_default()
    }

    pub fn root_surface_options(&self) -> SurfaceOptions {
        self.inner
            .lock()
            .ok()
            .and_then(|state| {
                state
                    .surfaces
                    .iter()
                    .next()
                    .map(|(_, options)| options.clone())
            })
            .unwrap_or_default()
    }

    pub fn drain_ui_commands(&self) -> Vec<UiCommand> {
        self.ui_commands
            .lock()
            .map(|commands| commands.drain())
            .unwrap_or_default()
    }

    pub fn set_theme_snapshot_json(&self, snapshot: String) {
        if let Ok(mut current) = self.theme_snapshot_json.lock() {
            *current = snapshot;
        }
    }

    fn theme_snapshot_json(&self) -> String {
        self.theme_snapshot_json
            .lock()
            .map(|snapshot| snapshot.clone())
            .unwrap_or_else(|_| "{}".to_owned())
    }

    fn submit_ui_command(&self, command: UiCommand) -> anyhow::Result<()> {
        let sender = self
            .ui_commands
            .lock()
            .map_err(|_| anyhow::anyhow!("ui command queue lock poisoned"))?
            .sender();
        sender
            .send(command)
            .map_err(|_| anyhow::anyhow!("ui command receiver has been dropped"))?;
        self.commit_wake().wake();
        Ok(())
    }
}

impl NativeHostState {
    fn new(
        commit_sender: crate::common::channel::ChannelSender<
            crate::common::mount::MountMutationBatch,
        >,
    ) -> Self {
        Self {
            next_surface_id: 1,
            next_node_id: 1,
            surfaces: BTreeMap::new(),
            nodes: BTreeMap::new(),
            roots: BTreeMap::new(),
            batcher: HostMutationBatcher::new(commit_sender),
            next_handler_id: 1,
            handler_slots: BTreeMap::new(),
        }
    }

    fn create_surface(&mut self, options: SurfaceOptions) -> SurfaceId {
        let id = SurfaceId(self.next_surface_id);
        self.next_surface_id += 1;
        self.surfaces.insert(id, options);
        self.roots.insert(id, Vec::new());
        id
    }

    fn allocate_node_id(&mut self) -> NativeObjectId {
        let id = NativeObjectId(self.next_node_id);
        self.next_node_id += 1;
        id
    }

    fn create_node(
        &mut self,
        surface_id: SurfaceId,
        kind: RetainedNodeKind,
        name: String,
        key: Option<String>,
        payload: NodePayload,
    ) -> anyhow::Result<NativeObjectId> {
        self.ensure_surface(surface_id)?;
        let id = self.allocate_node_id();
        self.nodes.insert(
            id,
            HostNode {
                id,
                surface_id,
                kind,
                name,
                key,
                payload,
                children: Vec::new(),
                materialized: false,
            },
        );
        Ok(id)
    }

    fn append_initial_child(
        &mut self,
        parent: NativeObjectId,
        child: NativeObjectId,
    ) -> anyhow::Result<()> {
        self.ensure_same_surface(parent, child)?;
        let parent = self.expect_node_mut(parent)?;
        if !parent.children.contains(&child) {
            parent.children.push(child);
        }
        Ok(())
    }

    fn prepare_for_commit(&mut self, surface_id: SurfaceId) -> anyhow::Result<()> {
        self.ensure_surface(surface_id)?;
        self.batcher.prepare_for_commit(surface_id);
        Ok(())
    }

    fn reset_after_commit(&mut self, wake: &dyn WakeSignal) -> anyhow::Result<()> {
        self.batcher.reset_after_commit(wake)
    }

    fn clear_container_children(&mut self, surface_id: SurfaceId) -> anyhow::Result<()> {
        self.ensure_surface(surface_id)?;
        self.roots.insert(surface_id, Vec::new());
        self.batcher.push(MountMutation::SetRootChildren {
            surface_id,
            children: Vec::new(),
        });
        Ok(())
    }

    fn clear_surface(
        &mut self,
        surface_id: SurfaceId,
        wake: &dyn WakeSignal,
    ) -> anyhow::Result<()> {
        self.ensure_surface(surface_id)?;
        self.batcher.prepare_for_commit(surface_id);
        let nodes = self
            .nodes
            .values()
            .filter(|node| node.surface_id == surface_id)
            .map(|node| node.id)
            .collect::<Vec<_>>();
        for node in nodes {
            self.delete_node(node)?;
        }
        self.roots.insert(surface_id, Vec::new());
        self.batcher.push(MountMutation::SetRootChildren {
            surface_id,
            children: Vec::new(),
        });
        self.batcher.reset_after_commit(wake)
    }

    fn append_child(
        &mut self,
        parent: NativeObjectId,
        child: NativeObjectId,
    ) -> anyhow::Result<()> {
        self.ensure_same_surface(parent, child)?;
        self.materialize_subtree(child)?;
        self.move_child_to_parent(parent, child, None)?;
        self.batcher
            .push(MountMutation::AppendChild { parent, child });
        Ok(())
    }

    fn append_child_to_container(
        &mut self,
        surface_id: SurfaceId,
        child: NativeObjectId,
    ) -> anyhow::Result<()> {
        self.ensure_node_in_surface(child, surface_id)?;
        self.materialize_subtree(child)?;
        let roots = self.roots.entry(surface_id).or_default();
        roots.retain(|id| *id != child);
        roots.push(child);
        self.batcher.push(MountMutation::SetRootChildren {
            surface_id,
            children: roots.clone(),
        });
        Ok(())
    }

    fn insert_before(
        &mut self,
        parent: NativeObjectId,
        child: NativeObjectId,
        before: NativeObjectId,
    ) -> anyhow::Result<()> {
        self.ensure_same_surface(parent, child)?;
        self.ensure_same_surface(parent, before)?;
        self.materialize_subtree(child)?;
        self.move_child_to_parent(parent, child, Some(before))?;
        self.batcher.push(MountMutation::InsertBefore {
            parent,
            child,
            before,
        });
        Ok(())
    }

    fn insert_in_container_before(
        &mut self,
        surface_id: SurfaceId,
        child: NativeObjectId,
        before: NativeObjectId,
    ) -> anyhow::Result<()> {
        self.ensure_node_in_surface(child, surface_id)?;
        self.ensure_node_in_surface(before, surface_id)?;
        self.materialize_subtree(child)?;
        let roots = self.roots.entry(surface_id).or_default();
        roots.retain(|id| *id != child);
        let index = roots
            .iter()
            .position(|id| *id == before)
            .unwrap_or(roots.len());
        roots.insert(index, child);
        self.batcher.push(MountMutation::SetRootChildren {
            surface_id,
            children: roots.clone(),
        });
        Ok(())
    }

    fn remove_child(
        &mut self,
        parent: NativeObjectId,
        child: NativeObjectId,
    ) -> anyhow::Result<()> {
        self.ensure_same_surface(parent, child)?;
        self.expect_node_mut(parent)?
            .children
            .retain(|id| *id != child);
        self.batcher
            .push(MountMutation::RemoveChild { parent, child });
        Ok(())
    }

    fn remove_child_from_container(
        &mut self,
        surface_id: SurfaceId,
        child: NativeObjectId,
    ) -> anyhow::Result<()> {
        self.ensure_node_in_surface(child, surface_id)?;
        let roots = self.roots.entry(surface_id).or_default();
        roots.retain(|id| *id != child);
        self.batcher.push(MountMutation::SetRootChildren {
            surface_id,
            children: roots.clone(),
        });
        Ok(())
    }

    fn update_node(&mut self, id: NativeObjectId, payload: NodePayload) -> anyhow::Result<()> {
        self.expect_node_mut(id)?.payload = payload.clone();
        self.batcher.push(MountMutation::UpdateNode { id, payload });
        Ok(())
    }

    fn update_text(&mut self, id: NativeObjectId, text: String) -> anyhow::Result<()> {
        let node = self.expect_node_mut(id)?;
        node.payload.text = Some(text.clone());
        self.batcher.push(MountMutation::UpdateText { id, text });
        Ok(())
    }

    fn delete_node(&mut self, id: NativeObjectId) -> anyhow::Result<()> {
        let Some(node) = self.nodes.remove(&id) else {
            return Ok(());
        };
        self.drop_handler_slots_for_node(node.surface_id, id);
        for child in node.children {
            self.delete_node(child)?;
        }
        self.batcher.push(MountMutation::DeleteNode { id });
        Ok(())
    }

    fn materialize_subtree(&mut self, id: NativeObjectId) -> anyhow::Result<()> {
        let node = self.expect_node(id)?.clone();
        if node.materialized {
            return Ok(());
        }

        match node.kind {
            RetainedNodeKind::Text => {
                self.batcher.push(MountMutation::CreateText {
                    id: node.id,
                    text: node.payload.text.clone().unwrap_or_default(),
                    payload: node.payload.clone(),
                });
            }
            _ => {
                self.batcher.push(MountMutation::CreateNode {
                    id: node.id,
                    kind: node.kind.clone(),
                    name: node.name.clone(),
                    key: node.key.clone(),
                    payload: node.payload.clone(),
                });
            }
        }

        self.expect_node_mut(id)?.materialized = true;
        for child in node.children {
            self.materialize_subtree(child)?;
            self.batcher
                .push(MountMutation::AppendChild { parent: id, child });
        }
        Ok(())
    }

    fn move_child_to_parent(
        &mut self,
        parent: NativeObjectId,
        child: NativeObjectId,
        before: Option<NativeObjectId>,
    ) -> anyhow::Result<()> {
        let parent_node = self.expect_node_mut(parent)?;
        parent_node.children.retain(|id| *id != child);
        let index = before
            .and_then(|before| parent_node.children.iter().position(|id| *id == before))
            .unwrap_or(parent_node.children.len());
        parent_node.children.insert(index, child);
        Ok(())
    }

    fn ensure_surface(&self, surface_id: SurfaceId) -> anyhow::Result<()> {
        if !self.surfaces.contains_key(&surface_id) {
            anyhow::bail!("unknown Raster surface {:?}", surface_id);
        }
        Ok(())
    }

    fn ensure_node_in_surface(
        &self,
        id: NativeObjectId,
        surface_id: SurfaceId,
    ) -> anyhow::Result<()> {
        let node = self.expect_node(id)?;
        if node.surface_id != surface_id {
            anyhow::bail!(
                "node {:?} belongs to surface {:?}, not {:?}",
                id,
                node.surface_id,
                surface_id
            );
        }
        Ok(())
    }

    fn ensure_same_surface(
        &self,
        left: NativeObjectId,
        right: NativeObjectId,
    ) -> anyhow::Result<SurfaceId> {
        let left = self.expect_node(left)?;
        let right = self.expect_node(right)?;
        if left.surface_id != right.surface_id {
            anyhow::bail!("nodes belong to different surfaces");
        }
        Ok(left.surface_id)
    }

    fn expect_node(&self, id: NativeObjectId) -> anyhow::Result<&HostNode> {
        self.nodes
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("unknown Raster node {:?}", id))
    }

    fn expect_node_mut(&mut self, id: NativeObjectId) -> anyhow::Result<&mut HostNode> {
        self.nodes
            .get_mut(&id)
            .ok_or_else(|| anyhow::anyhow!("unknown Raster node {:?}", id))
    }

    fn register_handler_slot(
        &mut self,
        surface_id: SurfaceId,
        node_id: NativeObjectId,
        kind: String,
        property: String,
    ) -> anyhow::Result<HandlerId> {
        self.ensure_node_in_surface(node_id, surface_id)?;
        let key = HandlerSlotKey {
            surface_id,
            node_id,
            kind,
            property,
        };
        if let Some(id) = self.handler_slots.get(&key) {
            return Ok(*id);
        }
        let id = HandlerId(self.next_handler_id);
        self.next_handler_id += 1;
        self.handler_slots.insert(key, id);
        Ok(id)
    }

    fn drop_handler_slots_for_node(&mut self, surface_id: SurfaceId, node_id: NativeObjectId) {
        self.handler_slots
            .retain(|key, _| key.surface_id != surface_id || key.node_id != node_id);
    }
}

pub fn install_native_binding<'js>(ctx: Ctx<'js>, state: NativeBindingState) -> JsResult<()> {
    let binding = Object::new(ctx.clone())?;

    {
        let state = state.clone();
        binding.set(
            "createSurface",
            Function::new(ctx.clone(), move |ctx: Ctx<'js>, _options: Value<'js>| {
                create_surface(ctx, state.clone(), _options)
            })?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "createNode",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>,
                      surface_id: u64,
                      kind: String,
                      name: String,
                      key: Option<String>,
                      payload: Value<'js>| {
                    create_node(ctx, state.clone(), surface_id, kind, name, key, payload)
                },
            )?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "createTextNode",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, surface_id: u64, text: String, payload: Value<'js>| {
                    create_text_node(ctx, state.clone(), surface_id, text, payload)
                },
            )?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "appendInitialChild",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, parent: Value<'js>, child: Value<'js>| {
                    append_initial_child(ctx, state.clone(), parent, child)
                },
            )?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "prepareForCommit",
            Function::new(ctx.clone(), move |ctx: Ctx<'js>, surface_id: u64| {
                prepare_for_commit(ctx, state.clone(), surface_id)
            })?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "resetAfterCommit",
            Function::new(ctx.clone(), move |ctx: Ctx<'js>, _surface_id: u64| {
                reset_after_commit(ctx, state.clone())
            })?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "clearContainerChildren",
            Function::new(ctx.clone(), move |ctx: Ctx<'js>, surface_id: u64| {
                clear_container_children(ctx, state.clone(), surface_id)
            })?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "clearSurface",
            Function::new(ctx.clone(), move |ctx: Ctx<'js>, surface_id: u64| {
                clear_surface(ctx, state.clone(), surface_id)
            })?,
        )?;
    }

    install_mutation_functions(ctx.clone(), state.clone(), &binding)?;
    install_handler_functions(ctx.clone(), state.clone(), &binding)?;
    install_notification_functions(ctx.clone(), state.clone(), &binding)?;
    install_chart_functions(ctx.clone(), state.clone(), &binding)?;
    install_theme_functions(ctx.clone(), state.clone(), &binding)?;

    ctx.globals().set("__rasterNative", binding)
}

fn install_theme_functions<'js>(
    ctx: Ctx<'js>,
    state: NativeBindingState,
    binding: &Object<'js>,
) -> JsResult<()> {
    binding.set(
        "getTheme",
        Function::new(ctx, move || state.theme_snapshot_json())?,
    )?;
    Ok(())
}

fn install_chart_functions<'js>(
    ctx: Ctx<'js>,
    state: NativeBindingState,
    binding: &Object<'js>,
) -> JsResult<()> {
    {
        let state = state.clone();
        binding.set(
            "chartAppendData",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, handle: Value<'js>, rows: Value<'js>| {
                    let handle = handle_from_js(&ctx, handle)?;
                    let rows = chart_rows_from_js(rows)?;
                    to_js_result(&ctx, || {
                        state.submit_ui_command(UiCommand::ChartAppendData {
                            node_id: handle.node_id,
                            rows,
                        })
                    })
                },
            )?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "chartReplaceData",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, handle: Value<'js>, rows: Value<'js>| {
                    let handle = handle_from_js(&ctx, handle)?;
                    let rows = chart_rows_from_js(rows)?;
                    to_js_result(&ctx, || {
                        state.submit_ui_command(UiCommand::ChartReplaceData {
                            node_id: handle.node_id,
                            rows,
                        })
                    })
                },
            )?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "chartClearData",
            Function::new(ctx.clone(), move |ctx: Ctx<'js>, handle: Value<'js>| {
                let handle = handle_from_js(&ctx, handle)?;
                to_js_result(&ctx, || {
                    state.submit_ui_command(UiCommand::ChartClearData {
                        node_id: handle.node_id,
                    })
                })
            })?,
        )?;
    }

    Ok(())
}

fn install_notification_functions<'js>(
    ctx: Ctx<'js>,
    state: NativeBindingState,
    binding: &Object<'js>,
) -> JsResult<()> {
    {
        let state = state.clone();
        binding.set(
            "notificationShow",
            Function::new(ctx.clone(), move |ctx: Ctx<'js>, options: Value<'js>| {
                let payload = notification_payload_from_js(&ctx, options)?;
                to_js_result(&ctx, || {
                    state.submit_ui_command(UiCommand::ShowNotification(payload))
                })
            })?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "notificationDismiss",
            Function::new(ctx.clone(), move |ctx: Ctx<'js>, id: String| {
                to_js_result(&ctx, || {
                    state.submit_ui_command(UiCommand::DismissNotification { id })
                })
            })?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "notificationClear",
            Function::new(ctx.clone(), move |ctx: Ctx<'js>| {
                to_js_result(&ctx, || {
                    state.submit_ui_command(UiCommand::ClearNotifications)
                })
            })?,
        )?;
    }

    Ok(())
}

fn install_mutation_functions<'js>(
    ctx: Ctx<'js>,
    state: NativeBindingState,
    binding: &Object<'js>,
) -> JsResult<()> {
    {
        let state = state.clone();
        binding.set(
            "appendChild",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, parent: Value<'js>, child: Value<'js>| {
                    with_handles(&ctx, parent, child, |parent, child| {
                        state
                            .inner
                            .lock()
                            .map_err(|_| anyhow::anyhow!("native binding state lock poisoned"))?
                            .append_child(parent.node_id, child.node_id)
                    })
                },
            )?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "appendChildToContainer",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, surface_id: u64, child: Value<'js>| {
                    let child = handle_from_js(&ctx, child)?;
                    to_js_result(&ctx, || {
                        state
                            .inner
                            .lock()
                            .map_err(|_| anyhow::anyhow!("native binding state lock poisoned"))?
                            .append_child_to_container(SurfaceId(surface_id), child.node_id)
                    })
                },
            )?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "insertBefore",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, parent: Value<'js>, child: Value<'js>, before: Value<'js>| {
                    let parent = handle_from_js(&ctx, parent)?;
                    let child = handle_from_js(&ctx, child)?;
                    let before = handle_from_js(&ctx, before)?;
                    to_js_result(&ctx, || {
                        state
                            .inner
                            .lock()
                            .map_err(|_| anyhow::anyhow!("native binding state lock poisoned"))?
                            .insert_before(parent.node_id, child.node_id, before.node_id)
                    })
                },
            )?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "insertInContainerBefore",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, surface_id: u64, child: Value<'js>, before: Value<'js>| {
                    let child = handle_from_js(&ctx, child)?;
                    let before = handle_from_js(&ctx, before)?;
                    to_js_result(&ctx, || {
                        state
                            .inner
                            .lock()
                            .map_err(|_| anyhow::anyhow!("native binding state lock poisoned"))?
                            .insert_in_container_before(
                                SurfaceId(surface_id),
                                child.node_id,
                                before.node_id,
                            )
                    })
                },
            )?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "removeChild",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, parent: Value<'js>, child: Value<'js>| {
                    with_handles(&ctx, parent, child, |parent, child| {
                        state
                            .inner
                            .lock()
                            .map_err(|_| anyhow::anyhow!("native binding state lock poisoned"))?
                            .remove_child(parent.node_id, child.node_id)
                    })
                },
            )?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "removeChildFromContainer",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, surface_id: u64, child: Value<'js>| {
                    let child = handle_from_js(&ctx, child)?;
                    to_js_result(&ctx, || {
                        state
                            .inner
                            .lock()
                            .map_err(|_| anyhow::anyhow!("native binding state lock poisoned"))?
                            .remove_child_from_container(SurfaceId(surface_id), child.node_id)
                    })
                },
            )?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "updateNode",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, handle: Value<'js>, payload: Value<'js>| {
                    let handle = handle_from_js(&ctx, handle)?;
                    let payload = payload_from_js(payload)?;
                    to_js_result(&ctx, || {
                        state
                            .inner
                            .lock()
                            .map_err(|_| anyhow::anyhow!("native binding state lock poisoned"))?
                            .update_node(handle.node_id, payload)
                    })
                },
            )?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "updateTextNode",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, handle: Value<'js>, text: String| {
                    let handle = handle_from_js(&ctx, handle)?;
                    to_js_result(&ctx, || {
                        state
                            .inner
                            .lock()
                            .map_err(|_| anyhow::anyhow!("native binding state lock poisoned"))?
                            .update_text(handle.node_id, text)
                    })
                },
            )?,
        )?;
    }

    {
        let state = state.clone();
        binding.set(
            "deleteNode",
            Function::new(ctx.clone(), move |ctx: Ctx<'js>, handle: Value<'js>| {
                let handle = handle_from_js(&ctx, handle)?;
                to_js_result(&ctx, || {
                    state
                        .inner
                        .lock()
                        .map_err(|_| anyhow::anyhow!("native binding state lock poisoned"))?
                        .delete_node(handle.node_id)
                })
            })?,
        )?;
    }

    Ok(())
}

fn install_handler_functions<'js>(
    ctx: Ctx<'js>,
    state: NativeBindingState,
    binding: &Object<'js>,
) -> JsResult<()> {
    {
        let state = state.clone();
        binding.set(
            "registerHandlerSlot",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>,
                      surface_id: u64,
                      node_tag: u64,
                      kind: String,
                      property: String,
                      _event_or_query_type: Option<String>| {
                    state
                        .inner
                        .lock()
                        .map_err(|_| js_error(&ctx, "native binding state lock poisoned"))?
                        .register_handler_slot(
                            SurfaceId(surface_id),
                            NativeObjectId(node_tag),
                            kind,
                            property,
                        )
                        .map(|id| id.0)
                        .map_err(|error| js_error(&ctx, &error.to_string()))
                },
            )?,
        )?;
    }
    binding.set(
        "updateHandlerSlot",
        Function::new(
            ctx.clone(),
            move |_handler_slot_id: u64, _js_function_ref: Value<'js>| {
                Ok::<_, llrt_core::Error>(())
            },
        )?,
    )?;
    {
        let state = state.clone();
        binding.set(
            "dropHandlerSlotsForNode",
            Function::new(ctx.clone(), move |surface_id: u64, node_tag: u64| {
                if let Ok(mut state) = state.inner.lock() {
                    state.drop_handler_slots_for_node(
                        SurfaceId(surface_id),
                        NativeObjectId(node_tag),
                    );
                }
                Ok::<_, llrt_core::Error>(())
            })?,
        )?;
    }
    Ok(())
}

fn create_surface<'js>(
    ctx: Ctx<'js>,
    state: NativeBindingState,
    options: Value<'js>,
) -> JsResult<u64> {
    let options = surface_options_from_js(options)?;
    let id = state
        .inner
        .lock()
        .map_err(|_| js_error(&ctx, "native binding state lock poisoned"))?
        .create_surface(options);
    Ok(id.0)
}

fn create_node<'js>(
    ctx: Ctx<'js>,
    state: NativeBindingState,
    surface_id: u64,
    kind: String,
    name: String,
    key: Option<String>,
    payload: Value<'js>,
) -> JsResult<Object<'js>> {
    let payload = payload_from_js(payload)?;
    let handle = {
        let mut state = state
            .inner
            .lock()
            .map_err(|_| js_error(&ctx, "native binding state lock poisoned"))?;
        let id = state
            .create_node(
                SurfaceId(surface_id),
                retained_kind_from_native_kind(&kind),
                name,
                key,
                payload,
            )
            .map_err(|error| js_error(&ctx, &error.to_string()))?;
        NativeHandle {
            surface_id: SurfaceId(surface_id),
            node_id: id,
        }
    };
    node_handle_to_js(&ctx, handle)
}

fn create_text_node<'js>(
    ctx: Ctx<'js>,
    state: NativeBindingState,
    surface_id: u64,
    text: String,
    payload: Value<'js>,
) -> JsResult<Object<'js>> {
    let mut payload = payload_from_js(payload)?;
    payload.text = Some(text);
    let handle = {
        let mut state = state
            .inner
            .lock()
            .map_err(|_| js_error(&ctx, "native binding state lock poisoned"))?;
        let id = state
            .create_node(
                SurfaceId(surface_id),
                RetainedNodeKind::Text,
                "#text".to_owned(),
                None,
                payload,
            )
            .map_err(|error| js_error(&ctx, &error.to_string()))?;
        NativeHandle {
            surface_id: SurfaceId(surface_id),
            node_id: id,
        }
    };
    node_handle_to_js(&ctx, handle)
}

fn append_initial_child<'js>(
    ctx: Ctx<'js>,
    state: NativeBindingState,
    parent: Value<'js>,
    child: Value<'js>,
) -> JsResult<()> {
    with_handles(&ctx, parent, child, |parent, child| {
        state
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("native binding state lock poisoned"))?
            .append_initial_child(parent.node_id, child.node_id)
    })
}

fn prepare_for_commit<'js>(
    ctx: Ctx<'js>,
    state: NativeBindingState,
    surface_id: u64,
) -> JsResult<()> {
    to_js_result(&ctx, || {
        state
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("native binding state lock poisoned"))?
            .prepare_for_commit(SurfaceId(surface_id))
    })
}

fn reset_after_commit<'js>(ctx: Ctx<'js>, state: NativeBindingState) -> JsResult<()> {
    let wake = state.commit_wake();
    to_js_result(&ctx, || {
        state
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("native binding state lock poisoned"))?
            .reset_after_commit(wake.as_ref())
    })
}

fn clear_container_children<'js>(
    ctx: Ctx<'js>,
    state: NativeBindingState,
    surface_id: u64,
) -> JsResult<()> {
    to_js_result(&ctx, || {
        state
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("native binding state lock poisoned"))?
            .clear_container_children(SurfaceId(surface_id))
    })
}

fn clear_surface<'js>(ctx: Ctx<'js>, state: NativeBindingState, surface_id: u64) -> JsResult<()> {
    let wake = state.commit_wake();
    to_js_result(&ctx, || {
        state
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("native binding state lock poisoned"))?
            .clear_surface(SurfaceId(surface_id), wake.as_ref())
    })
}

fn with_handles<'js>(
    ctx: &Ctx<'js>,
    left: Value<'js>,
    right: Value<'js>,
    f: impl FnOnce(NativeHandle, NativeHandle) -> anyhow::Result<()>,
) -> JsResult<()> {
    let left = handle_from_js(ctx, left)?;
    let right = handle_from_js(ctx, right)?;
    to_js_result(ctx, || f(left, right))
}

fn to_js_result<'js>(ctx: &Ctx<'js>, f: impl FnOnce() -> anyhow::Result<()>) -> JsResult<()> {
    f().map_err(|error| js_error(ctx, &error.to_string()))
}

fn handle_from_js<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<NativeHandle> {
    let object = value
        .as_object()
        .ok_or_else(|| js_error(ctx, "native handle must be an object"))?;
    Ok(NativeHandle {
        surface_id: SurfaceId(object.get::<_, u64>("surface_id")?),
        node_id: NativeObjectId(object.get::<_, u64>("node_tag")?),
    })
}

fn node_handle_to_js<'js>(ctx: &Ctx<'js>, handle: NativeHandle) -> JsResult<Object<'js>> {
    let object = Object::new(ctx.clone())?;
    object.set("surface_id", handle.surface_id.0)?;
    object.set("node_tag", handle.node_id.0)?;
    object.set("revision_id", 0_u64)?;
    object.set("generation", 1_u64)?;
    Ok(object)
}

fn payload_from_js<'js>(value: Value<'js>) -> JsResult<NodePayload> {
    let mut payload = NodePayload::default();
    let Some(object) = value.as_object() else {
        return Ok(payload);
    };

    if let Some(text) = object.get::<_, Option<String>>("text")? {
        payload.text = Some(text);
    }
    if let Some(props) = object.get::<_, Option<Value>>("props")? {
        payload.props = value_to_object_map(props)?;
    }
    if let Some(style) = object.get::<_, Option<Value>>("style")? {
        payload.style = style_value_to_object_map(style)?;
    }
    if let Some(event_bindings) = object.get::<_, Option<Value>>("event_bindings")? {
        payload.event_bindings = handler_bindings_from_js(event_bindings, "event_type")?;
    }
    if let Some(query_bindings) = object.get::<_, Option<Value>>("query_bindings")? {
        payload.query_bindings = handler_bindings_from_js(query_bindings, "query_type")?;
    }
    Ok(payload)
}

fn surface_options_from_js<'js>(value: Value<'js>) -> JsResult<SurfaceOptions> {
    let Some(object) = value.as_object() else {
        return Ok(SurfaceOptions::default());
    };
    Ok(SurfaceOptions {
        width: object.get::<_, Option<u32>>("width")?,
        height: object.get::<_, Option<u32>>("height")?,
        perfdetect: object
            .get::<_, Option<bool>>("perfdetect")?
            .unwrap_or(false),
    })
}

fn notification_payload_from_js<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
) -> JsResult<NotificationCommandPayload> {
    let object = value
        .as_object()
        .ok_or_else(|| js_error(ctx, "notification options must be an object"))?;
    let message = object
        .get::<_, Option<String>>("message")?
        .unwrap_or_default();
    if message.is_empty() {
        return Err(js_error(
            ctx,
            "notification.show requires a non-empty message",
        ));
    }
    Ok(NotificationCommandPayload {
        id: object.get::<_, Option<String>>("id")?,
        type_: notification_type_from_str(
            object
                .get::<_, Option<String>>("type")?
                .as_deref()
                .unwrap_or("info"),
        ),
        title: object.get::<_, Option<String>>("title")?,
        message,
        autohide: object.get::<_, Option<bool>>("autohide")?.unwrap_or(true),
    })
}

fn notification_type_from_str(value: &str) -> NotificationType {
    match value {
        "success" => NotificationType::Success,
        "warning" => NotificationType::Warning,
        "error" => NotificationType::Error,
        _ => NotificationType::Info,
    }
}

fn chart_rows_from_js<'js>(value: Value<'js>) -> JsResult<Vec<NodeValue>> {
    let Some(array) = value.as_array() else {
        return Ok(Vec::new());
    };
    let mut rows = Vec::new();
    for item in array.iter::<Value>() {
        rows.push(node_value_from_js(item?)?);
    }
    Ok(rows)
}

fn handler_bindings_from_js<'js>(
    value: Value<'js>,
    type_field: &str,
) -> JsResult<Vec<HandlerBinding>> {
    let Some(array) = value.as_array() else {
        return Ok(Vec::new());
    };
    let mut bindings = Vec::new();
    for item in array.iter::<Value>() {
        let item = item?;
        let Some(object) = item.as_object() else {
            continue;
        };
        bindings.push(HandlerBinding {
            property: object.get::<_, String>("property")?,
            event_or_query_type: object.get::<_, Option<String>>(type_field)?,
            handler_id: HandlerId(object.get::<_, u64>("handler_slot_id")?),
        });
    }
    Ok(bindings)
}

fn style_value_to_object_map<'js>(value: Value<'js>) -> JsResult<BTreeMap<String, NodeValue>> {
    match value.type_of() {
        llrt_core::Type::Array => {
            let array = value
                .as_array()
                .ok_or_else(|| Exception::throw_type(value.ctx(), "expected style array"))?;
            let mut result = BTreeMap::new();
            for item in array.iter::<Value>() {
                let item = item?;
                if matches!(
                    item.type_of(),
                    llrt_core::Type::Uninitialized
                        | llrt_core::Type::Undefined
                        | llrt_core::Type::Null
                ) {
                    continue;
                }
                result.extend(style_value_to_object_map(item)?);
            }
            Ok(result)
        }
        llrt_core::Type::Object => value_to_object_map(value),
        _ => Ok(BTreeMap::new()),
    }
}

fn value_to_object_map<'js>(value: Value<'js>) -> JsResult<BTreeMap<String, NodeValue>> {
    let Some(object) = value.as_object() else {
        return Ok(BTreeMap::new());
    };
    let mut result = BTreeMap::new();
    for key in object.keys::<String>() {
        let key = key?;
        let value = object.get::<_, Value>(key.as_str())?;
        result.insert(key, node_value_from_js(value)?);
    }
    Ok(result)
}

fn node_value_from_js<'js>(value: Value<'js>) -> JsResult<NodeValue> {
    match value.type_of() {
        llrt_core::Type::Uninitialized | llrt_core::Type::Undefined | llrt_core::Type::Null => {
            Ok(NodeValue::Null)
        }
        llrt_core::Type::Bool => Ok(NodeValue::Bool(value.as_bool().unwrap_or(false))),
        llrt_core::Type::Int | llrt_core::Type::Float => {
            Ok(NodeValue::Number(value.as_number().unwrap_or(0.0)))
        }
        llrt_core::Type::String => Ok(NodeValue::String(
            value
                .as_string()
                .and_then(|string| string.to_string().ok())
                .unwrap_or_default(),
        )),
        llrt_core::Type::Array => {
            let array = value
                .as_array()
                .ok_or_else(|| Exception::throw_type(value.ctx(), "expected array"))?;
            let mut items = Vec::new();
            for item in array.iter::<Value>() {
                items.push(node_value_from_js(item?)?);
            }
            Ok(NodeValue::Array(items))
        }
        llrt_core::Type::Object => Ok(NodeValue::Object(value_to_object_map(value)?)),
        _ => Ok(NodeValue::Null),
    }
}

fn retained_kind_from_native_kind(kind: &str) -> RetainedNodeKind {
    match kind {
        "text" => RetainedNodeKind::Text,
        "input" => RetainedNodeKind::Input,
        "textarea" => RetainedNodeKind::Textarea,
        "widget" | "slot" | "config_provider" => RetainedNodeKind::Widget,
        "fragment" => RetainedNodeKind::Fragment,
        _ => RetainedNodeKind::View,
    }
}

fn js_error(ctx: &Ctx<'_>, message: &str) -> llrt_core::Error {
    Exception::throw_message(ctx, message)
}
