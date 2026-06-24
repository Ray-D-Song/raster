//! GPUI app thread runner.

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use futures::{StreamExt, channel::mpsc};
use gpui::{
    AnyElement, App, Context, Empty, Entity, IntoElement, QuitMode, Render, WeakEntity, Window,
    WindowOptions, div, prelude::*,
};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use gpui::{Bounds, SharedString, TitlebarOptions, WindowBounds, px, size};
use gpui_component::ActiveTheme;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use crate::gpui_backend::assets::RasterAssets;
use gpui_component_assets::Assets;

use crate::{
    bridge::{BridgeEventDispatch, SharedBridgeState, emit_handler_invoke, emit_runtime_event},
    common::{
        channel::{UiCommand, WakeSignal},
        ids::{NativeObjectId, SurfaceId},
        mount::{NodeValue, RetainedNodeKind},
        utils::logger,
    },
    gpui_backend::{asset_context::with_render_assets, bridge_host::drain_bridge_ingress},
    gpui_backend::{
        components::{
            alert::{AlertRenderContext, RasterAlertState, is_alert_node},
            avatar::{
                is_avatar_group_node, is_avatar_node, render_avatar_from_node,
                render_avatar_group_from_node,
            },
            button::render_button_from_node,
            button_group::{is_button_group_node, render_button_group_from_node},
            chart::{RasterChartState, is_chart_node, render_chart_from_node},
            checkbox::{is_checkbox_node, render_checkbox_from_node},
            color_picker::{
                RasterColorPickerState, is_color_picker_node, render_color_picker_from_node,
            },
            date_picker::{
                RasterDatePickerState, is_date_picker_node, render_date_picker_from_node,
            },
            time_picker::{
                RasterTimePickerState, is_time_picker_node, render_time_picker_from_node,
            },
            dialog::{DialogRenderContext, RasterDialogState, is_dialog_node},
            form::{is_field_node, is_form_node, render_field_from_node, render_form_from_node},
            helper::props::event_handler,
            icon::{is_icon_node, render_icon_from_node},
            input::{RasterInputState, is_text_control_node, render_input_from_node},
            radio::{
                is_radio_group_node, is_radio_node, render_radio_from_node,
                render_radio_group_from_node,
            },
            select::{RasterSelectState, is_select_node, render_select_from_node},
            sheet::{RasterSheetState, SheetRenderContext, is_sheet_node},
            slider::{RasterSliderState, is_slider_node, render_slider_from_node},
            switch::{is_switch_node, render_switch_from_node},
            tab::{is_tab_bar_node, is_tab_node, render_tab_bar_from_node, render_tab_from_node},
            text_and_label::{
                render_text_label_element_from_node, render_text_label_or_rich_text_from_node,
            },
            virtual_list::{
                RasterVirtualListState, is_virtual_list_node, render_virtual_list_from_node,
            },
        },
        config_provider::{
            RasterThemeSnapshot, apply_raster_default_theme, apply_theme_snapshot,
            find_config_provider_theme, is_config_provider_node, reset_theme_snapshot,
        },
        embedded_themes::load_embedded_themes,
        notification::RasterNotificationCenter,
        perf::{PerfMonitor, decorate_refresh},
        render_model::{
            model::RenderModel,
            style::{
                apply_style, apply_view_style, has_horizontal_scroll_overflow,
                has_scroll_overflow, has_vertical_scroll_overflow,
            },
        },
        retained_tree::mutation::{ApplyOutcome, OwnerId},
        retained_tree::tree::RetainedTree,
        theme_snapshot::theme_snapshot_json,
    },
    js_runtime::host::NativeBindingState,
};

pub struct DevReloadConfig {
    pub demo_bundle_path: std::path::PathBuf,
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn start_desktop(
    width: u32,
    height: u32,
    dev_reload: Option<DevReloadConfig>,
    native_binding: NativeBindingState,
    bridge: SharedBridgeState,
) {
    logger::info("gpui_backend initialize start");
    gpui_platform::application()
        .with_assets(RasterAssets::new(Assets))
        .with_quit_mode(QuitMode::LastWindowClosed)
        .run(move |cx: &mut App| {
            if let Some(config) = &dev_reload {
                logger::info(format!(
                    "gpui_backend dev bundle path: {}",
                    config.demo_bundle_path.display()
                ));
            }

            let bounds = Bounds::centered(None, size(px(width as f32), px(height as f32)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from("Raster template")),
                    appears_transparent: false,
                    traffic_light_position: None,
                }),
                focus: true,
                show: true,
                is_movable: true,
                is_resizable: true,
                is_minimizable: true,
                ..WindowOptions::default()
            };

            open_raster_window(cx, options, native_binding, bridge);
            cx.activate(true);
            logger::info("gpui_backend initialize success");
        });
}

pub fn open_raster_window(
    cx: &mut App,
    options: WindowOptions,
    native_binding: NativeBindingState,
    bridge: SharedBridgeState,
) {
    gpui_component::init(cx);
    load_embedded_themes(cx);
    apply_raster_default_theme(cx);
    native_binding.set_theme_snapshot_json(theme_snapshot_json(cx));

    cx.on_window_closed(|cx, _| {
        if cx.windows().is_empty() {
            cx.quit();
        }
    })
    .detach();

    cx.open_window(options, |window, cx| {
        let raster_root = cx.new(|cx| RasterRootView::new(native_binding, bridge, cx));
        cx.new(|cx| gpui_component::Root::new(raster_root, window, cx))
    })
    .expect("failed to open Raster GPUI window");
}

