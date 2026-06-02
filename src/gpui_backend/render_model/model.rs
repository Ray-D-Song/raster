use crate::gpui_backend::render_model::style::RenderStyle;

/// GPUI-facing model derived from retained node payloads.
///
/// The model is deliberately plain data. GPUI render code should read this
/// data and produce temporary elements without reparsing JS props.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderModel {
    View(ViewModel),
    Label(LabelModel),
    Widget(WidgetModel),
    Fragment,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ViewModel {
    pub style: RenderStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LabelModel {
    pub text: String,
    pub style: RenderStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetModel {
    pub component_name: String,
    pub style: RenderStyle,
}
