use std::collections::BTreeMap;

use gpui::{
    AnyElement, Corners, Hsla, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
    transparent_black,
};
use gpui_component::{
    chart::{AreaChart, BarChart, CandlestickChart, LineChart, PieChart},
    plot::shape::BarAlignment,
};

use crate::{
    common::{
        mount::{NodeValue, RetainedNodeKind},
        utils::logger,
    },
    gpui_backend::{
        components::helper::props::{bool_prop, component_props, number_prop, string_prop},
        render_model::{model::RenderModel, style::parse_color},
        retained_tree::node::RetainedNode,
    },
};

type ChartDatum = BTreeMap<String, NodeValue>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChartKind {
    Line,
    Bar,
    Area,
    Pie,
    Candlestick,
}

pub(in crate::gpui_backend) struct RasterChartState {
    data: Vec<ChartDatum>,
    last_data_prop: Option<Vec<ChartDatum>>,
    max_data_length: Option<usize>,
    kind: ChartKind,
}

impl RasterChartState {
    pub(in crate::gpui_backend) fn new(node: &RetainedNode) -> Self {
        let mut state = Self {
            data: Vec::new(),
            last_data_prop: None,
            max_data_length: None,
            kind: chart_kind(node).unwrap_or(ChartKind::Line),
        };
        state.sync_from_node(node);
        state
    }

    pub(in crate::gpui_backend) fn matches_node(&self, node: &RetainedNode) -> bool {
        chart_kind(node).is_some_and(|kind| kind == self.kind)
    }

    pub(in crate::gpui_backend) fn sync_from_node(&mut self, node: &RetainedNode) {
        self.kind = chart_kind(node).unwrap_or(self.kind);
        let props = component_props(node);
        self.max_data_length = number_prop(props, "maxDataLength")
            .map(|value| value.max(0.0) as usize)
            .filter(|value| *value > 0);

        let next_data_prop = data_prop(props.get("data"));
        if next_data_prop != self.last_data_prop {
            self.data = next_data_prop.clone().unwrap_or_default();
            self.last_data_prop = next_data_prop;
            self.trim_to_max();
        }
    }

    pub(in crate::gpui_backend) fn append_data(&mut self, rows: Vec<NodeValue>) {
        self.data
            .extend(rows.into_iter().filter_map(row_from_value));
        self.trim_to_max();
    }

    pub(in crate::gpui_backend) fn replace_data(&mut self, rows: Vec<NodeValue>) {
        self.data = rows.into_iter().filter_map(row_from_value).collect();
        self.last_data_prop = None;
        self.trim_to_max();
    }

    pub(in crate::gpui_backend) fn clear_data(&mut self) {
        self.data.clear();
        self.last_data_prop = None;
    }

    fn trim_to_max(&mut self) {
        let Some(max_len) = self.max_data_length else {
            return;
        };
        if self.data.len() > max_len {
            let drain_count = self.data.len() - max_len;
            self.data.drain(0..drain_count);
        }
    }

    fn data(&self) -> &[ChartDatum] {
        &self.data
    }
}

pub(in crate::gpui_backend) fn is_chart_node(node: &RetainedNode) -> bool {
    chart_kind(node).is_some()
}

pub(in crate::gpui_backend) fn render_chart_from_node(
    node: &RetainedNode,
    state: &RasterChartState,
) -> Option<AnyElement> {
    if !is_chart_node(node) {
        return None;
    }

    let RenderModel::Widget(widget) = &node.render_model else {
        return None;
    };

    let chart = match state.kind {
        ChartKind::Line => render_line_chart(node, state.data()),
        ChartKind::Bar => render_bar_chart(node, state.data()),
        ChartKind::Area => render_area_chart(node, state.data()),
        ChartKind::Pie => render_pie_chart(node, state.data()),
        ChartKind::Candlestick => render_candlestick_chart(node, state.data()),
    };

    let mut wrapper = crate::gpui_backend::render_model::style::apply_style(
        div().id(("raster-chart", node.id.0)),
        &widget.style,
    );
    if widget.style.height.is_none() {
        wrapper = wrapper.h(px(240.0));
    }

    Some(wrapper.child(chart).into_any_element())
}

fn render_line_chart(node: &RetainedNode, data: &[ChartDatum]) -> AnyElement {
    let props = component_props(node);
    let x_key = string_prop(props, "x").unwrap_or_else(|| "x".to_owned());
    let y_key = string_prop(props, "y").unwrap_or_else(|| "y".to_owned());
    let rows = data
        .iter()
        .filter(|row| number_field(row, &y_key).is_some())
        .cloned()
        .collect::<Vec<_>>();

    let mut chart = LineChart::new(rows)
        .x(move |row: &ChartDatum| string_field(row, &x_key))
        .y(move |row: &ChartDatum| number_field(row, &y_key).unwrap_or_default())
        .tick_margin(tick_margin(props))
        .grid(bool_prop(props, "grid").unwrap_or(true))
        .x_axis(bool_prop(props, "xAxis").unwrap_or(true));

    if bool_prop(props, "dot").unwrap_or(false) {
        chart = chart.dot();
    }
    if let Some(color) = string_prop(props, "stroke").and_then(|value| parse_color(&value)) {
        chart = chart.stroke(color);
    }
    match string_prop(props, "interpolation").as_deref() {
        Some("linear") => chart.linear().into_any_element(),
        Some("stepAfter") => chart.step_after().into_any_element(),
        _ => chart.natural().into_any_element(),
    }
}

