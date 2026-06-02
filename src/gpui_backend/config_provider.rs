//! Root/global ConfigProvider support for gpui-component theme updates.

use std::collections::BTreeMap;

use gpui::{App, SharedString, WindowAppearance, px};
use gpui_component::{Theme, ThemeMode};

use crate::{
    common::{mount::NodeValue, utils::logger},
    gpui_backend::{
        components::helper::props,
        render_model::style::parse_color,
        retained_tree::{node::RetainedNode, tree::RetainedTree},
    },
};

#[derive(Debug, Clone, PartialEq, Default)]
pub(in crate::gpui_backend) struct RasterThemeSnapshot {
    mode: Option<RasterThemeMode>,
    radius: Option<f32>,
    radius_lg: Option<f32>,
    font_size: Option<f32>,
    font_family: Option<String>,
    mono_font_size: Option<f32>,
    mono_font_family: Option<String>,
    colors: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RasterThemeMode {
    Light,
    Dark,
    System,
}

pub(in crate::gpui_backend) fn is_config_provider_node(node: &RetainedNode) -> bool {
    node.component_name() == "ConfigProvider"
}

pub(in crate::gpui_backend) fn find_config_provider_theme(
    tree: &RetainedTree,
    surface_id: crate::common::ids::SurfaceId,
) -> Option<RasterThemeSnapshot> {
    let roots = tree.surface(surface_id)?.roots.clone();
    let mut found = Vec::new();
    for root in roots {
        collect_config_provider_themes(tree, root, &mut found);
    }

    if found.len() > 1 {
        logger::warn("multiple root ConfigProvider themes found; using the first one");
    }

    found.into_iter().next()
}

pub(in crate::gpui_backend) fn apply_theme_snapshot(snapshot: &RasterThemeSnapshot, cx: &mut App) {
    match snapshot.mode {
        Some(RasterThemeMode::Light) => apply_raster_theme_mode(ResolvedThemeMode::Light, cx),
        Some(RasterThemeMode::Dark) => apply_raster_theme_mode(ResolvedThemeMode::Dark, cx),
        Some(RasterThemeMode::System) => {
            let mode = ResolvedThemeMode::from_window_appearance(cx.window_appearance());
            apply_raster_theme_mode(mode, cx);
        }
        None => {}
    }

    let theme = Theme::global_mut(cx);
    if let Some(radius) = snapshot.radius {
        theme.radius = px(radius);
    }
    if let Some(radius_lg) = snapshot.radius_lg {
        theme.radius_lg = px(radius_lg);
    }
    if let Some(font_size) = snapshot.font_size {
        theme.font_size = px(font_size);
    }
    if let Some(font_family) = &snapshot.font_family {
        theme.font_family = SharedString::from(font_family.clone());
    }
    if let Some(mono_font_size) = snapshot.mono_font_size {
        theme.mono_font_size = px(mono_font_size);
    }
    if let Some(mono_font_family) = &snapshot.mono_font_family {
        theme.mono_font_family = SharedString::from(mono_font_family.clone());
    }

    for (token, color) in &snapshot.colors {
        let Some(color) = parse_color(color) else {
            logger::warn(format!("invalid ConfigProvider theme color for `{token}`"));
            continue;
        };
        match token.as_str() {
            "background" => theme.background = color,
            "foreground" => theme.foreground = color,
            "border" => theme.border = color,
            "input" => theme.input = color,
            "primary" => theme.primary = color,
            "primaryForeground" => theme.primary_foreground = color,
            "secondary" => theme.secondary = color,
            "secondaryForeground" => theme.secondary_foreground = color,
            "accent" => theme.accent = color,
            "accentForeground" => theme.accent_foreground = color,
            "muted" => theme.muted = color,
            "mutedForeground" => theme.muted_foreground = color,
            "popover" => theme.popover = color,
            "popoverForeground" => theme.popover_foreground = color,
            "ring" => theme.ring = color,
            "danger" => theme.danger = color,
            "success" => theme.success = color,
            "warning" => theme.warning = color,
            "info" => theme.info = color,
            _ => logger::warn(format!("unsupported ConfigProvider theme color `{token}`")),
        }
    }

    cx.refresh_windows();
}

pub(in crate::gpui_backend) fn apply_raster_default_theme(cx: &mut App) {
    let mode = ResolvedThemeMode::from_window_appearance(cx.window_appearance());
    apply_raster_theme_mode(mode, cx);
}

fn apply_raster_theme_mode(mode: ResolvedThemeMode, cx: &mut App) {
    match mode {
        ResolvedThemeMode::Light => apply_raster_light_theme(cx),
        ResolvedThemeMode::Dark => apply_raster_dark_theme(cx),
    }
}

fn apply_raster_light_theme(cx: &mut App) {
    Theme::change(ThemeMode::Light, None, cx);
    let theme = Theme::global_mut(cx);
    theme.radius = px(8.0);
    theme.radius_lg = px(8.0);
    theme.sheet.margin_top = px(0.0);

    set_theme_color(&mut theme.background, "#faf9f8");
    set_theme_color(&mut theme.foreground, "#1b1a19");
    set_theme_color(&mut theme.border, "#c8c6c4");
    set_theme_color(&mut theme.input, "#8a8886");
    set_theme_color(&mut theme.primary, "#0078d4");
    set_theme_color(&mut theme.primary_foreground, "#ffffff");
    set_theme_color(&mut theme.primary_hover, "#106ebe");
    set_theme_color(&mut theme.primary_active, "#005a9e");
    set_theme_color(&mut theme.button_primary, "#0078d4");
    set_theme_color(&mut theme.button_primary_foreground, "#ffffff");
    set_theme_color(&mut theme.button_primary_hover, "#106ebe");
    set_theme_color(&mut theme.button_primary_active, "#005a9e");
    set_theme_color(&mut theme.secondary, "#edebe9");
    set_theme_color(&mut theme.secondary_foreground, "#1b1a19");
    set_theme_color(&mut theme.secondary_hover, "#e1dfdd");
    set_theme_color(&mut theme.secondary_active, "#d2d0ce");
    set_theme_color(&mut theme.accent, "#106ebe");
    set_theme_color(&mut theme.accent_foreground, "#ffffff");
    set_theme_color(&mut theme.muted, "#edebe9");
    set_theme_color(&mut theme.muted_foreground, "#605e5c");
    set_theme_color(&mut theme.popover, "#ffffff");
    set_theme_color(&mut theme.popover_foreground, "#1b1a19");
    set_theme_color(&mut theme.ring, "#2899f5");
    set_theme_color(&mut theme.info, "#0078d4");
    set_theme_color(&mut theme.info_foreground, "#ffffff");
    set_theme_color(&mut theme.success, "#107c10");
    set_theme_color(&mut theme.success_foreground, "#ffffff");
    set_theme_color(&mut theme.warning, "#fce100");
    set_theme_color(&mut theme.warning_foreground, "#1b1a19");
    set_theme_color(&mut theme.danger, "#d13438");
    set_theme_color(&mut theme.danger_foreground, "#ffffff");
    set_theme_color(&mut theme.selection, "#c7e0f4");
    set_theme_color(&mut theme.link, "#0078d4");
    set_theme_color(&mut theme.link_hover, "#005a9e");
    set_theme_color(&mut theme.switch, "#605e5c");
    set_theme_color(&mut theme.switch_thumb, "#ffffff");
    set_theme_color(&mut theme.slider_bar, "#0078d4");
    set_theme_color(&mut theme.slider_thumb, "#ffffff");
    set_theme_color(&mut theme.tab, "#faf9f8");
    set_theme_color(&mut theme.tab_active, "#faf9f8");
    set_theme_color(&mut theme.tab_active_foreground, "#1b1a19");
    set_theme_color(&mut theme.tab_bar, "#faf9f8");
    set_theme_color(&mut theme.tab_bar_segmented, "#edebe9");
    set_theme_color(&mut theme.tab_foreground, "#605e5c");
    set_theme_color(&mut theme.title_bar, "#faf9f8");
    set_theme_color(&mut theme.title_bar_border, "#c8c6c4");

    cx.refresh_windows();
}

fn apply_raster_dark_theme(cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);
    let theme = Theme::global_mut(cx);
    theme.radius = px(8.0);
    theme.radius_lg = px(8.0);
    theme.sheet.margin_top = px(0.0);