pub(in crate::gpui_backend) struct RasterRootView {
    native_binding: NativeBindingState,
    bridge: SharedBridgeState,
    tree: Rc<RefCell<RetainedTree>>,
    owners: Rc<RefCell<OwnerRegistry>>,
    perf: Rc<RefCell<PerfMonitor>>,
    notification_center: Entity<RasterNotificationCenter>,
    alert_state: RasterAlertState,
    dialog_state: RasterDialogState,
    sheet_state: RasterSheetState,
    surface_id: SurfaceId,
    applied_theme: Option<RasterThemeSnapshot>,
}

impl RasterRootView {
    fn new(
        native_binding: NativeBindingState,
        _bridge: SharedBridgeState,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut tree = RetainedTree::new();
        let surface_id = tree.create_surface();
        let perfdetect = native_binding.surface_options(surface_id).perfdetect;
        let bridge = native_binding.bridge();
        install_commit_wake(native_binding.clone(), cx);
        Self {
            native_binding,
            bridge,
            tree: Rc::new(RefCell::new(tree)),
            owners: Rc::new(RefCell::new(OwnerRegistry::new())),
            perf: Rc::new(RefCell::new(PerfMonitor::new(perfdetect))),
            notification_center: cx.new(|_| RasterNotificationCenter::new()),
            alert_state: RasterAlertState::new(),
            dialog_state: RasterDialogState::new(),
            sheet_state: RasterSheetState::new(),
            surface_id,
            applied_theme: None,
        }
    }

    pub(in crate::gpui_backend) fn bridge(&self) -> SharedBridgeState {
        self.bridge.clone()
    }

    pub(in crate::gpui_backend) fn apply_ui_command(
        &mut self,
        command: UiCommand,
        cx: &mut Context<Self>,
    ) {
        match command {
            UiCommand::ShowNotification(_)
            | UiCommand::DismissNotification { .. }
            | UiCommand::ClearNotifications => {
                self.notification_center
                    .update(cx, |center, cx| center.apply_command(command, cx));
            }
            UiCommand::ChartAppendData { .. }
            | UiCommand::ChartReplaceData { .. }
            | UiCommand::ChartClearData { .. } => {}
        }
    }

    pub(in crate::gpui_backend) fn dispatch_chart_command(
        &self,
        node_id: NativeObjectId,
        cx: &mut Context<Self>,
        apply: impl FnOnce(&mut RasterChartState) + 'static,
    ) {
        let owner = self.owners.borrow().node_owners.get(&node_id).cloned();
        let Some(owner) = owner else {
            logger::warn(format!("Chart command target {:?} has no owner", node_id));
            return;
        };
        owner.update(cx, move |owner, cx| {
            let Some(chart_state) = owner.chart_state.as_mut() else {
                logger::warn(format!("Chart command target {:?} is not a chart", node_id));
                return;
            };
            apply(chart_state);
            cx.notify();
        });
    }

    fn drain_commits(&mut self) -> ApplyOutcome {
        let mut outcome = ApplyOutcome::new();
        let batches = self.native_binding.drain_commits();
        for batch in batches {
            match self.tree.borrow_mut().apply_batch(batch) {
                Ok(batch_outcome) => {
                    outcome.merge(batch_outcome);
                }
                Err(error) => {
                    logger::error(format!("failed to apply retained tree batch: {error}"))
                }
            }
        }
        outcome
    }

    fn drain_commits_and_notify(&mut self, cx: &mut Context<Self>) -> bool {
        drain_bridge_ingress(self, cx);
        let outcome = self.drain_commits();
        let has_dirty = !outcome.is_clean();
        let theme_changed = self.sync_config_provider_theme(cx);
        self.ensure_owner_entities(cx);
        self.notify_dirty_owners(outcome, cx);
        has_dirty || theme_changed
    }

    fn sync_dialog_before_layer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let open_dialogs = open_dialog_ids(&self.tree, self.surface_id);
        if open_dialogs.len() > 1 {
            logger::warn("multiple open Dialog nodes found; only the first one is rendered");
        }

