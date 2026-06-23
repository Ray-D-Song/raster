//! Root/global ConfigProvider support for gpui-component theme updates.

use std::collections::{BTreeMap, BTreeSet};

use gpui::{App, Hsla, SharedString, px};
use gpui_component::{Colorize, Theme, ThemeColor, ThemeMode};

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

    let is_dark = theme.mode.is_dark();
    let explicit = apply_explicit_theme_colors(&snapshot.colors, &mut *theme);
    reconcile_derived_theme_colors(&mut *theme, is_dark, &explicit);

    cx.refresh_windows();
}

fn apply_explicit_theme_colors(
    colors: &BTreeMap<String, String>,
    theme: &mut ThemeColor,
) -> BTreeSet<String> {
    let mut explicit = BTreeSet::new();
    for (token, value) in colors {
        let Some(color) = parse_color(value) else {
            logger::warn(format!("invalid ConfigProvider theme color for `{token}`"));
            continue;
        };
        if apply_color_token(theme, token, color) {
            explicit.insert(token.clone());
        } else {
            logger::warn(format!("unsupported ConfigProvider theme color `{token}`"));
        }
    }
    explicit
}

fn apply_color_token(theme: &mut ThemeColor, token: &str, color: Hsla) -> bool {
    macro_rules! theme_color_tokens {
        ($theme:ident, $color:ident, $($js:literal => $field:ident),* $(,)?) => {
            match token {
                $( $js => { $theme.$field = $color; true } ),*
                _ => false,
            }
        };
    }

    theme_color_tokens!(theme, color,
        "accent" => accent,
        "accentForeground" => accent_foreground,
        "accordion" => accordion,
        "accordionHover" => accordion_hover,
        "background" => background,
        "blue" => blue,
        "blueLight" => blue_light,
        "border" => border,
        "buttonPrimary" => button_primary,
        "buttonPrimaryActive" => button_primary_active,
        "buttonPrimaryForeground" => button_primary_foreground,
        "buttonPrimaryHover" => button_primary_hover,
        "caret" => caret,
        "chart1" => chart_1,
        "chart2" => chart_2,
        "chart3" => chart_3,
        "chart4" => chart_4,
        "chart5" => chart_5,
        "chartBullish" => chart_bullish,
        "chartBearish" => chart_bearish,
        "cyan" => cyan,
        "cyanLight" => cyan_light,
        "danger" => danger,
        "dangerActive" => danger_active,
        "dangerForeground" => danger_foreground,
        "dangerHover" => danger_hover,
        "descriptionListLabel" => description_list_label,
        "descriptionListLabelForeground" => description_list_label_foreground,
        "dragBorder" => drag_border,
        "dropTarget" => drop_target,
        "foreground" => foreground,
        "green" => green,
        "greenLight" => green_light,
        "groupBox" => group_box,
        "groupBoxForeground" => group_box_foreground,
        "info" => info,
        "infoActive" => info_active,
        "infoForeground" => info_foreground,
        "infoHover" => info_hover,
        "input" => input,
        "link" => link,
        "linkActive" => link_active,
        "linkHover" => link_hover,
        "list" => list,
        "listActive" => list_active,
        "listActiveBorder" => list_active_border,
        "listEven" => list_even,
        "listHead" => list_head,
        "listHover" => list_hover,
        "magenta" => magenta,
        "magentaLight" => magenta_light,
        "muted" => muted,
        "mutedForeground" => muted_foreground,
        "overlay" => overlay,
        "popover" => popover,
        "popoverForeground" => popover_foreground,
        "primary" => primary,
        "primaryActive" => primary_active,
        "primaryForeground" => primary_foreground,
        "primaryHover" => primary_hover,
        "progressBar" => progress_bar,
        "red" => red,
        "redLight" => red_light,
        "ring" => ring,
        "scrollbar" => scrollbar,
        "scrollbarThumb" => scrollbar_thumb,
        "scrollbarThumbHover" => scrollbar_thumb_hover,
        "secondary" => secondary,
        "secondaryActive" => secondary_active,
        "secondaryForeground" => secondary_foreground,
        "secondaryHover" => secondary_hover,
        "selection" => selection,
        "sidebar" => sidebar,
        "sidebarAccent" => sidebar_accent,
        "sidebarAccentForeground" => sidebar_accent_foreground,
        "sidebarBorder" => sidebar_border,
        "sidebarForeground" => sidebar_foreground,
        "sidebarPrimary" => sidebar_primary,
        "sidebarPrimaryForeground" => sidebar_primary_foreground,
        "skeleton" => skeleton,
        "sliderBar" => slider_bar,
        "sliderThumb" => slider_thumb,
        "success" => success,
        "successActive" => success_active,
        "successForeground" => success_foreground,
        "successHover" => success_hover,
        "switch" => switch,
        "switchThumb" => switch_thumb,
        "tab" => tab,
        "tabActive" => tab_active,
        "tabActiveForeground" => tab_active_foreground,
        "tabBar" => tab_bar,
        "tabBarSegmented" => tab_bar_segmented,
        "tabForeground" => tab_foreground,
        "table" => table,
        "tableActive" => table_active,
        "tableActiveBorder" => table_active_border,
        "tableEven" => table_even,
        "tableFoot" => table_foot,
        "tableFootForeground" => table_foot_foreground,
        "tableHead" => table_head,
        "tableHeadForeground" => table_head_foreground,
        "tableHover" => table_hover,
        "tableRowBorder" => table_row_border,
        "tiles" => tiles,
        "titleBar" => title_bar,
        "titleBarBorder" => title_bar_border,
        "warning" => warning,
        "warningActive" => warning_active,
        "warningForeground" => warning_foreground,
        "warningHover" => warning_hover,
        "windowBorder" => window_border,
        "yellow" => yellow,
        "yellowLight" => yellow_light,
    )
}

