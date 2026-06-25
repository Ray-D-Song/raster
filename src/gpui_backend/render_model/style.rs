use std::collections::BTreeMap;

use gpui::{
    AbsoluteLength, AlignContent, AlignItems, BoxShadow, DefiniteLength, Display, FlexDirection,
    FlexWrap, FontStyle, FontWeight, Hsla, JustifyContent, Length, Overflow,
    Position, Rgba, Styled, hsla, point, px,
};

use crate::common::mount::NodeValue;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RenderStyle {
    pub display: Option<Display>,
    pub flex_direction: Option<FlexDirection>,
    pub flex_wrap: Option<FlexWrap>,
    pub justify_content: Option<JustifyContent>,
    pub align_items: Option<AlignItems>,
    pub align_self: Option<AlignItems>,
    pub align_content: Option<AlignContent>,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub min_width: Option<Length>,
    pub min_height: Option<Length>,
    pub max_width: Option<Length>,
    pub max_height: Option<Length>,
    pub aspect_ratio: Option<f32>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub flex_basis: Option<Length>,
    pub row_gap: Option<DefiniteLength>,
    pub column_gap: Option<DefiniteLength>,
    pub position: Option<Position>,
    pub top: Option<Length>,
    pub right: Option<Length>,
    pub bottom: Option<Length>,
    pub left: Option<Length>,
    pub overflow_x: Option<Overflow>,
    pub overflow_y: Option<Overflow>,
    pub padding: RenderEdges<DefiniteLength>,
    pub margin: RenderEdges<Length>,
    pub background_color: Option<Hsla>,
    pub border_widths: RenderEdges<AbsoluteLength>,
    pub border_color: Option<Hsla>,
    pub border_radii: RenderCorners<AbsoluteLength>,
    pub opacity: Option<f32>,
    pub color: Option<Hsla>,
    pub font_size: Option<AbsoluteLength>,
    pub font_weight: Option<FontWeight>,
    pub font_style: Option<FontStyle>,
    pub underline: Option<bool>,
    pub box_shadow: Vec<BoxShadow>,
    pub backdrop_blur: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderEdges<T> {
    pub top: Option<T>,
    pub right: Option<T>,
    pub bottom: Option<T>,
    pub left: Option<T>,
}

impl<T> Default for RenderEdges<T> {
    fn default() -> Self {
        Self {
            top: None,
            right: None,
            bottom: None,
            left: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderCorners<T> {
    pub top_left: Option<T>,
    pub top_right: Option<T>,
    pub bottom_right: Option<T>,
    pub bottom_left: Option<T>,
}

impl<T> Default for RenderCorners<T> {
    fn default() -> Self {
        Self {
            top_left: None,
            top_right: None,
            bottom_right: None,
            bottom_left: None,
        }
    }
}

pub fn parse_render_style(style: &BTreeMap<String, NodeValue>) -> RenderStyle {
    let mut parsed = RenderStyle {
        display: string_value(style, "display").and_then(parse_display),
        flex_direction: string_value(style, "flexDirection").and_then(parse_flex_direction),
        flex_wrap: string_value(style, "flexWrap").and_then(parse_flex_wrap),
        justify_content: string_value(style, "justifyContent").and_then(parse_justify_content),
        align_items: string_value(style, "alignItems").and_then(parse_align_items),
        align_self: string_value(style, "alignSelf").and_then(parse_align_items),
        align_content: string_value(style, "alignContent").and_then(parse_align_content),
        width: dimension_value(style, "width"),
        height: dimension_value(style, "height"),
        min_width: dimension_value(style, "minWidth"),
        min_height: dimension_value(style, "minHeight"),
        max_width: dimension_value(style, "maxWidth"),
        max_height: dimension_value(style, "maxHeight"),
        aspect_ratio: number_value(style, "aspectRatio"),
        flex_grow: number_value(style, "flexGrow"),
        flex_shrink: number_value(style, "flexShrink"),
        flex_basis: dimension_value(style, "flexBasis"),
        row_gap: definite_value(style, "rowGap"),
        column_gap: definite_value(style, "columnGap"),
        position: string_value(style, "position").and_then(parse_position),
        top: dimension_value(style, "top"),
        right: dimension_value(style, "right"),
        bottom: dimension_value(style, "bottom"),
        left: dimension_value(style, "left"),
        overflow_x: string_value(style, "overflowX").and_then(parse_overflow),
        overflow_y: string_value(style, "overflowY").and_then(parse_overflow),
        padding: edge_value(style.get("padding"), definite_from_node),
        margin: edge_value(style.get("margin"), dimension_from_node),
        background_color: string_value(style, "backgroundColor").and_then(parse_color),
        border_widths: edge_value(style.get("borderWidth"), absolute_from_node),
        border_color: string_value(style, "borderColor").and_then(parse_color),
        border_radii: corner_value(style.get("borderRadius"), absolute_from_node),
        opacity: number_value(style, "opacity"),
        color: string_value(style, "color").and_then(parse_color),
        font_size: style.get("fontSize").and_then(absolute_from_node),
        font_weight: style.get("fontWeight").and_then(parse_font_weight),
        font_style: string_value(style, "fontStyle").and_then(parse_font_style),
        underline: string_value(style, "textDecorationLine").and_then(parse_text_decoration),
        box_shadow: style
            .get("boxShadow")
            .map(parse_box_shadow)
            .unwrap_or_default(),
        backdrop_blur: number_value(style, "backdropBlur"),
    };

    if let Some(flex) = number_value(style, "flex") {
        parsed.flex_grow.get_or_insert(flex);
        parsed.flex_shrink.get_or_insert(1.0);
        parsed
            .flex_basis
            .get_or_insert(Length::Definite(px(0.0).into()));
    }

    if let Some(gap) = definite_value(style, "gap") {
        parsed.row_gap.get_or_insert(gap);
        parsed.column_gap.get_or_insert(gap);
    }

    if let Some(overflow) = string_value(style, "overflow").and_then(parse_overflow) {
        parsed.overflow_x.get_or_insert(overflow);
        parsed.overflow_y.get_or_insert(overflow);
    }

    apply_edge_override(
        &mut parsed.border_widths,
        style,
        "borderTopWidth",
        "borderRightWidth",
        "borderBottomWidth",
        "borderLeftWidth",
        absolute_from_node,
    );
    apply_corner_override(
        &mut parsed.border_radii,
        style,
        "borderTopLeftRadius",
        "borderTopRightRadius",
        "borderBottomRightRadius",
        "borderBottomLeftRadius",
        absolute_from_node,
    );

    parsed
}

pub fn apply_style<T: Styled>(mut element: T, style: &RenderStyle) -> T {
    {
        let target = element.style();
        if let Some(value) = style.display {
            target.display = Some(value);
        }
        if let Some(value) = style.flex_direction {
            target.flex_direction = Some(value);
        }
        if let Some(value) = style.flex_wrap {
            target.flex_wrap = Some(value);
        }
        if let Some(value) = style.justify_content {
            target.justify_content = Some(value);
        }
        if let Some(value) = style.align_items {
            target.align_items = Some(value);
        }
        if let Some(value) = style.align_self {
            target.align_self = Some(value);
        }
        if let Some(value) = style.align_content {
            target.align_content = Some(value);
        }
        if let Some(value) = style.width {
            target.size.width = Some(value);
        }
        if let Some(value) = style.height {
            target.size.height = Some(value);
        }
        if let Some(value) = style.min_width {
            target.min_size.width = Some(value);
        }
        if let Some(value) = style.min_height {
            target.min_size.height = Some(value);
        }
        if let Some(value) = style.max_width {
            target.max_size.width = Some(value);
        }
        if let Some(value) = style.max_height {
            target.max_size.height = Some(value);
        }
        if let Some(value) = style.aspect_ratio {
            target.aspect_ratio = Some(value);
        }
        if let Some(value) = style.flex_grow {
            target.flex_grow = Some(value);
        }
        if let Some(value) = style.flex_shrink {
            target.flex_shrink = Some(value);
        }
        if let Some(value) = style.flex_basis {
            target.flex_basis = Some(value);
        }
        if let Some(value) = style.row_gap {
            target.gap.height = Some(value);
        }
        if let Some(value) = style.column_gap {
            target.gap.width = Some(value);
        }
        if let Some(value) = style.position {
            target.position = Some(value);
        }
        if let Some(value) = style.top {
            target.inset.top = Some(value);
        }
        if let Some(value) = style.right {
            target.inset.right = Some(value);
        }
        if let Some(value) = style.bottom {
            target.inset.bottom = Some(value);
        }
        if let Some(value) = style.left {
            target.inset.left = Some(value);
        }
        if let Some(value) = style.overflow_x {
            target.overflow.x = Some(value);
        }
        if let Some(value) = style.overflow_y {
            target.overflow.y = Some(value);
        }
        if let Some(value) = style.padding.top {
            target.padding.top = Some(value);
        }
        if let Some(value) = style.padding.right {
            target.padding.right = Some(value);
        }
        if let Some(value) = style.padding.bottom {
            target.padding.bottom = Some(value);
        }
        if let Some(value) = style.padding.left {
            target.padding.left = Some(value);
        }
        if let Some(value) = style.margin.top {
            target.margin.top = Some(value);
        }
        if let Some(value) = style.margin.right {
            target.margin.right = Some(value);
        }
        if let Some(value) = style.margin.bottom {
            target.margin.bottom = Some(value);
        }
        if let Some(value) = style.margin.left {
            target.margin.left = Some(value);
        }
        if let Some(value) = style.background_color {
            target.background = Some(value.into());
        }
        if let Some(value) = style.border_widths.top {
            target.border_widths.top = Some(value);
        }
        if let Some(value) = style.border_widths.right {
            target.border_widths.right = Some(value);
        }
        if let Some(value) = style.border_widths.bottom {
            target.border_widths.bottom = Some(value);
        }
        if let Some(value) = style.border_widths.left {
            target.border_widths.left = Some(value);
        }
        if let Some(value) = style.border_color {
            target.border_color = Some(value);
        }
        if let Some(value) = style.border_radii.top_left {
            target.corner_radii.top_left = Some(value);
        }
        if let Some(value) = style.border_radii.top_right {
            target.corner_radii.top_right = Some(value);
        }
        if let Some(value) = style.border_radii.bottom_right {
            target.corner_radii.bottom_right = Some(value);
        }
        if let Some(value) = style.border_radii.bottom_left {
            target.corner_radii.bottom_left = Some(value);
        }
        if let Some(value) = style.opacity {
            target.opacity = Some(value);
        }
    }

    if let Some(value) = style.color {
        element = element.text_color(value);
    }
    if let Some(value) = style.font_size {
        element = element.text_size(value);
    }
    if let Some(value) = style.font_weight {
        element = element.font_weight(value);
    }
    if let Some(value) = style.font_style {
        element = match value {
            FontStyle::Italic => element.italic(),
            FontStyle::Normal => element.not_italic(),
            FontStyle::Oblique => element,
        };
    }
    if style.underline == Some(true) {
        element = element.underline();
    }

    if let Some(radius) = style.backdrop_blur.filter(|radius| *radius > 0.0) {
        element = element.backdrop_blur(px(radius));
    }

    if style.box_shadow.is_empty() {
        element = element.shadow_none();
    } else {
        element = element.shadow(style.box_shadow.clone());
    }

    element
}

/// Apply styles for View wrapper elements.
///
/// Flex items with `flex-grow` default to `min-width: auto`, which prevents
/// them from shrinking below their content width. Default `minWidth: 0` on
/// growing flex children lets nested long text wrap instead of overflowing.
pub fn apply_view_style<T: Styled>(element: T, style: &RenderStyle) -> T {
    let element = if style.min_width.is_none() && style.flex_grow.is_some() {
        element.min_w(px(0.))
    } else {
        element
    };
    apply_style(element, style)
}

pub fn has_scroll_overflow(style: &RenderStyle) -> bool {
    matches!(style.overflow_x, Some(Overflow::Scroll))
        || matches!(style.overflow_y, Some(Overflow::Scroll))
}

pub fn has_horizontal_scroll_overflow(style: &RenderStyle) -> bool {
    matches!(style.overflow_x, Some(Overflow::Scroll))
}

pub fn has_vertical_scroll_overflow(style: &RenderStyle) -> bool {
    matches!(style.overflow_y, Some(Overflow::Scroll))
}

fn apply_edge_override<T: Copy>(
    edges: &mut RenderEdges<T>,
    style: &BTreeMap<String, NodeValue>,
    top: &str,
    right: &str,
    bottom: &str,
    left: &str,
    parse: impl Fn(&NodeValue) -> Option<T>,
) {
    if let Some(value) = style.get(top).and_then(&parse) {
        edges.top = Some(value);
    }
    if let Some(value) = style.get(right).and_then(&parse) {
        edges.right = Some(value);
    }
    if let Some(value) = style.get(bottom).and_then(&parse) {
        edges.bottom = Some(value);
    }
    if let Some(value) = style.get(left).and_then(&parse) {
        edges.left = Some(value);
    }
}

fn apply_corner_override<T: Copy>(
    corners: &mut RenderCorners<T>,
    style: &BTreeMap<String, NodeValue>,
    top_left: &str,
    top_right: &str,
    bottom_right: &str,
    bottom_left: &str,
    parse: impl Fn(&NodeValue) -> Option<T>,
) {
    if let Some(value) = style.get(top_left).and_then(&parse) {
        corners.top_left = Some(value);
    }
    if let Some(value) = style.get(top_right).and_then(&parse) {
        corners.top_right = Some(value);
    }
    if let Some(value) = style.get(bottom_right).and_then(&parse) {
        corners.bottom_right = Some(value);
    }
    if let Some(value) = style.get(bottom_left).and_then(&parse) {
        corners.bottom_left = Some(value);
    }
}

fn edge_value<T: Copy>(
    value: Option<&NodeValue>,
    parse: impl Fn(&NodeValue) -> Option<T>,
) -> RenderEdges<T> {
    match value {
        Some(value) => {
            if let Some(uniform) = parse(value) {
                return RenderEdges {
                    top: Some(uniform),
                    right: Some(uniform),
                    bottom: Some(uniform),
                    left: Some(uniform),
                };
            }
            let NodeValue::Object(object) = value else {
                return RenderEdges::default();
            };
            RenderEdges {
                top: object.get("top").and_then(&parse),
                right: object.get("right").and_then(&parse),
                bottom: object.get("bottom").and_then(&parse),
                left: object.get("left").and_then(&parse),
            }
        }
        None => RenderEdges::default(),
    }
}

fn corner_value<T: Copy>(
    value: Option<&NodeValue>,
    parse: impl Fn(&NodeValue) -> Option<T>,
) -> RenderCorners<T> {
    let Some(value) = value else {
        return RenderCorners::default();
    };
    let Some(uniform) = parse(value) else {
        return RenderCorners::default();
    };
    RenderCorners {
        top_left: Some(uniform),
        top_right: Some(uniform),
        bottom_right: Some(uniform),
        bottom_left: Some(uniform),
    }
}

fn dimension_value(style: &BTreeMap<String, NodeValue>, key: &str) -> Option<Length> {
    style.get(key).and_then(dimension_from_node)
}

fn definite_value(style: &BTreeMap<String, NodeValue>, key: &str) -> Option<DefiniteLength> {
    style.get(key).and_then(definite_from_node)
}

fn dimension_from_node(value: &NodeValue) -> Option<Length> {
    match value {
        NodeValue::Number(value) => Some(Length::Definite(px(*value as f32).into())),
        NodeValue::String(value) => Length::try_from(value.as_str()).ok(),
        _ => None,
    }
}

fn definite_from_node(value: &NodeValue) -> Option<DefiniteLength> {
    match value {
        NodeValue::Number(value) => Some(px(*value as f32).into()),
        NodeValue::String(value) => DefiniteLength::try_from(value.as_str()).ok(),
        _ => None,
    }
}

fn absolute_from_node(value: &NodeValue) -> Option<AbsoluteLength> {
    match value {
        NodeValue::Number(value) => Some(px(*value as f32).into()),
        NodeValue::String(value) => AbsoluteLength::try_from(value.as_str()).ok(),
        _ => None,
    }
}

fn number_value(style: &BTreeMap<String, NodeValue>, key: &str) -> Option<f32> {
    match style.get(key) {
        Some(NodeValue::Number(value)) => Some(*value as f32),
        Some(NodeValue::String(value)) => value.parse().ok(),
        _ => None,
    }
}

fn string_value<'a>(style: &'a BTreeMap<String, NodeValue>, key: &str) -> Option<&'a str> {
    match style.get(key) {
        Some(NodeValue::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

fn parse_display(value: &str) -> Option<Display> {
    match value {
        "flex" => Some(Display::Flex),
        "block" => Some(Display::Block),
        "none" => Some(Display::None),
        _ => None,
    }
}

fn parse_flex_direction(value: &str) -> Option<FlexDirection> {
    match value {
        "row" => Some(FlexDirection::Row),
        "column" => Some(FlexDirection::Column),
        "row-reverse" => Some(FlexDirection::RowReverse),
        "column-reverse" => Some(FlexDirection::ColumnReverse),
        _ => None,
    }
}

fn parse_flex_wrap(value: &str) -> Option<FlexWrap> {
    match value {
        "nowrap" => Some(FlexWrap::NoWrap),
        "wrap" => Some(FlexWrap::Wrap),
        "wrap-reverse" => Some(FlexWrap::WrapReverse),
        _ => None,
    }
}

fn parse_align_items(value: &str) -> Option<AlignItems> {
    match value {
        "stretch" => Some(AlignItems::Stretch),
        "flex-start" => Some(AlignItems::FlexStart),
        "flex-end" => Some(AlignItems::FlexEnd),
        "start" => Some(AlignItems::Start),
        "end" => Some(AlignItems::End),
        "center" => Some(AlignItems::Center),
        "baseline" => Some(AlignItems::Baseline),
        _ => None,
    }
}

fn parse_align_content(value: &str) -> Option<AlignContent> {
    match value {
        "stretch" => Some(AlignContent::Stretch),
        "flex-start" => Some(AlignContent::FlexStart),
        "flex-end" => Some(AlignContent::FlexEnd),
        "start" => Some(AlignContent::Start),
        "end" => Some(AlignContent::End),
        "center" => Some(AlignContent::Center),
        "space-between" => Some(AlignContent::SpaceBetween),
        "space-around" => Some(AlignContent::SpaceAround),
        "space-evenly" => Some(AlignContent::SpaceEvenly),
        _ => None,
    }
}

fn parse_justify_content(value: &str) -> Option<JustifyContent> {
    parse_align_content(value)
}

fn parse_position(value: &str) -> Option<Position> {
    match value {
        "relative" => Some(Position::Relative),
        "absolute" => Some(Position::Absolute),
        _ => None,
    }
}

fn parse_overflow(value: &str) -> Option<Overflow> {
    match value {
        "visible" => Some(Overflow::Visible),
        "hidden" => Some(Overflow::Hidden),
        "clip" => Some(Overflow::Clip),
        "scroll" | "auto" => Some(Overflow::Scroll),
        _ => None,
    }
}

pub(crate) fn parse_color(value: &str) -> Option<Hsla> {
    let color = csscolorparser::parse(value).ok()?;
    Some(Hsla::from(Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }))
}

fn parse_font_weight(value: &NodeValue) -> Option<FontWeight> {
    match value {
        NodeValue::String(value) if value == "normal" => Some(FontWeight::NORMAL),
        NodeValue::String(value) if value == "bold" => Some(FontWeight::BOLD),
        NodeValue::String(value) => value.parse::<f32>().ok().map(FontWeight),
        NodeValue::Number(value) => Some(FontWeight(*value as f32)),
        _ => None,
    }
}

fn parse_font_style(value: &str) -> Option<FontStyle> {
    match value {
        "normal" => Some(FontStyle::Normal),
        "italic" => Some(FontStyle::Italic),
        _ => None,
    }
}

fn parse_text_decoration(value: &str) -> Option<bool> {
    match value {
        "underline" => Some(true),
        "none" => Some(false),
        _ => None,
    }
}

fn parse_box_shadow(value: &NodeValue) -> Vec<BoxShadow> {
    match value {
        NodeValue::Null => Vec::new(),
        NodeValue::String(value) => parse_box_shadow_string(value),
        NodeValue::Array(items) => items.iter().flat_map(parse_box_shadow).collect(),
        NodeValue::Object(object) => parse_box_shadow_object(object)
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

fn parse_box_shadow_string(value: &str) -> Vec<BoxShadow> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
        return Vec::new();
    }
    if let Some(preset) = box_shadow_preset(trimmed) {
        return preset;
    }
    split_box_shadow_layers(trimmed)
        .into_iter()
        .filter_map(|layer| parse_box_shadow_css_layer(&layer))
        .collect()
}

fn parse_box_shadow_object(object: &BTreeMap<String, NodeValue>) -> Option<BoxShadow> {
    let offset_x = number_field(object, "offsetX").unwrap_or(0.0);
    let offset_y = number_field(object, "offsetY").unwrap_or(0.0);
    let blur_radius = number_field(object, "blurRadius").unwrap_or(0.0);
    let spread_radius = number_field(object, "spreadRadius").unwrap_or(0.0);
    let color = object
        .get("color")
        .and_then(|value| value.as_str())
        .and_then(parse_color)
        .unwrap_or_else(|| hsla(0., 0., 0., 0.25));
    Some(box_shadow(
        offset_x,
        offset_y,
        blur_radius,
        spread_radius,
        color,
    ))
}

fn box_shadow_preset(name: &str) -> Option<Vec<BoxShadow>> {
    Some(match name {
        "xs" => vec![box_shadow(0., 1., 2., 0., hsla(0., 0., 0., 0.05))],
        "sm" => vec![
            box_shadow(0., 1., 3., 0., hsla(0., 0., 0., 0.1)),
            box_shadow(0., 1., 2., -1., hsla(0., 0., 0., 0.1)),
        ],
        "md" => vec![
            box_shadow(0., 4., 6., -1., hsla(0., 0., 0., 0.1)),
            box_shadow(0., 2., 4., -2., hsla(0., 0., 0., 0.1)),
        ],
        "lg" => vec![
            box_shadow(0., 10., 15., -3., hsla(0., 0., 0., 0.1)),
            box_shadow(0., 4., 6., -4., hsla(0., 0., 0., 0.1)),
        ],
        "xl" => vec![
            box_shadow(0., 20., 25., -5., hsla(0., 0., 0., 0.1)),
            box_shadow(0., 8., 10., -6., hsla(0., 0., 0., 0.1)),
        ],
        _ => return None,
    })
}

fn parse_box_shadow_css_layer(layer: &str) -> Option<BoxShadow> {
    let mut tokens = tokenize_css_layer(layer);
    if tokens.is_empty() {
        return None;
    }
    if tokens[0].eq_ignore_ascii_case("inset") {
        tokens.remove(0);
    }
    if tokens.len() < 3 {
        return None;
    }
    let color = parse_color(&tokens.pop()?).or_else(|| {
        let second = tokens.pop()?;
        let first = tokens.pop()?;
        let combined = format!("{first} {second}");
        parse_color(&combined)
    })?;
    let (offset_x, offset_y, blur_radius, spread_radius) = match tokens.len() {
        4 => (
            parse_length_token(&tokens[0])?,
            parse_length_token(&tokens[1])?,
            parse_length_token(&tokens[2])?,
            parse_length_token(&tokens[3])?,
        ),
        3 => (
            parse_length_token(&tokens[0])?,
            parse_length_token(&tokens[1])?,
            parse_length_token(&tokens[2])?,
            0.0,
        ),
        2 => (
            parse_length_token(&tokens[0])?,
            parse_length_token(&tokens[1])?,
            0.0,
            0.0,
        ),
        _ => return None,
    };
    Some(box_shadow(
        offset_x,
        offset_y,
        blur_radius,
        spread_radius,
        color,
    ))
}

fn split_box_shadow_layers(value: &str) -> Vec<String> {
    let mut layers = Vec::new();
    let mut current = String::new();
    let mut paren_depth: i32 = 0;
    for ch in value.chars() {
        match ch {
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if paren_depth == 0 => {
                if !current.trim().is_empty() {
                    layers.push(current.trim().to_owned());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        layers.push(current.trim().to_owned());
    }
    layers
}

fn tokenize_css_layer(layer: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut paren_depth: i32 = 0;
    for ch in layer.trim().chars() {
        if ch == '(' {
            paren_depth += 1;
            current.push(ch);
        } else if ch == ')' {
            paren_depth = paren_depth.saturating_sub(1);
            current.push(ch);
        } else if ch.is_whitespace() && paren_depth == 0 {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_length_token(token: &str) -> Option<f32> {
    let trimmed = token.trim();
    if trimmed == "0" {
        return Some(0.0);
    }
    if let Some(value) = trimmed.strip_suffix("px") {
        return value.parse().ok();
    }
    trimmed.parse().ok()
}

fn number_field(object: &BTreeMap<String, NodeValue>, key: &str) -> Option<f32> {
    match object.get(key) {
        Some(NodeValue::Number(value)) => Some(*value as f32),
        Some(NodeValue::String(value)) => value.parse().ok(),
        _ => None,
    }
}

fn box_shadow(
    offset_x: f32,
    offset_y: f32,
    blur_radius: f32,
    spread_radius: f32,
    color: Hsla,
) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(offset_x), px(offset_y)),
        blur_radius: px(blur_radius),
        spread_radius: px(spread_radius),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shadow_alpha(shadow: &BoxShadow) -> f32 {
        shadow.color.a
    }

    fn shadow_offset_y(shadow: &BoxShadow) -> f32 {
        f32::from(shadow.offset.y)
    }

    fn shadow_blur(shadow: &BoxShadow) -> f32 {
        f32::from(shadow.blur_radius)
    }

    #[test]
    fn parses_css_box_shadow_string() {
        let shadows = parse_box_shadow_string("0 8px 16px rgba(0, 0, 0, 0.04)");
        assert_eq!(shadows.len(), 1);
        assert!((shadow_offset_y(&shadows[0]) - 8.0).abs() < f32::EPSILON);
        assert!((shadow_blur(&shadows[0]) - 16.0).abs() < f32::EPSILON);
        assert!((shadow_alpha(&shadows[0]) - 0.04).abs() < 0.001);
    }

    #[test]
    fn parses_sm_preset() {
        let shadows = parse_box_shadow_string("sm");
        assert_eq!(shadows.len(), 2);
    }

    #[test]
    fn parses_structured_box_shadow_object() {
        let mut object = BTreeMap::new();
        object.insert("offsetY".to_owned(), NodeValue::Number(8.0));
        object.insert("blurRadius".to_owned(), NodeValue::Number(16.0));
        object.insert(
            "color".to_owned(),
            NodeValue::String("rgba(0, 0, 0, 0.04)".to_owned()),
        );
        let shadows = parse_box_shadow(&NodeValue::Object(object));
        assert_eq!(shadows.len(), 1);
        assert!((shadow_offset_y(&shadows[0]) - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn parses_multiple_css_layers() {
        let shadows =
            parse_box_shadow_string("0 1px 3px rgba(0,0,0,0.1), 0 8px 16px rgba(0,0,0,0.04)");
        assert_eq!(shadows.len(), 2);
    }

    #[test]
    fn none_and_missing_clear_shadow() {
        assert!(parse_box_shadow_string("none").is_empty());
        assert!(parse_box_shadow_string("").is_empty());
        assert!(parse_render_style(&BTreeMap::new()).box_shadow.is_empty());
    }

    #[test]
    fn invalid_css_box_shadow_is_ignored() {
        assert!(parse_box_shadow_string("not-a-shadow").is_empty());
    }
}
