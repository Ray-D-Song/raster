use gpui::{App, Hsla, Pixels};
use gpui_component::{
    ActiveTheme as _, Anchor, Colorize as _, Edges, Theme, ThemeMode, scroll::ScrollbarShow,
};
use serde_json::{Map, Value, json};

pub(in crate::gpui_backend) fn theme_snapshot_json(cx: &App) -> String {
    serde_json::to_string(&theme_to_snapshot(cx.theme())).unwrap_or_else(|_| "{}".to_owned())
}

fn theme_to_snapshot(theme: &Theme) -> Value {
    json!({
        "colors": colors(theme),
        "highlightTheme": camelize_json_keys(serde_json::to_value(&*theme.highlight_theme).unwrap_or(Value::Null)),
        "lightTheme": camelize_json_keys(serde_json::to_value(&*theme.light_theme).unwrap_or(Value::Null)),
        "darkTheme": camelize_json_keys(serde_json::to_value(&*theme.dark_theme).unwrap_or(Value::Null)),
        "mode": theme_mode(theme.mode),
        "fontFamily": theme.font_family.to_string(),
        "fontSize": px_value(theme.font_size),
        "monoFontFamily": theme.mono_font_family.to_string(),
        "monoFontSize": px_value(theme.mono_font_size),
        "radius": px_value(theme.radius),
        "radiusLg": px_value(theme.radius_lg),
        "shadow": theme.shadow,
        "transparent": color(theme.transparent),
        "scrollbarShow": scrollbar_show(theme.scrollbar_show),
        "notification": {
            "placement": anchor(theme.notification.placement),
            "margins": edges(theme.notification.margins.clone()),
            "maxItems": theme.notification.max_items,
        },
        "tileGridSize": px_value(theme.tile_grid_size),
        "tileShadow": theme.tile_shadow,
        "tileRadius": px_value(theme.tile_radius),
        "list": {
            "activeHighlight": theme.list.active_highlight,
        },
        "sheet": {
            "marginTop": px_value(theme.sheet.margin_top),
        },
    })
}

fn colors(theme: &Theme) -> Value {
    let mut colors = Map::new();
    macro_rules! insert_color {
        ($js:literal, $rust:ident) => {
            colors.insert($js.to_owned(), Value::String(color(theme.colors.$rust)));
        };
    }
    insert_color!("accent", accent);
    insert_color!("accentForeground", accent_foreground);
    insert_color!("accordion", accordion);
    insert_color!("accordionHover", accordion_hover);
    insert_color!("background", background);
    insert_color!("border", border);
    insert_color!("buttonPrimary", button_primary);
    insert_color!("buttonPrimaryActive", button_primary_active);
    insert_color!("buttonPrimaryForeground", button_primary_foreground);
    insert_color!("buttonPrimaryHover", button_primary_hover);
    insert_color!("groupBox", group_box);
    insert_color!("groupBoxForeground", group_box_foreground);
    insert_color!("caret", caret);
    insert_color!("chart1", chart_1);
    insert_color!("chart2", chart_2);
    insert_color!("chart3", chart_3);
    insert_color!("chart4", chart_4);
    insert_color!("chart5", chart_5);
    insert_color!("chartBullish", chart_bullish);
    insert_color!("chartBearish", chart_bearish);
    insert_color!("danger", danger);
    insert_color!("dangerActive", danger_active);
    insert_color!("dangerForeground", danger_foreground);
    insert_color!("dangerHover", danger_hover);
    insert_color!("descriptionListLabel", description_list_label);
    insert_color!(
        "descriptionListLabelForeground",
        description_list_label_foreground
    );
    insert_color!("dragBorder", drag_border);
    insert_color!("dropTarget", drop_target);
    insert_color!("foreground", foreground);
    insert_color!("info", info);
    insert_color!("infoActive", info_active);
    insert_color!("infoForeground", info_foreground);
    insert_color!("infoHover", info_hover);
    insert_color!("input", input);
    insert_color!("link", link);
    insert_color!("linkActive", link_active);
    insert_color!("linkHover", link_hover);
    insert_color!("list", list);
    insert_color!("listActive", list_active);
    insert_color!("listActiveBorder", list_active_border);
    insert_color!("listEven", list_even);
    insert_color!("listHead", list_head);
    insert_color!("listHover", list_hover);
    insert_color!("muted", muted);
    insert_color!("mutedForeground", muted_foreground);
    insert_color!("popover", popover);
    insert_color!("popoverForeground", popover_foreground);
    insert_color!("primary", primary);
    insert_color!("primaryActive", primary_active);
    insert_color!("primaryForeground", primary_foreground);
    insert_color!("primaryHover", primary_hover);
    insert_color!("progressBar", progress_bar);
    insert_color!("ring", ring);
    insert_color!("scrollbar", scrollbar);
    insert_color!("scrollbarThumb", scrollbar_thumb);
    insert_color!("scrollbarThumbHover", scrollbar_thumb_hover);
    insert_color!("secondary", secondary);
    insert_color!("secondaryActive", secondary_active);
    insert_color!("secondaryForeground", secondary_foreground);
    insert_color!("secondaryHover", secondary_hover);
    insert_color!("selection", selection);
    insert_color!("sidebar", sidebar);
    insert_color!("sidebarAccent", sidebar_accent);
    insert_color!("sidebarAccentForeground", sidebar_accent_foreground);
    insert_color!("sidebarBorder", sidebar_border);
    insert_color!("sidebarForeground", sidebar_foreground);
    insert_color!("sidebarPrimary", sidebar_primary);
    insert_color!("sidebarPrimaryForeground", sidebar_primary_foreground);
    insert_color!("skeleton", skeleton);
    insert_color!("sliderBar", slider_bar);
    insert_color!("sliderThumb", slider_thumb);
    insert_color!("success", success);
    insert_color!("successForeground", success_foreground);
    insert_color!("successHover", success_hover);
    insert_color!("successActive", success_active);
    insert_color!("switch", switch);
    insert_color!("switchThumb", switch_thumb);
    insert_color!("tab", tab);
    insert_color!("tabActive", tab_active);
    insert_color!("tabActiveForeground", tab_active_foreground);
    insert_color!("tabBar", tab_bar);
    insert_color!("tabBarSegmented", tab_bar_segmented);
    insert_color!("tabForeground", tab_foreground);
    insert_color!("table", table);
    insert_color!("tableActive", table_active);
    insert_color!("tableActiveBorder", table_active_border);
    insert_color!("tableEven", table_even);
    insert_color!("tableHead", table_head);
    insert_color!("tableHeadForeground", table_head_foreground);
    insert_color!("tableFoot", table_foot);
    insert_color!("tableFootForeground", table_foot_foreground);
    insert_color!("tableHover", table_hover);
    insert_color!("tableRowBorder", table_row_border);
    insert_color!("titleBar", title_bar);
    insert_color!("titleBarBorder", title_bar_border);
    insert_color!("tiles", tiles);
    insert_color!("warning", warning);
    insert_color!("warningActive", warning_active);
    insert_color!("warningHover", warning_hover);
    insert_color!("warningForeground", warning_foreground);
    insert_color!("overlay", overlay);
    insert_color!("windowBorder", window_border);
    insert_color!("red", red);
    insert_color!("redLight", red_light);
    insert_color!("green", green);
    insert_color!("greenLight", green_light);
    insert_color!("blue", blue);
    insert_color!("blueLight", blue_light);
    insert_color!("yellow", yellow);
    insert_color!("yellowLight", yellow_light);
    insert_color!("magenta", magenta);
    insert_color!("magentaLight", magenta_light);
    insert_color!("cyan", cyan);
    insert_color!("cyanLight", cyan_light);
    Value::Object(colors)
}