fn render_bar_chart(node: &RetainedNode, data: &[ChartDatum]) -> AnyElement {
    let props = component_props(node);
    let band_key = string_prop(props, "band").unwrap_or_else(|| "band".to_owned());
    let value_key = string_prop(props, "value").unwrap_or_else(|| "value".to_owned());
    let rows = data
        .iter()
        .filter(|row| number_field(row, &value_key).is_some())
        .cloned()
        .collect::<Vec<_>>();
    let alignment = parse_bar_alignment(
        string_prop(props, "alignment")
            .as_deref()
            .unwrap_or("bottom"),
    );

    let mut chart = BarChart::new(rows)
        .band(move |row: &ChartDatum| string_field(row, &band_key))
        .value(move |row: &ChartDatum| number_field(row, &value_key).unwrap_or_default())
        .tick_margin(tick_margin(props))
        .grid(bool_prop(props, "grid").unwrap_or(true))
        .label_axis(bool_prop(props, "labelAxis").unwrap_or(true))
        .alignment(alignment);

    if let Some(radius) = number_prop(props, "cornerRadius") {
        chart = chart.corner_radii(Corners::all(px(radius as f32)));
    }
    if let Some(label_key) = string_prop(props, "label") {
        chart = chart.label(move |row: &ChartDatum| string_field(row, &label_key));
    }
    if let Some(fill) = string_prop(props, "fill") {
        if let Some(color) = parse_color(&fill) {
            chart = chart.fill(move |_, _, _, _| color);
        } else {
            chart = chart.fill(move |row: &ChartDatum, _, _, _| {
                color_field(row, &fill).unwrap_or_else(transparent_black)
            });
        }
    }

    chart.into_any_element()
}

fn render_area_chart(node: &RetainedNode, data: &[ChartDatum]) -> AnyElement {
    let props = component_props(node);
    let x_key = string_prop(props, "x").unwrap_or_else(|| "x".to_owned());
    let series = area_series(props);
    let rows = data
        .iter()
        .filter(|row| {
            series
                .iter()
                .all(|series| number_field(row, &series.y).is_some())
        })
        .cloned()
        .collect::<Vec<_>>();

    let mut chart = AreaChart::new(rows)
        .x(move |row: &ChartDatum| string_field(row, &x_key))
        .tick_margin(tick_margin(props))
        .grid(bool_prop(props, "grid").unwrap_or(true))
        .x_axis(bool_prop(props, "xAxis").unwrap_or(true));

    for series in series {
        let y_key = series.y.clone();
        chart = chart.y(move |row: &ChartDatum| number_field(row, &y_key).unwrap_or_default());
        if let Some(color) = series.stroke.and_then(|value| parse_color(&value)) {
            chart = chart.stroke(color);
        }
        if let Some(color) = series.fill.and_then(|value| parse_color(&value)) {
            chart = chart.fill(color);
        }
        chart = match series.interpolation.as_deref() {
            Some("linear") => chart.linear(),
            Some("stepAfter") => chart.step_after(),
            _ => chart.natural(),
        };
    }

    chart.into_any_element()
}

fn render_pie_chart(node: &RetainedNode, data: &[ChartDatum]) -> AnyElement {
    let props = component_props(node);
    let value_key = string_prop(props, "value").unwrap_or_else(|| "value".to_owned());
    let rows = data
        .iter()
        .filter(|row| number_field(row, &value_key).is_some())
        .cloned()
        .collect::<Vec<_>>();

    let mut chart = PieChart::new(rows)
        .value(move |row: &ChartDatum| number_field(row, &value_key).unwrap_or_default() as f32);
    if let Some(radius) = number_prop(props, "innerRadius") {
        chart = chart.inner_radius(radius as f32);
    }
    if let Some(radius) = number_prop(props, "outerRadius") {
        chart = chart.outer_radius(radius as f32);
    }
    if let Some(pad_angle) = number_prop(props, "padAngle") {
        chart = chart.pad_angle(pad_angle as f32);
    }
    if let Some(color) = string_prop(props, "color") {
        if let Some(static_color) = parse_color(&color) {
            chart = chart.color(move |_| static_color);
        } else {
            chart = chart.color(move |row: &ChartDatum| {
                color_field(row, &color).unwrap_or_else(transparent_black)
            });
        }
    }

    chart.into_any_element()
}