pub(in crate::gpui_backend) fn reconcile_derived_theme_colors(
    theme: &mut ThemeColor,
    is_dark: bool,
    explicit: &BTreeSet<String>,
) {
    let active_darken = if is_dark { 0.2 } else { 0.1 };
    let hover_opacity = 0.9;

    macro_rules! derive {
        ($key:literal, $field:ident = $value:expr) => {
            set_unless_explicit(explicit, $key, &mut theme.$field, $value);
        };
    }

    derive!(
        "redLight",
        red_light = theme.background.blend(theme.red.opacity(0.8))
    );
    derive!(
        "greenLight",
        green_light = theme.background.blend(theme.green.opacity(0.8))
    );
    derive!(
        "blueLight",
        blue_light = theme.background.blend(theme.blue.opacity(0.8))
    );
    derive!(
        "magentaLight",
        magenta_light = theme.background.blend(theme.magenta.opacity(0.8))
    );
    derive!(
        "yellowLight",
        yellow_light = theme.background.blend(theme.yellow.opacity(0.8))
    );
    derive!(
        "cyanLight",
        cyan_light = theme.background.blend(theme.cyan.opacity(0.8))
    );

    derive!(
        "mutedForeground",
        muted_foreground = theme.muted.blend(theme.foreground.opacity(0.7))
    );

    derive!("primaryForeground", primary_foreground = theme.foreground);
    derive!(
        "primaryHover",
        primary_hover = theme
            .background
            .blend(theme.primary.opacity(hover_opacity))
    );
    derive!(
        "primaryActive",
        primary_active = theme.primary.darken(active_darken)
    );

    derive!("buttonPrimary", button_primary = theme.primary);
    derive!(
        "buttonPrimaryForeground",
        button_primary_foreground = theme.primary_foreground
    );
    derive!(
        "buttonPrimaryHover",
        button_primary_hover = theme.primary_hover
    );
    derive!(
        "buttonPrimaryActive",
        button_primary_active = theme.primary_active
    );

    derive!("secondaryForeground", secondary_foreground = theme.foreground);
    derive!(
        "secondaryHover",
        secondary_hover = theme
            .background
            .blend(theme.secondary.opacity(hover_opacity))
    );
    derive!(
        "secondaryActive",
        secondary_active = theme.secondary.darken(active_darken)
    );

    derive!("success", success = theme.green);
    derive!(
        "successForeground",
        success_foreground = theme.primary_foreground
    );
    derive!(
        "successHover",
        success_hover = theme
            .background
            .blend(theme.success.opacity(hover_opacity))
    );
    derive!(
        "successActive",
        success_active = theme.success.darken(active_darken)
    );

    derive!("info", info = theme.cyan);
    derive!("infoForeground", info_foreground = theme.primary_foreground);
    derive!(
        "infoHover",
        info_hover = theme.background.blend(theme.info.opacity(hover_opacity))
    );
    derive!("infoActive", info_active = theme.info.darken(active_darken));

    derive!("warning", warning = theme.yellow);
    derive!(
        "warningForeground",
        warning_foreground = theme.primary_foreground
    );
    derive!(
        "warningHover",
        warning_hover = theme.background.blend(theme.warning.opacity(0.9))
    );
    derive!(
        "warningActive",
        warning_active = theme.background.blend(theme.warning.darken(active_darken))
    );

    derive!("accent", accent = theme.secondary);
    derive!("accentForeground", accent_foreground = theme.foreground);
    derive!("accordion", accordion = theme.background);
    derive!("accordionHover", accordion_hover = theme.accent.opacity(0.8));
    derive!(
        "groupBox",
        group_box = theme.background.blend(
            theme
                .secondary
                .opacity(if is_dark { 0.3 } else { 0.4 })
        )
    );
    derive!("groupBoxForeground", group_box_foreground = theme.foreground);

    derive!("caret", caret = theme.primary);
    derive!("chart1", chart_1 = theme.blue.lighten(0.4));
    derive!("chart2", chart_2 = theme.blue.lighten(0.2));
    derive!("chart3", chart_3 = theme.blue);
    derive!("chart4", chart_4 = theme.blue.darken(0.2));
    derive!("chart5", chart_5 = theme.blue.darken(0.4));
    derive!("chartBullish", chart_bullish = theme.green);
    derive!("chartBearish", chart_bearish = theme.red);

    derive!("danger", danger = theme.red);
    derive!(
        "dangerActive",
        danger_active = theme.danger.darken(active_darken)
    );
    derive!(
        "dangerForeground",
        danger_foreground = theme.primary_foreground
    );
    derive!(
        "dangerHover",
        danger_hover = theme.background.blend(theme.danger.opacity(0.9))
    );

    derive!(
        "descriptionListLabel",
        description_list_label = theme.background.blend(theme.border.opacity(0.2))
    );
    derive!(
        "descriptionListLabelForeground",
        description_list_label_foreground = theme.muted_foreground
    );
    derive!("dragBorder", drag_border = theme.primary.opacity(0.65));
    derive!("dropTarget", drop_target = theme.primary.opacity(0.2));
    derive!("input", input = theme.border);
    derive!("link", link = theme.primary);
    derive!("linkActive", link_active = theme.link);
    derive!("linkHover", link_hover = theme.link);

    derive!("list", list = theme.background);
    derive!(
        "listActive",
        list_active = theme.background.blend(theme.primary.opacity(0.1))
    );
    derive!(
        "listActiveBorder",
        list_active_border = theme.background.blend(theme.primary.opacity(0.6))
    );
    derive!("listEven", list_even = theme.list);
    derive!("listHead", list_head = theme.list);
    derive!("listHover", list_hover = theme.accent.opacity(0.6));

    derive!("popover", popover = theme.background);
    derive!("popoverForeground", popover_foreground = theme.foreground);
    derive!("progressBar", progress_bar = theme.primary);
    derive!("ring", ring = theme.blue);
    derive!("scrollbar", scrollbar = theme.background);
    derive!("scrollbarThumb", scrollbar_thumb = theme.accent);
    derive!(
        "scrollbarThumbHover",
        scrollbar_thumb_hover = theme.scrollbar_thumb
    );
    derive!("selection", selection = theme.primary);

    derive!(
        "sidebar",
        sidebar = theme.background.blend(theme.border.opacity(0.15))
    );
    derive!("sidebarAccent", sidebar_accent = theme.accent);
    derive!(
        "sidebarAccentForeground",
        sidebar_accent_foreground = theme.accent_foreground
    );
    derive!("sidebarBorder", sidebar_border = theme.border);
    derive!("sidebarForeground", sidebar_foreground = theme.foreground);
    derive!("sidebarPrimary", sidebar_primary = theme.primary);
    derive!(
        "sidebarPrimaryForeground",
        sidebar_primary_foreground = theme.primary_foreground
    );

    derive!("skeleton", skeleton = theme.secondary);
    derive!("sliderBar", slider_bar = theme.primary);
    derive!("sliderThumb", slider_thumb = theme.primary_foreground);
    derive!("switch", switch = theme.secondary_active);
    derive!("switchThumb", switch_thumb = theme.background);

    derive!("tab", tab = theme.background);
    derive!("tabActive", tab_active = theme.background);
    derive!("tabActiveForeground", tab_active_foreground = theme.foreground);
    derive!("tabBar", tab_bar = theme.background);
    derive!("tabBarSegmented", tab_bar_segmented = theme.secondary);
    derive!("tabForeground", tab_foreground = theme.foreground);

    derive!("table", table = theme.list);
    derive!("tableActive", table_active = theme.list_active);
    derive!(
        "tableActiveBorder",
        table_active_border = theme.list_active_border
    );
    derive!("tableEven", table_even = theme.list_even);
    derive!("tableHead", table_head = theme.list_head);
    derive!(
        "tableHeadForeground",
        table_head_foreground = theme.muted_foreground
    );
    derive!("tableFoot", table_foot = theme.list_head);
    derive!(
        "tableFootForeground",
        table_foot_foreground = theme.muted_foreground
    );
    derive!("tableHover", table_hover = theme.list_hover);
    derive!("tableRowBorder", table_row_border = theme.border);
    derive!("titleBar", title_bar = theme.background);
    derive!("titleBarBorder", title_bar_border = theme.border);
    derive!("tiles", tiles = theme.background);
    derive!("windowBorder", window_border = theme.border);

    if !explicit.contains("listActive") {
        theme.list_active = theme.list_active.alpha(theme.list_active.a.min(0.2));
    }
    if !explicit.contains("tableActive") {
        theme.table_active = theme.table_active.alpha(theme.table_active.a.min(0.2));
    }
    if !explicit.contains("selection") {
        theme.selection = theme.selection.alpha(theme.selection.a.min(0.3));
    }
}