fn color(value: Hsla) -> String {
    value.to_hex()
}

fn px_value(value: Pixels) -> f32 {
    f32::from(value)
}

fn edges(value: Edges<Pixels>) -> Value {
    json!({
        "top": px_value(value.top),
        "right": px_value(value.right),
        "bottom": px_value(value.bottom),
        "left": px_value(value.left),
    })
}

fn theme_mode(value: ThemeMode) -> &'static str {
    match value {
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
    }
}

fn scrollbar_show(value: ScrollbarShow) -> &'static str {
    match value {
        ScrollbarShow::Scrolling => "scrolling",
        ScrollbarShow::Hover => "hover",
        ScrollbarShow::Always => "always",
    }
}

fn anchor(value: Anchor) -> &'static str {
    match value {
        Anchor::TopLeft => "topLeft",
        Anchor::TopCenter => "topCenter",
        Anchor::TopRight => "topRight",
        Anchor::BottomLeft => "bottomLeft",
        Anchor::BottomCenter => "bottomCenter",
        Anchor::BottomRight => "bottomRight",
        Anchor::LeftCenter => "leftCenter",
        Anchor::RightCenter => "rightCenter",
    }
}

fn camelize_json_keys(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(camelize_json_keys).collect()),
        Value::Object(entries) => Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| (camelize_key(&key), camelize_json_keys(value)))
                .collect::<Map<_, _>>(),
        ),
        value => value,
    }
}

fn camelize_key(value: &str) -> String {
    let mut result = String::new();
    let mut upper_next = false;
    for ch in value.chars() {
        if ch == '_' || ch == '.' {
            upper_next = true;
        } else if upper_next {
            result.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use gpui_component::Theme;

    use super::{camelize_key, theme_to_snapshot};

    #[test]
    fn camelizes_theme_schema_keys() {
        assert_eq!(camelize_key("font.size"), "fontSize");
        assert_eq!(
            camelize_key("primary.hover.background"),
            "primaryHoverBackground"
        );
        assert_eq!(camelize_key("is_default"), "isDefault");
    }

    #[test]
    fn serializes_core_theme_snapshot_fields() {
        let snapshot = theme_to_snapshot(&Theme::default());

        assert!(snapshot.pointer("/colors/background").is_some());
        assert!(snapshot.pointer("/colors/foreground").is_some());
        assert!(snapshot.pointer("/colors/primary").is_some());
        assert!(snapshot.pointer("/notification/margins/top").is_some());
        assert!(snapshot.pointer("/notification/maxItems").is_some());
        assert!(snapshot.pointer("/list/activeHighlight").is_some());
        assert!(snapshot.pointer("/sheet/marginTop").is_some());
        assert!(snapshot.pointer("/fontSize").is_some());
        assert!(snapshot.pointer("/radius").is_some());
    }
}