fn render_candlestick_chart(node: &RetainedNode, data: &[ChartDatum]) -> AnyElement {
    let props = component_props(node);
    let x_key = string_prop(props, "x").unwrap_or_else(|| "x".to_owned());
    let open_key = string_prop(props, "open").unwrap_or_else(|| "open".to_owned());
    let high_key = string_prop(props, "high").unwrap_or_else(|| "high".to_owned());
    let low_key = string_prop(props, "low").unwrap_or_else(|| "low".to_owned());
    let close_key = string_prop(props, "close").unwrap_or_else(|| "close".to_owned());
    let rows = data
        .iter()
        .filter(|row| {
            number_field(row, &open_key).is_some()
                && number_field(row, &high_key).is_some()
                && number_field(row, &low_key).is_some()
                && number_field(row, &close_key).is_some()
        })
        .cloned()
        .collect::<Vec<_>>();

    let open_for_chart = open_key.clone();
    let high_for_chart = high_key.clone();
    let low_for_chart = low_key.clone();
    let close_for_chart = close_key.clone();
    let mut chart = CandlestickChart::new(rows)
        .x(move |row: &ChartDatum| string_field(row, &x_key))
        .open(move |row: &ChartDatum| number_field(row, &open_for_chart).unwrap_or_default())
        .high(move |row: &ChartDatum| number_field(row, &high_for_chart).unwrap_or_default())
        .low(move |row: &ChartDatum| number_field(row, &low_for_chart).unwrap_or_default())
        .close(move |row: &ChartDatum| number_field(row, &close_for_chart).unwrap_or_default())
        .tick_margin(tick_margin(props))
        .grid(bool_prop(props, "grid").unwrap_or(true))
        .x_axis(bool_prop(props, "xAxis").unwrap_or(true));

    if let Some(ratio) = number_prop(props, "bodyWidthRatio") {
        chart = chart.body_width_ratio(ratio as f32);
    }

    chart.into_any_element()
}

#[derive(Clone)]
struct AreaSeriesConfig {
    y: String,
    stroke: Option<String>,
    fill: Option<String>,
    interpolation: Option<String>,
}

fn area_series(props: &BTreeMap<String, NodeValue>) -> Vec<AreaSeriesConfig> {
    if let Some(NodeValue::Array(series)) = props.get("series") {
        let parsed = series
            .iter()
            .filter_map(|series| {
                let NodeValue::Object(series) = series else {
                    return None;
                };
                Some(AreaSeriesConfig {
                    y: series.get("y").map(display_value)?,
                    stroke: series.get("stroke").map(display_value),
                    fill: series.get("fill").map(display_value),
                    interpolation: series.get("interpolation").map(display_value),
                })
            })
            .collect::<Vec<_>>();
        if !parsed.is_empty() {
            return parsed;
        }
    }

    vec![AreaSeriesConfig {
        y: string_prop(props, "y").unwrap_or_else(|| "y".to_owned()),
        stroke: string_prop(props, "stroke"),
        fill: string_prop(props, "fill"),
        interpolation: string_prop(props, "interpolation"),
    }]
}

fn chart_kind(node: &RetainedNode) -> Option<ChartKind> {
    if node.kind != RetainedNodeKind::Widget {
        return None;
    }
    match node.component_name() {
        "LineChart" => Some(ChartKind::Line),
        "BarChart" => Some(ChartKind::Bar),
        "AreaChart" => Some(ChartKind::Area),
        "PieChart" => Some(ChartKind::Pie),
        "CandlestickChart" => Some(ChartKind::Candlestick),
        _ => None,
    }
}

fn data_prop(value: Option<&NodeValue>) -> Option<Vec<ChartDatum>> {
    let NodeValue::Array(rows) = value? else {
        return Some(Vec::new());
    };
    Some(rows.iter().filter_map(row_from_ref).collect())
}

fn row_from_value(value: NodeValue) -> Option<ChartDatum> {
    match value {
        NodeValue::Object(row) => Some(row),
        _ => {
            logger::warn("Chart data row must be an object; row skipped");
            None
        }
    }
}

fn row_from_ref(value: &NodeValue) -> Option<ChartDatum> {
    match value {
        NodeValue::Object(row) => Some(row.clone()),
        _ => {
            logger::warn("Chart data row must be an object; row skipped");
            None
        }
    }
}

fn string_field(row: &ChartDatum, key: &str) -> String {
    row.get(key).map(display_value).unwrap_or_default()
}

fn number_field(row: &ChartDatum, key: &str) -> Option<f64> {
    match row.get(key) {
        Some(NodeValue::Number(value)) => Some(*value),
        Some(NodeValue::String(value)) => value.parse().ok(),
        _ => None,
    }
}

fn color_field(row: &ChartDatum, key: &str) -> Option<Hsla> {
    row.get(key).map(display_value).and_then(|value| {
        let parsed = parse_color(&value);
        if parsed.is_none() {
            logger::warn(format!("invalid Chart color value: {value}"));
        }
        parsed
    })
}

fn tick_margin(props: &BTreeMap<String, NodeValue>) -> usize {
    number_prop(props, "tickMargin")
        .map(|value| value.max(1.0) as usize)
        .unwrap_or(1)
}

fn parse_bar_alignment(value: &str) -> BarAlignment {
    match value {
        "top" => BarAlignment::Top,
        "right" => BarAlignment::Right,
        "left" => BarAlignment::Left,
        _ => BarAlignment::Bottom,
    }
}

fn display_value(value: &NodeValue) -> String {
    crate::gpui_backend::components::helper::props::display_value(value)
}