        if let Some(node_id) = open_dialogs.first().copied() {
            let node = self.tree.borrow().node(node_id).cloned();
            if let Some(node) = node {
                self.dialog_state.sync_from_node(
                    &node,
                    true,
                    DialogRenderContext {
                        tree: self.tree.clone(),
                        owners: self.owners.clone(),
                        perf: self.perf.clone(),
                        bridge: self.bridge.clone(),
                        root: cx.entity().downgrade(),
                    },
                    window,
                    cx,
                );
            }
        } else {
            self.dialog_state.sync_closed(window, cx);
        }
    }

    fn sync_alert_before_layer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let open_alerts = open_alert_ids(&self.tree, self.surface_id);
        if open_alerts.len() > 1 {
            logger::warn("multiple open Alert nodes found; only the first one is rendered");
        }

        if let Some(node_id) = open_alerts.first().copied() {
            let node = self.tree.borrow().node(node_id).cloned();
            if let Some(node) = node {
                self.alert_state.sync_from_node(
                    &node,
                    true,
                    AlertRenderContext {
                        tree: self.tree.clone(),
                        owners: self.owners.clone(),
                        perf: self.perf.clone(),
                        bridge: self.bridge.clone(),
                        root: cx.entity().downgrade(),
                    },
                    window,
                    cx,
                );
            }
        } else {
            self.alert_state.sync_closed(window, cx);
        }
    }

    fn sync_sheet_before_layer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let open_sheets = open_sheet_ids(&self.tree, self.surface_id);
        if open_sheets.len() > 1 {
            logger::warn("multiple open Sheet nodes found; only the first one is rendered");
        }

        if let Some(node_id) = open_sheets.first().copied() {
            let node = self.tree.borrow().node(node_id).cloned();
            if let Some(node) = node {
                self.sheet_state.sync_from_node(
                    &node,
                    true,
                    SheetRenderContext {
                        tree: self.tree.clone(),
                        owners: self.owners.clone(),
                        perf: self.perf.clone(),
                        bridge: self.bridge.clone(),
                        root: cx.entity().downgrade(),
                    },
                    window,
                    cx,
                );
            }
        } else {
            self.sheet_state.sync_closed(window, cx);
        }
    }

    fn sync_config_provider_theme(&mut self, cx: &mut App) -> bool {
        let snapshot = {
            let tree = self.tree.borrow();
            find_config_provider_theme(&tree, self.surface_id)
        };
        if snapshot == self.applied_theme {
            return false;
        }
        match &snapshot {
            Some(snapshot) => apply_theme_snapshot(snapshot, cx),
            None if self.applied_theme.is_some() => reset_theme_snapshot(cx),
            None => {}
        }
        self.native_binding
            .set_theme_snapshot_json(theme_snapshot_json(cx));
        emit_runtime_event(&self.bridge, "themechange", NodeValue::Null);
        self.applied_theme = snapshot;
        true
    }

    fn ensure_owner_entities(&mut self, cx: &mut Context<Self>) {
        let root = cx.weak_entity();
        let mut owners = self.owners.borrow_mut();

        owners.ensure_surface(
            self.surface_id,
            self.tree.clone(),
            self.owners.clone(),
            self.perf.clone(),
            self.bridge.clone(),
            root.clone(),
            cx,
        );

        let live_owner_ids = self
            .tree
            .borrow()
            .owner_node_ids()
            .into_iter()
            .collect::<BTreeSet<_>>();
        owners
            .node_owners
            .retain(|id, _| live_owner_ids.contains(id));
        for id in live_owner_ids {
            owners.ensure_node(
                id,
                self.tree.clone(),
                self.owners.clone(),
                self.perf.clone(),
                self.bridge.clone(),
                root.clone(),
                cx,
            );
        }
    }

    pub(in crate::gpui_backend) fn notify_all_owners(&self, cx: &mut Context<Self>) {
        cx.notify();
        let owners = self.owners.borrow();
        for entity in owners.surface_owners.values() {
            entity.update(cx, |_, cx| cx.notify());
        }
        for entity in owners.node_owners.values() {
            entity.update(cx, |_, cx| cx.notify());
        }
    }

    fn notify_dirty_owners(&self, outcome: ApplyOutcome, cx: &mut Context<Self>) {
        if outcome.is_clean() {
            return;
        }
        let owners = self.owners.borrow();
        for owner in outcome.dirty_owners() {
            match owner {
                OwnerId::Surface(surface_id) => {
                    if let Some(entity) = owners.surface_owners.get(&surface_id) {
                        entity.update(cx, |_, cx| cx.notify());
                    } else {
                        logger::error(format!("missing surface owner for {:?}", surface_id));
                    }
                }
                OwnerId::Node(node_id) => {
                    if let Some(entity) = owners.node_owners.get(&node_id) {
                        if let Some(expires_at) = self.perf.borrow_mut().record_dirty(node_id) {
                            schedule_perf_clear(entity, expires_at, cx);
                        }
                        entity.update(cx, |_, cx| cx.notify());
                    } else {
                        logger::error(format!("missing node owner for {:?}", node_id));
                    }
                }
            }
        }
    }
}

impl Render for RasterRootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let assets = self.bridge.assets();
        with_render_assets(assets, || {
            self.render_inner(window, cx)
        })
    }
}

impl RasterRootView {
    fn render_inner(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        drain_bridge_ingress(self, cx);
        self.drain_commits();
        self.sync_config_provider_theme(cx);
        self.ensure_owner_entities(cx);
        self.sync_sheet_before_layer(window, cx);
        self.sync_dialog_before_layer(window, cx);
        self.sync_alert_before_layer(window, cx);
        let sheet_layer = gpui_component::Root::render_sheet_layer(window, cx);
        let dialog_layer = gpui_component::Root::render_dialog_layer(window, cx);
        let background = cx.theme().background;
        let mut element = div()
            .size_full()
            .relative()
            .bg(background)
            .flex()
            .flex_col()
            .gap_2();
        let surface_owner = self
            .owners
            .borrow()
            .surface_owners
            .get(&self.surface_id)
            .cloned();
        if let Some(surface_owner) = surface_owner {
            element = element.child(surface_owner);
        } else {
            element = element.child("Raster GPUI backend is waiting for JS commits.");
        }
        element
            .when_some(sheet_layer, |this, layer| this.child(layer))
            .when_some(dialog_layer, |this, layer| this.child(layer))
            .child(self.notification_center.clone())
    }
}

pub(in crate::gpui_backend) struct OwnerRegistry {
    surface_owners: BTreeMap<SurfaceId, Entity<SurfaceOwnerView>>,
    node_owners: BTreeMap<NativeObjectId, Entity<NodeOwnerView>>,
}

impl OwnerRegistry {
    fn new() -> Self {
        Self {
            surface_owners: BTreeMap::new(),
            node_owners: BTreeMap::new(),
        }
    }

