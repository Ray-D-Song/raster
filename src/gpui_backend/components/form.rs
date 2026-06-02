use std::{cell::RefCell, rc::Rc};

use gpui::{
    AlignItems, AnyElement, Axis, IntoElement, ParentElement, Styled, WeakEntity, div, px, rems,
};
use gpui_component::{
    ActiveTheme, Sizable, Size,
    form::{Field as NativeField, Form as NativeForm, field},
};

use crate::{
    common::{
        channel::{ChannelSender, RuntimeCommand},
        ids::NativeObjectId,
        mount::RetainedNodeKind,
    },
    gpui_backend::{
        app::{OwnerRegistry, RasterRootView, render_node_child},
        components::helper::props::{
            bool_prop, component_props, display_value, number_prop, string_prop,
        },
        perf::PerfMonitor,
        render_model::{model::RenderModel, style::apply_style},
        retained_tree::{node::RetainedNode, tree::RetainedTree},
    },
};

pub(in crate::gpui_backend) fn render_form_from_node(
    node: &RetainedNode,
    tree: &Rc<RefCell<RetainedTree>>,
    owners: &Rc<RefCell<OwnerRegistry>>,
    perf: &Rc<RefCell<PerfMonitor>>,
    runtime_commands: ChannelSender<RuntimeCommand>,
    root: WeakEntity<RasterRootView>,
) -> Option<AnyElement> {
    if !is_form_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let props = component_props(node);
    let layout = string_prop(props, "layout")
        .or_else(|| string_prop(props, "axis"))
        .as_deref()
        .map(parse_axis)
        .unwrap_or(Axis::Vertical);
    let mut form = match layout {
        Axis::Horizontal => NativeForm::horizontal(),
        Axis::Vertical => NativeForm::vertical(),
    };

    if let Some(size) = string_prop(props, "size") {
        form = form.with_size(Size::from_str(&size));
    }
    if let Some(columns) = number_prop(props, "columns") {
        form = form.columns(columns.max(1.0) as usize);
    }
    if let Some(label_width) = number_prop(props, "labelWidth") {
        form = form.label_width(px(label_width.max(0.0) as f32));
    }
    if let Some(label_text_size) = number_prop(props, "labelTextSize") {
        form = form.label_text_size(rems(label_text_size.max(0.0) as f32));
    }

    for child in &node.children {
        let field = form_field_from_child(
            *child,
            tree,
            owners,
            perf,
            runtime_commands.clone(),
            root.clone(),
        );
        if let Some(field) = field {
            form = form.child(field);
        }
    }

    Some(apply_style(form, &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn render_field_from_node(
    node: &RetainedNode,
    tree: &Rc<RefCell<RetainedTree>>,
    owners: &Rc<RefCell<OwnerRegistry>>,
    perf: &Rc<RefCell<PerfMonitor>>,
    runtime_commands: ChannelSender<RuntimeCommand>,
    root: WeakEntity<RasterRootView>,
) -> Option<AnyElement> {
    if !is_field_node(node) || !field_visible(node) {
        return None;
    }

    Some(
        build_field_from_node(
            node,
            tree,
            owners,
            perf,
            runtime_commands.clone(),
            root.clone(),
        )
        .into_any_element(),
    )
}

pub(in crate::gpui_backend) fn is_form_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Form"
}

pub(in crate::gpui_backend) fn is_field_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Field"
}

fn form_field_from_child(
    child_id: NativeObjectId,
    tree: &Rc<RefCell<RetainedTree>>,
    owners: &Rc<RefCell<OwnerRegistry>>,
    perf: &Rc<RefCell<PerfMonitor>>,
    runtime_commands: ChannelSender<RuntimeCommand>,
    root: WeakEntity<RasterRootView>,
) -> Option<NativeField> {
    let child = tree.borrow().node(child_id).cloned()?;
    if is_field_node(&child) {
        field_visible(&child)
            .then(|| build_field_from_node(&child, tree, owners, perf, runtime_commands, root))
    } else {
        Some(field().label_indent(false).child(render_node_child(
            child_id,
            tree,
            owners,
            perf,
            runtime_commands,
            root,
        )))
    }
}

fn build_field_from_node(
    node: &RetainedNode,
    tree: &Rc<RefCell<RetainedTree>>,
    owners: &Rc<RefCell<OwnerRegistry>>,
    perf: &Rc<RefCell<PerfMonitor>>,
    runtime_commands: ChannelSender<RuntimeCommand>,
    root: WeakEntity<RasterRootView>,
) -> NativeField {
    let RenderModel::Widget(model) = &node.render_model else {
        return field();
    };

    let props = component_props(node);
    let mut native_field = field();

    if let Some(label) = props.get("label").map(display_value) {
        native_field = native_field.label(label);
    }
    if let Some(description) = props.get("description").map(display_value) {
        if bool_prop(props, "__validationError").unwrap_or(false) {
            native_field = native_field.description_fn(move |_window, cx| {
                div().text_color(cx.theme().danger).child(description.clone())
            });
        } else {
            native_field = native_field.description(description);
        }
    }
    if let Some(required) = bool_prop(props, "required") {
        native_field = native_field.required(required);
    }
    if let Some(label_indent) = bool_prop(props, "labelIndent") {
        native_field = native_field.label_indent(label_indent);
    }
    if let Some(align) = string_prop(props, "align").and_then(|value| parse_align(&value)) {
        native_field = match align {
            AlignItems::Start => native_field.items_start(),
            AlignItems::Center => native_field.items_center(),
            AlignItems::End => native_field.items_end(),
            _ => native_field,
        };
    }
    if let Some(col_span) = number_prop(props, "colSpan") {
        native_field = native_field.col_span(col_span.max(1.0) as u16);
    }
    if let Some(col_start) = number_prop(props, "colStart") {
        native_field = native_field.col_start(col_start as i16);
    }
    if let Some(col_end) = number_prop(props, "colEnd") {
        native_field = native_field.col_end(col_end as i16);
    }

    for child in &node.children {
        native_field = native_field.child(render_node_child(
            *child,
            tree,
            owners,
            perf,
            runtime_commands.clone(),
            root.clone(),
        ));
    }

    apply_style(native_field, &model.style)
}

fn field_visible(node: &RetainedNode) -> bool {
    bool_prop(component_props(node), "visible") != Some(false)
}

fn parse_axis(value: &str) -> Axis {
    match value {
        "horizontal" | "row" => Axis::Horizontal,
        _ => Axis::Vertical,
    }
}

fn parse_align(value: &str) -> Option<AlignItems> {
    Some(match value {
        "start" => AlignItems::Start,
        "center" => AlignItems::Center,
        "end" => AlignItems::End,
        _ => return None,
    })
}