    set_theme_color(&mut theme.background, "#202020");
    set_theme_color(&mut theme.foreground, "#f3f2f1");
    set_theme_color(&mut theme.border, "#3b3a39");
    set_theme_color(&mut theme.input, "#605e5c");
    set_theme_color(&mut theme.primary, "#0078d4");
    set_theme_color(&mut theme.primary_foreground, "#ffffff");
    set_theme_color(&mut theme.primary_hover, "#106ebe");
    set_theme_color(&mut theme.primary_active, "#005a9e");
    set_theme_color(&mut theme.button_primary, "#0078d4");
    set_theme_color(&mut theme.button_primary_foreground, "#ffffff");
    set_theme_color(&mut theme.button_primary_hover, "#106ebe");
    set_theme_color(&mut theme.button_primary_active, "#005a9e");
    set_theme_color(&mut theme.secondary, "#2b2b2b");
    set_theme_color(&mut theme.secondary_foreground, "#f3f2f1");
    set_theme_color(&mut theme.secondary_hover, "#323130");
    set_theme_color(&mut theme.secondary_active, "#3b3a39");
    set_theme_color(&mut theme.accent, "#106ebe");
    set_theme_color(&mut theme.accent_foreground, "#ffffff");
    set_theme_color(&mut theme.muted, "#2d2d2d");
    set_theme_color(&mut theme.muted_foreground, "#c8c6c4");
    set_theme_color(&mut theme.popover, "#252423");
    set_theme_color(&mut theme.popover_foreground, "#f3f2f1");
    set_theme_color(&mut theme.ring, "#2899f5");
    set_theme_color(&mut theme.info, "#2899f5");
    set_theme_color(&mut theme.info_foreground, "#ffffff");
    set_theme_color(&mut theme.success, "#107c10");
    set_theme_color(&mut theme.success_foreground, "#ffffff");
    set_theme_color(&mut theme.warning, "#fce100");
    set_theme_color(&mut theme.warning_foreground, "#1b1a19");
    set_theme_color(&mut theme.danger, "#d13438");
    set_theme_color(&mut theme.danger_foreground, "#ffffff");
    set_theme_color(&mut theme.selection, "#0078d4");
    set_theme_color(&mut theme.link, "#2899f5");
    set_theme_color(&mut theme.link_hover, "#60cdff");
    set_theme_color(&mut theme.switch, "#605e5c");
    set_theme_color(&mut theme.switch_thumb, "#202020");
    set_theme_color(&mut theme.slider_bar, "#0078d4");
    set_theme_color(&mut theme.slider_thumb, "#ffffff");
    set_theme_color(&mut theme.tab, "#202020");
    set_theme_color(&mut theme.tab_active, "#202020");
    set_theme_color(&mut theme.tab_active_foreground, "#f3f2f1");
    set_theme_color(&mut theme.tab_bar, "#202020");
    set_theme_color(&mut theme.tab_bar_segmented, "#2b2b2b");
    set_theme_color(&mut theme.tab_foreground, "#c8c6c4");
    set_theme_color(&mut theme.title_bar, "#202020");
    set_theme_color(&mut theme.title_bar_border, "#3b3a39");