    fn ensure_surface(
        &mut self,
        surface_id: SurfaceId,
        tree: Rc<RefCell<RetainedTree>>,
        owners: Rc<RefCell<OwnerRegistry>>,
        perf: Rc<RefCell<PerfMonitor>>,
        bridge: SharedBridgeState,
        root: WeakEntity<RasterRootView>,
        cx: &mut Context<RasterRootView>,
    ) {
        self.surface_owners.entry(surface_id).or_insert_with(|| {
            cx.new(|_| SurfaceOwnerView {
                surface_id,
                tree,
                owners,
                perf,
                bridge,
                root,
            })
        });
    }

    fn ensure_node(
        &mut self,
        node_id: NativeObjectId,
        tree: Rc<RefCell<RetainedTree>>,
        owners: Rc<RefCell<OwnerRegistry>>,
        perf: Rc<RefCell<PerfMonitor>>,
        bridge: SharedBridgeState,
        root: WeakEntity<RasterRootView>,
        cx: &mut Context<RasterRootView>,
    ) {
        self.node_owners.entry(node_id).or_insert_with(|| {
            cx.new(|_| NodeOwnerView {
                node_id,
                tree,
                owners,
                perf,
                input_state: None,
                select_state: None,
                color_picker_state: None,
                date_picker_state: None,
                time_picker_state: None,
                slider_state: None,
                chart_state: None,
                virtual_list_state: None,
                bridge,
                root,
            })
        });
    }
}

struct SurfaceOwnerView {
    surface_id: SurfaceId,
    tree: Rc<RefCell<RetainedTree>>,
    owners: Rc<RefCell<OwnerRegistry>>,
    perf: Rc<RefCell<PerfMonitor>>,
    bridge: SharedBridgeState,
    root: WeakEntity<RasterRootView>,
}

impl Render for SurfaceOwnerView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        with_render_assets(self.bridge.assets(), || {
            let roots = self
                .tree
                .borrow()
                .surface(self.surface_id)
                .map(|surface| surface.roots.clone())
                .unwrap_or_default();
            let mut element = div().size_full().flex().flex_col().gap_2();
            for root in roots {
                element = append_surface_child(
                    element,
                    root,
                    &self.tree,
                    &self.owners,
                    &self.perf,
                    self.bridge.clone(),
                    self.root.clone(),
                );
            }
            element
        })
    }
}

fn append_surface_child(
    mut element: gpui::Div,
    id: NativeObjectId,
    tree: &Rc<RefCell<RetainedTree>>,
    owners: &Rc<RefCell<OwnerRegistry>>,
    perf: &Rc<RefCell<PerfMonitor>>,
    bridge: SharedBridgeState,
    root: WeakEntity<RasterRootView>,
) -> gpui::Div {
    let config_provider_children = {
        let tree_ref = tree.borrow();
        tree_ref
            .node(id)
            .filter(|node| is_config_provider_node(node))
            .map(|node| node.children.clone())
    };

    if let Some(children) = config_provider_children {
        for child in children {
            element = append_surface_child(
                element,
                child,
                tree,
                owners,
                perf,
                bridge.clone(),
                root.clone(),
            );
        }
        return element;
    }

    if is_layoutless_node(id, tree) {
        return element;
    }

    element.child(render_node_child(
        id,
        tree,
        owners,
        perf,
        bridge,
        root,
    ))
}

fn is_layoutless_node(id: NativeObjectId, tree: &Rc<RefCell<RetainedTree>>) -> bool {
    tree.borrow()
        .node(id)
        .is_some_and(|node| is_alert_node(node) || is_dialog_node(node) || is_sheet_node(node))
}

fn open_alert_ids(tree: &Rc<RefCell<RetainedTree>>, surface_id: SurfaceId) -> Vec<NativeObjectId> {
    let tree = tree.borrow();
    let roots = tree
        .surface(surface_id)
        .map(|surface| surface.roots.clone())
        .unwrap_or_default();
    let mut ids = Vec::new();
    for root in roots {
        collect_open_alert_ids(&tree, root, &mut ids);
    }
    ids
}

fn collect_open_alert_ids(tree: &RetainedTree, id: NativeObjectId, ids: &mut Vec<NativeObjectId>) {
    let Some(node) = tree.node(id) else {
        return;
    };
    if is_alert_node(node) && crate::gpui_backend::components::alert::alert_open(node) {
        ids.push(id);
    }
    for child in &node.children {
        collect_open_alert_ids(tree, *child, ids);
    }
}

fn open_sheet_ids(tree: &Rc<RefCell<RetainedTree>>, surface_id: SurfaceId) -> Vec<NativeObjectId> {
    let tree = tree.borrow();
    let roots = tree
        .surface(surface_id)
        .map(|surface| surface.roots.clone())
        .unwrap_or_default();
    let mut ids = Vec::new();
    for root in roots {
        collect_open_sheet_ids(&tree, root, &mut ids);
    }
    ids
}

fn collect_open_sheet_ids(tree: &RetainedTree, id: NativeObjectId, ids: &mut Vec<NativeObjectId>) {
    let Some(node) = tree.node(id) else {
        return;
    };
    if is_sheet_node(node) && crate::gpui_backend::components::sheet::sheet_open(node) {
        ids.push(id);
    }
    for child in &node.children {
        collect_open_sheet_ids(tree, *child, ids);
    }
}

