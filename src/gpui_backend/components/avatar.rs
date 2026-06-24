use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use gpui::{AlignSelf, AnyElement, ImageSource, IntoElement, Styled as _};
use gpui_component::{
    Sizable, Size,
    avatar::{Avatar, AvatarGroup},
};

use crate::{
    common::mount::{NodeValue, RetainedNodeKind},
    gpui_backend::{
        asset_context::current_render_image,
        components::helper::props::{
            bool_prop, component_props, display_value, number_prop, string_prop,
        },
        components::icon::icon_from_svg,
        render_model::{model::RenderModel, style::apply_style},
        retained_tree::{node::RetainedNode, tree::RetainedTree},
    },
};

pub(in crate::gpui_backend) fn render_avatar_from_node(node: &RetainedNode) -> Option<AnyElement> {
    if !is_avatar_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    Some(apply_style(build_avatar(node), &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn render_avatar_group_from_node(
    node: &RetainedNode,
    tree: &Rc<RefCell<RetainedTree>>,
) -> Option<AnyElement> {
    if !is_avatar_group_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let props = component_props(node);
    let size = string_prop(props, "size").map(|value| Size::from_str(&value));
    let mut group = AvatarGroup::new();
    if let Some(limit) = number_prop(props, "limit") {
        group = group.limit(limit.max(0.0) as usize);
    }
    if bool_prop(props, "ellipsis") == Some(true) {
        group = group.ellipsis();
    }
    if let Some(size) = size {
        group = group.with_size(size);
    }

    let tree_ref = tree.borrow();
    for avatar in avatar_children(node, &tree_ref) {
        group = group.child(avatar);
    }
    for avatar in avatar_specs(props) {
        group = group.child(avatar);
    }
    drop(tree_ref);

    let mut group = apply_style(group, &model.style);
    group.style().align_self.get_or_insert(AlignSelf::FlexStart);
    Some(group.into_any_element())
}

pub(in crate::gpui_backend) fn is_avatar_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Avatar"
}

pub(in crate::gpui_backend) fn is_avatar_group_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "AvatarGroup"
}

fn avatar_children(node: &RetainedNode, tree: &RetainedTree) -> Vec<Avatar> {
    node.children
        .iter()
        .filter_map(|child_id| tree.node(*child_id))
        .filter(|child| is_avatar_node(child))
        .map(build_avatar)
        .collect()
}

fn avatar_specs(props: &BTreeMap<String, NodeValue>) -> Vec<Avatar> {
    if let Some(NodeValue::Array(items)) = props.get("items").or_else(|| props.get("avatars")) {
        return items.iter().filter_map(avatar_from_spec).collect();
    }

    match props.get("names") {
        Some(NodeValue::Array(names)) => names
            .iter()
            .map(display_value)
            .map(|name| Avatar::new().name(name))
            .collect(),
        _ => Vec::new(),
    }
}

fn avatar_from_spec(value: &NodeValue) -> Option<Avatar> {
    match value {
        NodeValue::String(name) => Some(Avatar::new().name(name.clone())),
        NodeValue::Object(spec) => {
            let mut avatar = Avatar::new();
            if let Some(src) = string_prop(spec, "src") {
                avatar = avatar_from_src(&src, avatar);
            }
            if let Some(name) = string_prop(spec, "name") {
                avatar = avatar.name(name);
            }
            if let Some(icon) = string_prop(spec, "placeholderSvg")
                .or_else(|| string_prop(spec, "iconSvg"))
                .map(|svg| icon_from_svg(&svg))
            {
                avatar = avatar.placeholder(icon);
            }
            Some(avatar)
        }
        _ => None,
    }
}

fn build_avatar(node: &RetainedNode) -> Avatar {
    let props = component_props(node);
    let mut avatar = Avatar::new();
    if let Some(src) = string_prop(props, "src") {
        avatar = avatar_from_src(&src, avatar);
    }
    if let Some(name) = string_prop(props, "name") {
        avatar = avatar.name(name);
    }
    if let Some(icon) = string_prop(props, "placeholderSvg").map(|svg| icon_from_svg(&svg)) {
        avatar = avatar.placeholder(icon);
    }
    if let Some(size) = string_prop(props, "size").map(|value| Size::from_str(&value)) {
        avatar = avatar.with_size(size);
    }
    avatar
}

fn avatar_from_src(src: &str, avatar: Avatar) -> Avatar {
    if is_remote_uri(src) {
        if let Some(image) = current_render_image(src) {
            avatar.src(ImageSource::Render(image))
        } else {
            avatar
        }
    } else {
        avatar.src(src)
    }
}

fn is_remote_uri(src: &str) -> bool {
    src.starts_with("http://") || src.starts_with("https://")
}