    cx.refresh_windows();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolvedThemeMode {
    Light,
    Dark,
}

impl ResolvedThemeMode {
    fn from_window_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::Light,
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::Dark,
        }
    }
}

pub(in crate::gpui_backend) fn reset_theme_snapshot(cx: &mut App) {
    apply_raster_default_theme(cx);
}

fn set_theme_color(target: &mut gpui::Hsla, value: &str) {
    if let Some(color) = parse_color(value) {
        *target = color;
    }
}

fn collect_config_provider_themes(
    tree: &RetainedTree,
    id: crate::common::ids::NativeObjectId,
    found: &mut Vec<RasterThemeSnapshot>,
) {
    let Some(node) = tree.node(id) else {
        return;
    };
    if is_config_provider_node(node)
        && let Some(snapshot) = parse_theme_snapshot(node)
    {
        found.push(snapshot);
    }
    for child in node.children.clone() {
        collect_config_provider_themes(tree, child, found);
    }
}

fn parse_theme_snapshot(node: &RetainedNode) -> Option<RasterThemeSnapshot> {
    let theme = object_prop(&node.payload.props, "theme")?;
    let mut snapshot = RasterThemeSnapshot {
        mode: props::string_prop(theme, "mode")
            .as_deref()
            .and_then(parse_mode),
        radius: props::number_prop(theme, "radius").map(|value| value as f32),
        radius_lg: props::number_prop(theme, "radiusLg").map(|value| value as f32),
        font_size: props::number_prop(theme, "fontSize").map(|value| value as f32),
        font_family: props::string_prop(theme, "fontFamily"),
        mono_font_size: props::number_prop(theme, "monoFontSize").map(|value| value as f32),
        mono_font_family: props::string_prop(theme, "monoFontFamily"),
        colors: BTreeMap::new(),
    };

    if let Some(colors) = object_prop(theme, "colors") {
        for (key, value) in colors {
            if let Some(value) = value.as_str() {
                snapshot.colors.insert(key.clone(), value.to_owned());
            }
        }
    }

    Some(snapshot)
}