fn open_dialog_ids(tree: &Rc<RefCell<RetainedTree>>, surface_id: SurfaceId) -> Vec<NativeObjectId> {
    let tree = tree.borrow();
    let roots = tree
        .surface(surface_id)
        .map(|surface| surface.roots.clone())
        .unwrap_or_default();
    let mut ids = Vec::new();
    for root in roots {
        collect_open_dialog_ids(&tree, root, &mut ids);
    }
    ids
}

fn collect_open_dialog_ids(tree: &RetainedTree, id: NativeObjectId, ids: &mut Vec<NativeObjectId>) {
    let Some(node) = tree.node(id) else {
        return;
    };
    if is_dialog_node(node) && crate::gpui_backend::components::dialog::dialog_open(node) {
        ids.push(id);
    }
    for child in &node.children {
        collect_open_dialog_ids(tree, *child, ids);
    }
}

pub(super) struct NodeOwnerView {
    node_id: NativeObjectId,
    tree: Rc<RefCell<RetainedTree>>,
    owners: Rc<RefCell<OwnerRegistry>>,
    perf: Rc<RefCell<PerfMonitor>>,
    input_state: Option<RasterInputState>,
    select_state: Option<RasterSelectState>,
    color_picker_state: Option<RasterColorPickerState>,
    date_picker_state: Option<RasterDatePickerState>,
    time_picker_state: Option<RasterTimePickerState>,
    slider_state: Option<RasterSliderState>,
    chart_state: Option<RasterChartState>,
    virtual_list_state: Option<RasterVirtualListState>,
    bridge: SharedBridgeState,
    root: WeakEntity<RasterRootView>,
}

impl Render for NodeOwnerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        with_render_assets(self.bridge.assets(), || self.render_impl(window, cx))
    }
}

