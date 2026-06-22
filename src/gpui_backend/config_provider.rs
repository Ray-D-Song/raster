//! Root/global ConfigProvider support for gpui-component theme updates.

use std::collections::BTreeMap;

use gpui::{App, SharedString, px};
use gpui_component::{Theme, ThemeMode};

use crate::{
    common::{mount::NodeValue, utils::logger},
    gpui_backend::{
        components::helper::props,
        embedded_themes::{default_theme_name, registry_theme},
        render_model::style::parse_color,
        retained_tree::{node::RetainedNode, tree::RetainedTree},
    },
};

#[derive(Debug, Clone, PartialEq, Default)]
pub(in crate::gpui_backend) struct RasterThemeSnapshot {
    preset: Option<RasterThemePreset>,
    mode: Option<RasterThemeMode>,
    radius: Option<f32>,
    radius_lg: Option<f32>,
    font_size: Option<f32>,
    font_family: Option<String>,
    mono_font_size: Option<f32>,
    mono_font_family: Option<String>,
    colors: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RasterThemePreset {
    Name(String),
    Pair {
        light: Option<String>,
        dark: Option<String>,
    },
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
    let target_mode = match snapshot.mode {
        Some(RasterThemeMode::Light) => Some(ThemeMode::Light),
        Some(RasterThemeMode::Dark) => Some(ThemeMode::Dark),
        Some(RasterThemeMode::System) => Some(cx.window_appearance().into()),
        None => snapshot.preset.as_ref().map(|_| Theme::global(cx).mode),
    };

    if let Some(mode) = target_mode {
        apply_theme_preset(snapshot.preset.as_ref(), mode, cx);
        Theme::change(mode, None, cx);
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
            "secondaryHover" => theme.secondary_hover = color,
            "secondaryActive" => theme.secondary_active = color,
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
    apply_default_theme_pair(cx);
    Theme::change(cx.window_appearance(), None, cx);
}

pub(in crate::gpui_backend) fn reset_theme_snapshot(cx: &mut App) {
    apply_raster_default_theme(cx);
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
        preset: parse_preset(theme),
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

fn parse_preset(object: &BTreeMap<String, NodeValue>) -> Option<RasterThemePreset> {
    match object.get("preset") {
        Some(NodeValue::String(value)) => Some(RasterThemePreset::Name(value.clone())),
        Some(NodeValue::Object(value)) => Some(RasterThemePreset::Pair {
            light: string_value(value, "light"),
            dark: string_value(value, "dark"),
        }),
        Some(_) => {
            logger::warn("unsupported ConfigProvider theme preset");
            None
        }
        None => None,
    }
}

fn string_value(object: &BTreeMap<String, NodeValue>, key: &str) -> Option<String> {
    match object.get(key) {
        Some(NodeValue::String(value)) => Some(value.clone()),
        _ => None,
    }
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

fn apply_theme_preset(preset: Option<&RasterThemePreset>, target_mode: ThemeMode, cx: &mut App) {
    apply_default_theme_pair(cx);

    match preset {
        Some(RasterThemePreset::Name(name)) => set_theme_for_mode(target_mode, name, cx),
        Some(RasterThemePreset::Pair { light, dark }) => {
            if let Some(name) = light {
                set_theme_for_mode(ThemeMode::Light, name, cx);
            }
            if let Some(name) = dark {
                set_theme_for_mode(ThemeMode::Dark, name, cx);
            }
        }
        None => {}
    }
}

fn apply_default_theme_pair(cx: &mut App) {
    set_theme_for_mode(ThemeMode::Light, default_theme_name(ThemeMode::Light), cx);
    set_theme_for_mode(ThemeMode::Dark, default_theme_name(ThemeMode::Dark), cx);
}

fn set_theme_for_mode(mode: ThemeMode, name: &str, cx: &mut App) {
    let resolved = registry_theme(cx, name).or_else(|| {
        logger::warn(format!(
            "unknown ConfigProvider theme preset `{name}`; using `{}`",
            default_theme_name(mode)
        ));
        registry_theme(cx, default_theme_name(mode))
    });

    let Some(config) = resolved else {
        logger::warn(format!(
            "default Raster theme preset `{}` is not loaded",
            default_theme_name(mode)
        ));
        return;
    };

    let theme = Theme::global_mut(cx);
    if mode.is_dark() {
        theme.dark_theme = config;
    } else {
        theme.light_theme = config;
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
        let mut preset = BTreeMap::new();
        preset.insert(
            "light".to_owned(),
            NodeValue::String("macOS Classic Light".to_owned()),
        );
        preset.insert("dark".to_owned(), NodeValue::String("Ayu Dark".to_owned()));
        theme.insert("mode".to_owned(), NodeValue::String("dark".to_owned()));
        theme.insert("preset".to_owned(), NodeValue::Object(preset));
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
        assert_eq!(
            snapshot.preset,
            Some(RasterThemePreset::Pair {
                light: Some("macOS Classic Light".to_owned()),
                dark: Some("Ayu Dark".to_owned())
            })
        );
        assert_eq!(snapshot.radius, Some(8.0));
        assert_eq!(
            snapshot.colors.get("primary").map(String::as_str),
            Some("#2563eb")
        );
    }
}