fn object_prop<'a>(
    object: &'a BTreeMap<String, NodeValue>,
    key: &str,
) -> Option<&'a BTreeMap<String, NodeValue>> {
    match object.get(key) {
        Some(NodeValue::Object(value)) => Some(value),
        _ => None,
    }
}

fn parse_mode(value: &str) -> Option<RasterThemeMode> {
    match value {
        "light" => Some(RasterThemeMode::Light),
        "dark" => Some(RasterThemeMode::Dark),
        "system" => Some(RasterThemeMode::System),
        _ => {
            logger::warn(format!("unsupported ConfigProvider theme mode `{value}`"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::{
            ids::{NativeObjectId, SurfaceId},
            mount::{NodePayload, RetainedNodeKind},
        },
        gpui_backend::retained_tree::node::RetainedNode,
    };

    #[test]
    fn parses_core_theme_tokens() {
        let mut colors = BTreeMap::new();
        colors.insert(
            "primary".to_owned(),
            NodeValue::String("#2563eb".to_owned()),
        );

        let mut theme = BTreeMap::new();
        theme.insert("mode".to_owned(), NodeValue::String("dark".to_owned()));
        theme.insert("radius".to_owned(), NodeValue::Number(8.0));
        theme.insert("colors".to_owned(), NodeValue::Object(colors));

        let mut props = BTreeMap::new();
        props.insert("theme".to_owned(), NodeValue::Object(theme));

        let node = RetainedNode::new(
            NativeObjectId(1),
            SurfaceId(1),
            RetainedNodeKind::Widget,
            "ConfigProvider",
            None,
            NodePayload {
                props,
                ..NodePayload::default()
            },
        );

        let snapshot = parse_theme_snapshot(&node).expect("theme snapshot");
        assert_eq!(snapshot.mode, Some(RasterThemeMode::Dark));
        assert_eq!(snapshot.radius, Some(8.0));
        assert_eq!(
            snapshot.colors.get("primary").map(String::as_str),
            Some("#2563eb")
        );
    }
}
