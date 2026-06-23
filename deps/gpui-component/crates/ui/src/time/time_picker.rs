use chrono::{Local, NaiveTime, Timelike};
use gpui::{
    App, ClickEvent, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement as _, IntoElement, KeyBinding, MouseButton, ParentElement as _, RenderOnce,
    ScrollHandle, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, Window,
    anchored, deferred, div, prelude::FluentBuilder as _, px,
};
use rust_i18n::t;

use crate::{
    ActiveTheme, Disableable, Icon, IconName, Sizable, Size, StyleSized as _, StyledExt as _,
    actions::{Cancel, Confirm},
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Delete, clear_button, input_style},
    v_flex,
};

const CONTEXT: &str = "TimePicker";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("delete", Delete, Some(CONTEXT)),
        KeyBinding::new("backspace", Delete, Some(CONTEXT)),
    ]);
}

/// Display format for [`TimePicker`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeFormat {
    #[default]
    Hms,
    Hm,
}

impl TimeFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hms => "HH:mm:ss",
            Self::Hm => "HH:mm",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "HH:mm" => Self::Hm,
            _ => Self::Hms,
        }
    }

    pub fn chrono_format(self) -> &'static str {
        match self {
            Self::Hms => "%H:%M:%S",
            Self::Hm => "%H:%M",
        }
    }

    fn show_seconds(self) -> bool {
        matches!(self, Self::Hms)
    }
}

/// Events emitted by the [`TimePicker`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimePickerEvent {
    Change(Option<NaiveTime>),
}

/// Stateful time picker.
pub struct TimePickerState {
    focus_handle: FocusHandle,
    committed: Option<NaiveTime>,
    draft: NaiveTime,
    open: bool,
    format: TimeFormat,
    hour_scroll: ScrollHandle,
    minute_scroll: ScrollHandle,
    second_scroll: ScrollHandle,
}

impl Focusable for TimePickerState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<TimePickerEvent> for TimePickerState {}

impl TimePickerState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            committed: None,
            draft: Local::now().time(),
            open: false,
            format: TimeFormat::default(),
            hour_scroll: ScrollHandle::new(),
            minute_scroll: ScrollHandle::new(),
            second_scroll: ScrollHandle::new(),
        }
    }

    pub fn format(mut self, format: TimeFormat) -> Self {
        self.format = format;
        self
    }

    pub fn time(&self) -> Option<NaiveTime> {
        self.committed
    }

    pub fn set_time(
        &mut self,
        time: Option<NaiveTime>,
        emit: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.committed = time;
        if let Some(time) = time {
            self.draft = time;
        }
        self.open = false;
        if emit {
            cx.emit(TimePickerEvent::Change(time));
        }
        cx.notify();
        let _ = window;
    }

    fn sync_scroll_handles(&self) {
        self.hour_scroll.scroll_to_item(self.draft.hour() as usize);
        self.minute_scroll.scroll_to_item(self.draft.minute() as usize);
        if self.format.show_seconds() {
            self.second_scroll.scroll_to_item(self.draft.second() as usize);
        }
    }

    fn open_panel(&mut self, cx: &mut Context<Self>) {
        self.draft = self
            .committed
            .unwrap_or_else(|| Local::now().time());
        self.open = true;
        self.sync_scroll_handles();
        cx.notify();
    }

    fn on_escape(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            cx.propagate();
            return;
        }
        self.focus_back_if_need(window, cx);
        self.open = false;
        cx.notify();
    }

    fn on_enter(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.commit(window, cx);
        } else {
            self.open_panel(cx);
        }
    }

    fn on_delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        self.clean(&ClickEvent::default(), window, cx);
    }

    fn focus_back_if_need(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        if let Some(focused) = window.focused(cx)
            && focused.contains(&self.focus_handle, window)
        {
            self.focus_handle.focus(window, cx);
        }
    }

    fn clean(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        self.set_time(None, true, window, cx);
    }

    fn toggle_panel(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.open = false;
        } else {
            self.open_panel(cx);
        }
        cx.notify();
    }

    fn apply_now(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.draft = Local::now().time();
        self.sync_scroll_handles();
        cx.notify();
    }

    fn confirm(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.commit(window, cx);
    }

    fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set_time(Some(self.draft), true, window, cx);
    }

    fn set_draft_hour(&mut self, hour: u32, cx: &mut Context<Self>) {
        if let Some(time) = self.draft.with_hour(hour) {
            self.draft = time;
            cx.notify();
        }
    }

    fn set_draft_minute(&mut self, minute: u32, cx: &mut Context<Self>) {
        if let Some(time) = self.draft.with_minute(minute) {
            self.draft = time;
            cx.notify();
        }
    }

    fn set_draft_second(&mut self, second: u32, cx: &mut Context<Self>) {
        if let Some(time) = self.draft.with_second(second) {
            self.draft = time;
            cx.notify();
        }
    }
}

