use std::{cell::RefCell, rc::Rc};

use gpui::{
    AnyElement, Axis, Entity, IntoElement, ListSizingBehavior, Pixels, Size, Styled, WeakEntity,
    div, px, size,
};
use gpui_component::{VirtualListScrollHandle, h_virtual_list, v_virtual_list};

use crate::{
    common::{
        channel::{ChannelSender, RuntimeCommand},
        ids::NativeObjectId,
        mount::RetainedNodeKind,
    },
    gpui_backend::{
        app::{NodeOwnerView, OwnerRegistry, RasterRootView, render_node_child},
        components::helper::props::{component_props, number_prop, string_prop},
        perf::PerfMonitor,
        render_model::{model::RenderModel, style::apply_style},
        retained_tree::node::RetainedNode,
        retained_tree::tree::RetainedTree,
    },
};

pub(in crate::gpui_backend) struct RasterVirtualListState {
    model: RasterVirtualListModel,
    scroll_handle: VirtualListScrollHandle,
}

impl RasterVirtualListState {
    pub(in crate::gpui_backend) fn new(node: &RetainedNode) -> Self {
        Self {
            model: RasterVirtualListModel::from_node(node),
            scroll_handle: VirtualListScrollHandle::new(),
        }
    }

    pub(in crate::gpui_backend) fn sync_from_node(&mut self, node: &RetainedNode) {
        self.model = RasterVirtualListModel::from_node(node);
    }

    pub(in crate::gpui_backend) fn model(&self) -> &RasterVirtualListModel {
        &self.model
    }

    pub(in crate::gpui_backend) fn scroll_handle(&self) -> &VirtualListScrollHandle {
        &self.scroll_handle
    }
}

pub(in crate::gpui_backend) fn render_virtual_list_from_node(
    node: &RetainedNode,
    state: &RasterVirtualListState,
    owner: Entity<NodeOwnerView>,
    tree: Rc<RefCell<RetainedTree>>,
    owners: Rc<RefCell<OwnerRegistry>>,
    perf: Rc<RefCell<PerfMonitor>>,
    runtime_commands: ChannelSender<RuntimeCommand>,
    root: WeakEntity<RasterRootView>,
) -> Option<AnyElement> {
    if !is_virtual_list_node(node) {
        return None;
    }

    let RenderModel::Widget(widget) = &node.render_model else {
        return None;
    };

    let model = state.model().clone();
    let retained_children = node.children.clone();
    let item_sizes = model.item_sizes();
    let id = ("raster-virtual-list", node.id.0);

    let list = match model.axis {
        Axis::Horizontal => h_virtual_list(owner, id, item_sizes, move |_this, range, _, _| {
            render_visible_items(
                &model,
                &retained_children,
                &tree,
                &owners,
                &perf,
                runtime_commands.clone(),
                root.clone(),
                range,
            )
        }),
        Axis::Vertical => v_virtual_list(owner, id, item_sizes, move |_this, range, _, _| {
            render_visible_items(
                &model,
                &retained_children,
                &tree,
                &owners,
                &perf,
                runtime_commands.clone(),
                root.clone(),
                range,
            )
        }),
    }
    .track_scroll(state.scroll_handle())
    .with_sizing_behavior(ListSizingBehavior::Auto)
    .flex_shrink_0();

    Some(apply_style(list, &widget.style).into_any_element())
}

pub(in crate::gpui_backend) fn is_virtual_list_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "VirtualList"
}

#[derive(Clone)]
pub(in crate::gpui_backend) struct RasterVirtualListModel {
    axis: Axis,
    item_sizes: Rc<Vec<Size<Pixels>>>,
}

impl RasterVirtualListModel {
    pub(in crate::gpui_backend) fn from_node(node: &RetainedNode) -> Self {
        let props = component_props(node);
        let axis = string_prop(props, "axis")
            .as_deref()
            .map(parse_axis)
            .unwrap_or(Axis::Vertical);
        let item_size = number_prop(props, "itemSize").unwrap_or(32.0).max(1.0) as f32;
        let count = node.children.len();
        let item_sizes = Rc::new(
            (0..count)
                .map(|_| match axis {
                    Axis::Horizontal => size(px(item_size), px(0.0)),
                    Axis::Vertical => size(px(0.0), px(item_size)),
                })
                .collect(),
        );

        Self { axis, item_sizes }
    }

    fn item_sizes(&self) -> Rc<Vec<Size<Pixels>>> {
        self.item_sizes.clone()
    }
}

fn render_visible_items(
    _model: &RasterVirtualListModel,
    retained_children: &[NativeObjectId],
    tree: &Rc<RefCell<RetainedTree>>,
    owners: &Rc<RefCell<OwnerRegistry>>,
    perf: &Rc<RefCell<PerfMonitor>>,
    runtime_commands: ChannelSender<RuntimeCommand>,
    root: WeakEntity<RasterRootView>,
    range: std::ops::Range<usize>,
) -> Vec<AnyElement> {
    range
        .map(|index| {
            if let Some(child_id) = retained_children.get(index).copied() {
                return render_node_child(
                    child_id,
                    tree,
                    owners,
                    perf,
                    runtime_commands.clone(),
                    root.clone(),
                );
            }

            div().into_any_element()
        })
        .collect()
}

fn parse_axis(value: &str) -> Axis {
    match value {
        "horizontal" | "x" => Axis::Horizontal,
        _ => Axis::Vertical,
    }
}