impl NodeOwnerView {
    fn render_impl(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let highlight = self.perf.borrow_mut().highlight(self.node_id);
        if let Some(node) = self.tree.borrow().node(self.node_id).cloned() {
            if is_text_control_node(&node) {
                self.select_state = None;
                self.color_picker_state = None;
                self.date_picker_state = None;
                self.time_picker_state = None;
                self.slider_state = None;
                self.chart_state = None;
                self.virtual_list_state = None;
                let dispatcher = event_dispatcher(self.bridge.clone(), self.root.clone());
                if self
                    .input_state
                    .as_ref()
                    .is_none_or(|state| !state.matches_config(&node))
                {
                    self.input_state = Some(RasterInputState::new(&node, dispatcher, window, cx));
                }
                if let Some(input_state) = &mut self.input_state {
                    input_state.sync_from_node(&node, window, cx);
                    if let Some(input) = render_input_from_node(&node, input_state.input()) {
                        return decorate_refresh(input, highlight);
                    }
                }
            } else if is_select_node(&node) {
                self.input_state = None;
                self.color_picker_state = None;
                self.date_picker_state = None;
                self.time_picker_state = None;
                self.slider_state = None;
                self.chart_state = None;
                self.virtual_list_state = None;
                if self
                    .select_state
                    .as_ref()
                    .is_none_or(|state| !state.matches_config(&node))
                {
                    self.select_state = Some(RasterSelectState::new(
                        &node,
                        self.bridge.clone(),
                        window,
                        cx,
                    ));
                }
                if let Some(select_state) = &mut self.select_state {
                    select_state.sync_from_node(&node, window, cx);
                    if let Some(select) = render_select_from_node(&node, select_state.select()) {
                        return decorate_refresh(select, highlight);
                    }
                }
            } else if is_color_picker_node(&node) {
                self.input_state = None;
                self.select_state = None;
                self.date_picker_state = None;
                self.time_picker_state = None;
                self.slider_state = None;
                self.chart_state = None;
                self.virtual_list_state = None;
                if self
                    .color_picker_state
                    .as_ref()
                    .is_none_or(|state| !state.matches_config(&node))
                {
                    self.color_picker_state = Some(RasterColorPickerState::new(
                        &node,
                        self.bridge.clone(),
                        window,
                        cx,
                    ));
                }
                if let Some(color_picker_state) = &mut self.color_picker_state {
                    color_picker_state.sync_from_node(&node, window, cx);
                    if let Some(color_picker) =
                        render_color_picker_from_node(&node, color_picker_state.color_picker())
                    {
                        return decorate_refresh(color_picker, highlight);
                    }
                }
            } else if is_date_picker_node(&node) {
                self.input_state = None;
                self.select_state = None;
                self.color_picker_state = None;
                self.time_picker_state = None;
                self.slider_state = None;
                self.chart_state = None;
                self.virtual_list_state = None;
                if self
                    .date_picker_state
                    .as_ref()
                    .is_none_or(|state| !state.matches_config(&node))
                {
                    self.date_picker_state = Some(RasterDatePickerState::new(
                        &node,
                        self.bridge.clone(),
                        window,
                        cx,
                    ));
                }
                if let Some(date_picker_state) = &mut self.date_picker_state {
                    date_picker_state.sync_from_node(&node, window, cx);
                    if let Some(date_picker) =
                        render_date_picker_from_node(&node, date_picker_state.date_picker())
                    {
                        return decorate_refresh(date_picker, highlight);
                    }
                }
            } else if is_time_picker_node(&node) {
                self.input_state = None;
                self.select_state = None;
                self.color_picker_state = None;
                self.date_picker_state = None;
                self.slider_state = None;
                self.chart_state = None;
                self.virtual_list_state = None;
                if self
                    .time_picker_state
                    .as_ref()
                    .is_none_or(|state| !state.matches_config(&node))
                {
                    self.time_picker_state = Some(RasterTimePickerState::new(
                        &node,
                        self.bridge.clone(),
                        window,
                        cx,
                    ));
                }
                if let Some(time_picker_state) = &mut self.time_picker_state {
                    time_picker_state.sync_from_node(&node, window, cx);
                    if let Some(time_picker) =
                        render_time_picker_from_node(&node, time_picker_state.time_picker())
                    {
                        return decorate_refresh(time_picker, highlight);
                    }
                }
            } else if is_slider_node(&node) {
                self.input_state = None;
                self.select_state = None;
                self.color_picker_state = None;
                self.date_picker_state = None;
                self.time_picker_state = None;
                self.chart_state = None;
                self.virtual_list_state = None;
                if self
                    .slider_state
                    .as_ref()
                    .is_none_or(|state| !state.matches_config(&node))
                {
                    self.slider_state = Some(RasterSliderState::new(
                        &node,
                        self.bridge.clone(),
                        cx,
                    ));
                }
                if let Some(slider_state) = &mut self.slider_state {
                    slider_state.sync_from_node(&node, window, cx);
                    if let Some(slider) = render_slider_from_node(&node, slider_state.slider()) {
                        return decorate_refresh(slider, highlight);
                    }
                }
            } else if is_virtual_list_node(&node) {
                self.input_state = None;
                self.select_state = None;
                self.color_picker_state = None;
                self.date_picker_state = None;
                self.time_picker_state = None;
                self.slider_state = None;
                self.chart_state = None;
                if self.virtual_list_state.is_none() {
                    self.virtual_list_state = Some(RasterVirtualListState::new(&node));
                }
                if let Some(virtual_list_state) = &mut self.virtual_list_state {
                    virtual_list_state.sync_from_node(&node);
                    if let Some(virtual_list) = render_virtual_list_from_node(
                        &node,
                        virtual_list_state,
                        cx.entity(),
                        self.tree.clone(),
                        self.owners.clone(),
                        self.perf.clone(),
                        self.bridge.clone(),
                        self.root.clone(),
                    ) {
                        return decorate_refresh(virtual_list, highlight);
                    }
                }
            } else if is_chart_node(&node) {
                self.input_state = None;
                self.select_state = None;
                self.color_picker_state = None;
                self.date_picker_state = None;
                self.time_picker_state = None;
                self.slider_state = None;
                self.virtual_list_state = None;
                if self
                    .chart_state
                    .as_ref()
                    .is_none_or(|state| !state.matches_node(&node))
                {
                    self.chart_state = Some(RasterChartState::new(&node));
                }
                if let Some(chart_state) = &mut self.chart_state {
                    chart_state.sync_from_node(&node);
                    if let Some(chart) = render_chart_from_node(&node, chart_state) {
                        return decorate_refresh(chart, highlight);
                    }
                }
            } else {
                self.input_state = None;
                self.select_state = None;
                self.color_picker_state = None;
                self.date_picker_state = None;
                self.time_picker_state = None;
                self.slider_state = None;
                self.chart_state = None;
                self.virtual_list_state = None;
                if is_button_group_node(&node) {
                    if let Some(button_group) = render_button_group_from_node(
                        &node,
                        &self.tree,
                        self.bridge.clone(),
                    ) {
                        return decorate_refresh(button_group, highlight);
                    }
                }
                let child_text = child_text(&self.tree, &node.children);
                if is_checkbox_node(&node) {
                    let dispatcher =
                        event_dispatcher(self.bridge.clone(), self.root.clone());
                    if let Some(checkbox) = render_checkbox_from_node(
                        &node,
                        child_text.iter().map(String::as_str),
                        dispatcher,
                    ) {
                        return decorate_refresh(checkbox, highlight);
                    }
                }
                if is_switch_node(&node) {
                    let dispatcher =
                        event_dispatcher(self.bridge.clone(), self.root.clone());
                    if let Some(switch) = render_switch_from_node(
                        &node,
                        child_text.iter().map(String::as_str),
                        dispatcher,
                    ) {
                        return decorate_refresh(switch, highlight);
                    }
                }
                if is_radio_group_node(&node) {
                    if let Some(radio_group) = render_radio_group_from_node(
                        &node,
                        &self.tree,
                        self.bridge.clone(),
                    ) {
                        return decorate_refresh(radio_group, highlight);
                    }
                }
                if is_radio_node(&node) {
                    let dispatcher =
                        event_dispatcher(self.bridge.clone(), self.root.clone());
                    if let Some(radio) = render_radio_from_node(
                        &node,
                        child_text.iter().map(String::as_str),
                        dispatcher,
                    ) {
                        return decorate_refresh(radio, highlight);
                    }
                }
                if is_avatar_group_node(&node) {
                    if let Some(avatar_group) = render_avatar_group_from_node(&node, &self.tree) {
                        return decorate_refresh(avatar_group, highlight);
                    }
                }
                if is_avatar_node(&node) {
                    if let Some(avatar) = render_avatar_from_node(&node) {
                        return decorate_refresh(avatar, highlight);
                    }
                }
                if is_icon_node(&node) {
                    if let Some(icon) = render_icon_from_node(&node) {
                        return decorate_refresh(icon, highlight);
                    }
                }
                if is_tab_bar_node(&node) {
                    if let Some(tab_bar) = render_tab_bar_from_node(
                        &node,
                        &self.tree,
                        &self.owners,
                        &self.perf,
                        self.bridge.clone(),
                        self.root.clone(),
                    ) {
                        return decorate_refresh(tab_bar, highlight);
                    }
                }
                if is_tab_node(&node) {
                    if let Some(tab) = render_tab_from_node(
                        &node,
                        child_text.iter().map(String::as_str),
                        self.bridge.clone(),
                    ) {
                        return decorate_refresh(tab, highlight);
                    }
                }
                if let Some(element) = render_text_label_or_rich_text_from_node(
                    &node,
                    child_text.iter().map(String::as_str),
                    window,
                    cx,
                ) {
                    return decorate_refresh(element, highlight);
                }
            }
        }

        decorate_refresh(
            render_node_inline(
                self.node_id,
                &self.tree,
                &self.owners,
                &self.perf,
                self.bridge.clone(),
                self.root.clone(),
            ),
            highlight,
        )
    }
}