/// A time picker element.
#[derive(IntoElement)]
pub struct TimePicker {
    id: ElementId,
    style: StyleRefinement,
    state: Entity<TimePickerState>,
    cleanable: bool,
    placeholder: Option<SharedString>,
    size: Size,
    appearance: bool,
    disabled: bool,
}

impl Sizable for TimePicker {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Focusable for TimePicker {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.read(cx).focus_handle(cx)
    }
}

impl Styled for TimePicker {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for TimePicker {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl TimePicker {
    pub fn new(state: &Entity<TimePickerState>) -> Self {
        Self {
            id: ElementId::Name("time-picker".into()),
            style: StyleRefinement::default(),
            state: state.clone(),
            cleanable: false,
            placeholder: None,
            size: Size::default(),
            appearance: true,
            disabled: false,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }

    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }
}

fn column_item_height(size: Size) -> gpui::Pixels {
    match size {
        Size::XSmall | Size::Small => px(28.),
        Size::Large => px(36.),
        _ => px(32.),
    }
}

fn render_time_column(
    column: &'static str,
    scroll_handle: &ScrollHandle,
    count: usize,
    selected: usize,
    state: &Entity<TimePickerState>,
    size: Size,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let item_height = column_item_height(size);
    let primary = cx.theme().primary;
    let primary_bg = cx.theme().primary.opacity(0.12);
    let muted = cx.theme().muted_foreground;

    div()
        .id(column)
        .flex_1()
        .h(px(200.))
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .children((0..count).map(move |index| {
            let selected_item = index == selected;
            let label = format!("{index:02}");
            div()
                .id((column, index))
                .h(item_height)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .when(selected_item, |this| this.bg(primary_bg).text_color(primary))
                .when(!selected_item, |this| this.text_color(muted))
                .child(label)
                .on_click(window.listener_for(state, move |this, _, _, cx| match column {
                    "hour" => this.set_draft_hour(index as u32, cx),
                    "minute" => this.set_draft_minute(index as u32, cx),
                    _ => this.set_draft_second(index as u32, cx),
                }))
        }))
}

impl RenderOnce for TimePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_focused = self.focus_handle(cx).contains_focused(window, cx);
        let state = self.state.read(cx);
        let show_clean = self.cleanable && state.committed.is_some();
        let placeholder = self
            .placeholder
            .clone()
            .unwrap_or_else(|| t!("TimePicker.placeholder").into());
        let display_title = state
            .committed
            .map(|time| time.format(state.format.chrono_format()).to_string())
            .unwrap_or_else(|| placeholder.to_string());
        let (bg, fg) = input_style(self.disabled, cx);
        let show_seconds = state.format.show_seconds();
        let open = state.open;
        let committed_none = state.committed.is_none();
        let draft_hour = state.draft.hour() as usize;
        let draft_minute = state.draft.minute() as usize;
        let draft_second = state.draft.second() as usize;
        let hour_scroll = state.hour_scroll.clone();
        let minute_scroll = state.minute_scroll.clone();
        let second_scroll = state.second_scroll.clone();

        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            .on_action(window.listener_for(&self.state, TimePickerState::on_enter))
            .on_action(window.listener_for(&self.state, TimePickerState::on_delete))
            .when(open, |this| {
                this.on_action(window.listener_for(&self.state, TimePickerState::on_escape))
            })
            .flex_none()
            .w_full()
            .relative()
            .input_text_size(self.size)
            .refine_style(&self.style)
            .child(
                div()
                    .id("time-picker-input")
                    .relative()
                    .flex()
                    .items_center()
                    .justify_between()
                    .when(self.appearance, |this| {
                        this.bg(bg)
                            .text_color(fg)
                            .when(self.disabled, |this| this.opacity(0.5))
                            .border_1()
                            .border_color(cx.theme().input)
                            .rounded(cx.theme().radius)
                            .when(cx.theme().shadow, |this| this.shadow_xs())
                            .when(is_focused, |this| this.focused_border(cx))
                    })
                    .overflow_hidden()
                    .input_text_size(self.size)
                    .input_size(self.size)
                    .when(!open && !self.disabled, |this| {
                        this.on_click(window.listener_for(&self.state, TimePickerState::toggle_panel))
                    })
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_1()
                            .child(
                                div()
                                    .w_full()
                                    .overflow_hidden()
                                    .when(committed_none, |this| {
                                        this.text_color(cx.theme().muted_foreground)
                                    })
                                    .child(display_title),
                            )
                            .when(!self.disabled, |this| {
                                this.when(show_clean, |this| {
                                    this.child(clear_button(cx).on_click(
                                        window.listener_for(&self.state, TimePickerState::clean),
                                    ))
                                })
                                .when(!show_clean, |this| {
                                    this.child(
                                        Icon::new(IconName::Clock)
                                            .xsmall()
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                })
                            }),
                    ),
            )
            .when(open, |this| {
                this.child(
                    deferred(
                        anchored().snap_to_window_with_margin(px(8.)).child(
                            div()
                                .occlude()
                                .mt_1p5()
                                .p_3()
                                .border_1()
                                .border_color(cx.theme().border)
                                .shadow_lg()
                                .rounded((cx.theme().radius * 2.).min(px(8.)))
                                .bg(cx.theme().popover)
                                .text_color(cx.theme().popover_foreground)
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    window.listener_for(&self.state, |view, _, window, cx| {
                                        view.on_escape(&Cancel, window, cx);
                                    }),
                                )
                                .child(
                                    v_flex()
                                        .gap_2()
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .justify_between()
                                                .child(
                                                    Button::new("time-picker-now")
                                                        .small()
                                                        .ghost()
                                                        .tab_stop(false)
                                                        .label(t!("TimePicker.now"))
                                                        .on_click(window.listener_for(
                                                            &self.state,
                                                            TimePickerState::apply_now,
                                                        )),
                                                )
                                                .child(
                                                    Button::new("time-picker-confirm")
                                                        .small()
                                                        .primary()
                                                        .tab_stop(false)
                                                        .label(t!("TimePicker.confirm"))
                                                        .on_click(window.listener_for(
                                                            &self.state,
                                                            TimePickerState::confirm,
                                                        )),
                                                ),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_1()
                                                .child(render_time_column(
                                                    "hour",
                                                    &hour_scroll,
                                                    24,
                                                    draft_hour,
                                                    &self.state,
                                                    self.size,
                                                    window,
                                                    cx,
                                                ))
                                                .child(render_time_column(
                                                    "minute",
                                                    &minute_scroll,
                                                    60,
                                                    draft_minute,
                                                    &self.state,
                                                    self.size,
                                                    window,
                                                    cx,
                                                ))
                                                .when(show_seconds, |this| {
                                                    this.child(render_time_column(
                                                        "second",
                                                        &second_scroll,
                                                        60,
                                                        draft_second,
                                                        &self.state,
                                                        self.size,
                                                        window,
                                                        cx,
                                                    ))
                                                }),
                                        ),
                                ),
                        ),
                    )
                    .with_priority(2),
                )
            })
    }
}