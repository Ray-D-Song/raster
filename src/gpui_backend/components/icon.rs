use gpui::{AnyElement, IntoElement, Radians, Styled};
use gpui_component::{Icon, IconName, Sizable, Size};

use crate::{
    common::mount::RetainedNodeKind,
    gpui_backend::{
        components::helper::props::{bool_prop, component_props, number_prop, string_prop},
        render_model::{
            model::RenderModel,
            style::{apply_style, parse_color},
        },
        retained_tree::node::RetainedNode,
    },
};

pub(in crate::gpui_backend) fn render_icon_from_node(node: &RetainedNode) -> Option<AnyElement> {
    if !is_icon_node(node) {
        return None;
    }

    let RenderModel::Widget(model) = &node.render_model else {
        return None;
    };

    let props = component_props(node);
    let mut icon = if bool_prop(props, "empty") == Some(true) {
        Icon::empty()
    } else if let Some(path) = string_prop(props, "path") {
        Icon::empty().path(path)
    } else if let Some(name) = string_prop(props, "name").or_else(|| string_prop(props, "icon")) {
        Icon::new(parse_icon_name(&name).unwrap_or(IconName::Info))
    } else {
        Icon::new(IconName::Info)
    };

    if let Some(size) = string_prop(props, "size").map(|value| Size::from_str(&value)) {
        icon = icon.with_size(size);
    }
    if let Some(rotation) = number_prop(props, "rotate") {
        icon = icon.rotate(Radians(rotation as f32));
    }
    if let Some(color) = string_prop(props, "color")
        .as_deref()
        .and_then(parse_color)
        .or(model.style.color)
    {
        icon = icon.text_color(color);
    }

    Some(apply_style(icon, &model.style).into_any_element())
}

pub(in crate::gpui_backend) fn is_icon_node(node: &RetainedNode) -> bool {
    node.kind == RetainedNodeKind::Widget && node.component_name() == "Icon"
}

pub(in crate::gpui_backend) fn parse_icon_name(name: &str) -> Option<IconName> {
    Some(match name {
        "a-large-small" => IconName::ALargeSmall,
        "alert" | "triangle-alert" | "warning" => IconName::TriangleAlert,
        "arrow-down" => IconName::ArrowDown,
        "arrow-left" => IconName::ArrowLeft,
        "arrow-right" => IconName::ArrowRight,
        "arrow-up" => IconName::ArrowUp,
        "asterisk" => IconName::Asterisk,
        "bell" => IconName::Bell,
        "book-open" => IconName::BookOpen,
        "bot" => IconName::Bot,
        "building" | "building-2" => IconName::Building2,
        "calendar" => IconName::Calendar,
        "case-sensitive" => IconName::CaseSensitive,
        "chart-pie" => IconName::ChartPie,
        "check" => IconName::Check,
        "chevron-down" => IconName::ChevronDown,
        "chevron-left" => IconName::ChevronLeft,
        "chevron-right" => IconName::ChevronRight,
        "chevron-up" => IconName::ChevronUp,
        "chevrons-up-down" => IconName::ChevronsUpDown,
        "circle-check" | "success" => IconName::CircleCheck,
        "circle-user" => IconName::CircleUser,
        "circle-x" | "error" => IconName::CircleX,
        "close" | "x" => IconName::Close,
        "copy" => IconName::Copy,
        "dash" => IconName::Dash,
        "delete" => IconName::Delete,
        "ellipsis" => IconName::Ellipsis,
        "ellipsis-vertical" => IconName::EllipsisVertical,
        "external-link" => IconName::ExternalLink,
        "eye" => IconName::Eye,
        "eye-off" => IconName::EyeOff,
        "file" => IconName::File,
        "folder" => IconName::Folder,
        "folder-closed" => IconName::FolderClosed,
        "folder-open" => IconName::FolderOpen,
        "frame" => IconName::Frame,
        "gallery-vertical-end" => IconName::GalleryVerticalEnd,
        "github" => IconName::Github,
        "globe" => IconName::Globe,
        "heart" => IconName::Heart,
        "heart-off" => IconName::HeartOff,
        "inbox" => IconName::Inbox,
        "info" => IconName::Info,
        "inspector" => IconName::Inspector,
        "layout-dashboard" => IconName::LayoutDashboard,
        "loader" => IconName::Loader,
        "loader-circle" => IconName::LoaderCircle,
        "map" => IconName::Map,
        "maximize" => IconName::Maximize,
        "menu" => IconName::Menu,
        "minimize" => IconName::Minimize,
        "minus" => IconName::Minus,
        "moon" => IconName::Moon,
        "palette" => IconName::Palette,
        "panel-bottom" => IconName::PanelBottom,
        "panel-bottom-open" => IconName::PanelBottomOpen,
        "panel-left" => IconName::PanelLeft,
        "panel-left-close" => IconName::PanelLeftClose,
        "panel-left-open" => IconName::PanelLeftOpen,
        "panel-right" => IconName::PanelRight,
        "panel-right-close" => IconName::PanelRightClose,
        "panel-right-open" => IconName::PanelRightOpen,
        "plus" => IconName::Plus,
        "redo" => IconName::Redo,
        "redo-2" => IconName::Redo2,
        "replace" => IconName::Replace,
        "resize-corner" => IconName::ResizeCorner,
        "search" => IconName::Search,
        "settings" => IconName::Settings,
        "settings-2" => IconName::Settings2,
        "sort-ascending" => IconName::SortAscending,
        "sort-descending" => IconName::SortDescending,
        "square-terminal" => IconName::SquareTerminal,
        "star" => IconName::Star,
        "star-off" => IconName::StarOff,
        "sun" => IconName::Sun,
        "thumbs-down" => IconName::ThumbsDown,
        "thumbs-up" => IconName::ThumbsUp,
        "undo" => IconName::Undo,
        "undo-2" => IconName::Undo2,
        "user" => IconName::User,
        "window-close" => IconName::WindowClose,
        "window-maximize" => IconName::WindowMaximize,
        "window-minimize" => IconName::WindowMinimize,
        "window-restore" => IconName::WindowRestore,
        _ => return None,
    })
}