fn schedule_perf_clear(
    entity: &Entity<NodeOwnerView>,
    expires_at: Instant,
    cx: &mut Context<RasterRootView>,
) {
    let weak = entity.downgrade();
    cx.spawn(async move |_, cx| {
        let now = Instant::now();
        if expires_at > now {
            cx.background_executor().timer(expires_at - now).await;
        }
        if let Some(entity) = weak.upgrade() {
            entity.update(cx, |_, cx| cx.notify());
        }
    })
    .detach();
}

pub(in crate::gpui_backend) fn render_node_child(
    id: NativeObjectId,
    tree: &Rc<RefCell<RetainedTree>>,
    owners: &Rc<RefCell<OwnerRegistry>>,
    perf: &Rc<RefCell<PerfMonitor>>,
    bridge: SharedBridgeState,
    root: WeakEntity<RasterRootView>,
) -> AnyElement {
    if let Some(owner) = owners.borrow().node_owners.get(&id).cloned() {
        return owner.into_any_element();
    }
    render_node_inline(id, tree, owners, perf, bridge, root)
}

fn render_node_inline(
    id: NativeObjectId,
    tree: &Rc<RefCell<RetainedTree>>,
    owners: &Rc<RefCell<OwnerRegistry>>,
    perf: &Rc<RefCell<PerfMonitor>>,
    bridge: SharedBridgeState,
    root: WeakEntity<RasterRootView>,
) -> AnyElement {
    let Some(node) = tree.borrow().node(id).cloned() else {
        return div().child("Missing retained node").into_any_element();
    };
    let model = node.render_model.clone();
    let children = node.children.clone();

    match model {
        RenderModel::View(view) => {
            if has_scroll_overflow(&view.style) {
                let mut element = apply_view_style(
                    div().id(("raster-scroll-view", id.0)).flex().flex_col(),
                    &view.style,
                )
                .when(has_horizontal_scroll_overflow(&view.style), |this| {
                    this.overflow_x_scroll()
                })
                .when(has_vertical_scroll_overflow(&view.style), |this| {
                    this.overflow_y_scroll()
                })
                .flex_1();
                if let Some(handler_id) = event_handler(&node, "onClick") {
                    let bridge = bridge.clone();
                    element = element.on_click(move |_, _, _| {
                        emit_handler_invoke(&bridge, handler_id, NodeValue::String(String::new()));
                    });
                }
                for child in children {
                    if is_layoutless_node(child, tree) {
                        continue;
                    }
                    element = element.child(render_node_child(
                        child,
                        tree,
                        owners,
                        perf,
                        bridge.clone(),
                        root.clone(),
                    ));
                }
                return element.into_any_element();
            } else {
                let mut element = apply_view_style(
                    div().id(("raster-view", id.0)).flex().flex_col(),
                    &view.style,
                );
                if let Some(handler_id) = event_handler(&node, "onClick") {
                    let bridge = bridge.clone();
                    element = element.on_click(move |_, _, _| {
                        emit_handler_invoke(&bridge, handler_id, NodeValue::String(String::new()));
                    });
                }
                for child in children {
                    if is_layoutless_node(child, tree) {
                        continue;
                    }
                    element = element.child(render_node_child(
                        child,
                        tree,
                        owners,
                        perf,
                        bridge.clone(),
                        root.clone(),
                    ));
                }
                return element.into_any_element();
            }
        }
        RenderModel::Label(label) => apply_style(div(), &label.style)
            .child(label.text)
            .into_any_element(),
        RenderModel::Widget(widget) => {
            let child_text = child_text(tree, &children);
            let dispatcher = event_dispatcher(bridge.clone(), root.clone());
            if is_config_provider_node(&node) {
                let mut element = apply_style(div(), &widget.style);
                for child in children {
                    if is_layoutless_node(child, tree) {
                        continue;
                    }
                    element = element.child(render_node_child(
                        child,
                        tree,
                        owners,
                        perf,
                        bridge.clone(),
                        root.clone(),
                    ));
                }
                return element.into_any_element();
            }
            if is_sheet_node(&node) {
                return Empty.into_any_element();
            }
            if is_dialog_node(&node) {
                return Empty.into_any_element();
            }
            if is_alert_node(&node) {
                return Empty.into_any_element();
            }
            if is_form_node(&node) {
                if let Some(form) = render_form_from_node(
                    &node,
                    tree,
                    owners,
                    perf,
                    bridge.clone(),
                    root.clone(),
                ) {
                    return form;
                }
            }
            if is_field_node(&node) {
                if let Some(field) = render_field_from_node(
                    &node,
                    tree,
                    owners,
                    perf,
                    bridge.clone(),
                    root.clone(),
                ) {
                    return field;
                }
                return Empty.into_any_element();
            }
            if let Some(button) = render_button_from_node(
                &node,
                child_text.iter().map(String::as_str),
                dispatcher.clone(),
            ) {
                return button;
            }
            if is_button_group_node(&node) {
                if let Some(button_group) =
                    render_button_group_from_node(&node, tree, bridge.clone())
                {
                    return button_group;
                }
            }
            if is_checkbox_node(&node) {
                if let Some(checkbox) = render_checkbox_from_node(
                    &node,
                    child_text.iter().map(String::as_str),
                    dispatcher.clone(),
                ) {
                    return checkbox;
                }
            }
            if is_switch_node(&node) {
                if let Some(switch) = render_switch_from_node(
                    &node,
                    child_text.iter().map(String::as_str),
                    dispatcher.clone(),
                ) {
                    return switch;
                }
            }
            if is_radio_group_node(&node) {
                if let Some(radio_group) =
                    render_radio_group_from_node(&node, tree, bridge.clone())
                {
                    return radio_group;
                }
            }
            if is_radio_node(&node) {
                if let Some(radio) =
                    render_radio_from_node(&node, child_text.iter().map(String::as_str), dispatcher)
                {
                    return radio;
                }
            }
            if is_avatar_group_node(&node) {
                if let Some(avatar_group) = render_avatar_group_from_node(&node, tree) {
                    return avatar_group;
                }
            }
            if is_avatar_node(&node) {
                if let Some(avatar) = render_avatar_from_node(&node) {
                    return avatar;
                }
            }
            if is_icon_node(&node) {
                if let Some(icon) = render_icon_from_node(&node) {
                    return icon;
                }
            }
            if is_tab_bar_node(&node) {
                if let Some(tab_bar) = render_tab_bar_from_node(
                    &node,
                    tree,
                    owners,
                    perf,
                    bridge.clone(),
                    root.clone(),
                ) {
                    return tab_bar;
                }
            }
            if is_tab_node(&node) {
                if let Some(tab) = render_tab_from_node(
                    &node,
                    child_text.iter().map(String::as_str),
                    bridge.clone(),
                ) {
                    return tab;
                }
            }
            if is_select_node(&node) {
                return apply_style(div(), &widget.style)
                    .child("Unsupported Select owner")
                    .into_any_element();
            }
            if is_color_picker_node(&node) {
                return apply_style(div(), &widget.style)
                    .child("Unsupported ColorPicker owner")
                    .into_any_element();
            }
            if is_date_picker_node(&node) {
                return apply_style(div(), &widget.style)
                    .child("Unsupported DatePicker owner")
                    .into_any_element();
            }
            if is_chart_node(&node) {
                return apply_style(div(), &widget.style)
                    .child("Unsupported Chart owner")
                    .into_any_element();
            }
            if node.kind == RetainedNodeKind::Widget && node.component_name() == "Label" {
                if let Some(label) = render_text_label_element_from_node(
                    &node,
                    child_text.iter().map(String::as_str),
                ) {
                    return label;
                }
            }
            apply_style(div(), &widget.style)
                .child(format!("Unsupported widget: {}", node.component_name()))
                .into_any_element()
        }
        RenderModel::Fragment => {
            let mut element = div();
            for child in children {
                if is_layoutless_node(child, tree) {
                    continue;
                }
                element = element.child(render_node_child(
                    child,
                    tree,
                    owners,
                    perf,
                    bridge.clone(),
                    root.clone(),
                ));
            }
            element.into_any_element()
        }
    }
}