fn set_unless_explicit(
    explicit: &BTreeSet<String>,
    key: &str,
    slot: &mut Hsla,
    value: Hsla,
) {
    if !explicit.contains(key) {
        *slot = value;
    }
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

    fn colors_close(left: Hsla, right: Hsla) -> bool {
        (left.h - right.h).abs() < 0.02
            && (left.s - right.s).abs() < 0.02
            && (left.l - right.l).abs() < 0.02
    }

    #[test]
    fn primary_override_cascades_to_button_slider_and_progress() {
        let primary = parse_color("#006b5f").expect("valid primary");
        let mut theme = ThemeColor::default();
        theme.primary = primary;

        let explicit = BTreeSet::from(["primary".to_owned()]);
        reconcile_derived_theme_colors(&mut theme, false, &explicit);

        assert!(colors_close(theme.button_primary, primary));
        assert!(colors_close(theme.slider_bar, primary));
        assert!(colors_close(theme.progress_bar, primary));
    }

    #[test]
    fn explicit_button_primary_is_not_overwritten_by_primary_cascade() {
        let primary = parse_color("#006b5f").expect("valid primary");
        let button_primary = parse_color("#ff0000").expect("valid override");
        let mut theme = ThemeColor::default();
        theme.primary = primary;
        theme.button_primary = button_primary;

        let explicit = BTreeSet::from([
            "primary".to_owned(),
            "buttonPrimary".to_owned(),
        ]);
        reconcile_derived_theme_colors(&mut theme, false, &explicit);

        assert!(colors_close(theme.button_primary, button_primary));
        assert!(colors_close(theme.slider_bar, primary));
    }

    #[test]
    fn secondary_override_cascades_to_switch_when_switch_not_explicit() {
        let secondary = parse_color("#0058be").expect("valid secondary");
        let mut theme = ThemeColor::default();
        theme.secondary = secondary;

        let explicit = BTreeSet::from(["secondary".to_owned()]);
        reconcile_derived_theme_colors(&mut theme, false, &explicit);

        assert!(colors_close(theme.secondary_active, secondary.darken(0.1)));
        assert!(colors_close(theme.switch, theme.secondary_active));
    }

    #[test]
    fn primary_foreground_cascades_to_slider_thumb_and_button_text() {
        let primary_foreground = parse_color("#ffffff").expect("valid foreground");
        let mut theme = ThemeColor::default();
        theme.primary_foreground = primary_foreground;

        let explicit = BTreeSet::from(["primaryForeground".to_owned()]);
        reconcile_derived_theme_colors(&mut theme, false, &explicit);

        assert!(colors_close(theme.slider_thumb, primary_foreground));
        assert!(colors_close(theme.button_primary_foreground, primary_foreground));
    }

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