fn child_text(tree: &Rc<RefCell<RetainedTree>>, children: &[NativeObjectId]) -> Vec<String> {
    let tree = tree.borrow();
    children
        .iter()
        .filter_map(|child| tree.node(*child))
        .filter_map(|child| child.payload.text.clone())
        .collect()
}

fn event_dispatcher(
    bridge: SharedBridgeState,
    _root: WeakEntity<RasterRootView>,
) -> BridgeEventDispatch {
    crate::bridge::bridge_event_dispatcher(bridge)
}

fn install_commit_wake(native_binding: NativeBindingState, cx: &mut Context<RasterRootView>) {
    let (sender, mut receiver) = mpsc::unbounded::<()>();
    let pending = Arc::new(AtomicBool::new(false));
    let wake: Arc<dyn WakeSignal> = Arc::new(GpuiCommitWake {
        sender,
        pending: pending.clone(),
    });
    native_binding.set_commit_wake(wake.clone());
    native_binding.bridge().set_host_wake(wake);

    cx.spawn(async move |root, cx| {
        while receiver.next().await.is_some() {
            pending.store(false, Ordering::SeqCst);
            if let Err(error) = root.update(cx, |view, cx| {
                view.drain_commits_and_notify(cx);
            }) {
                logger::error(format!("failed to drain JS commit after wake: {error}"));
                break;
            }
        }
    })
    .detach();
}

struct GpuiCommitWake {
    sender: mpsc::UnboundedSender<()>,
    pending: Arc<AtomicBool>,
}

impl WakeSignal for GpuiCommitWake {
    fn wake(&self) {
        if self.pending.swap(true, Ordering::SeqCst) {
            return;
        }
        if self.sender.unbounded_send(()).is_err() {
            self.pending.store(false, Ordering::SeqCst);
        }
    }
}
