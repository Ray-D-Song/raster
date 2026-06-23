use chrono::Local;
use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, Render, Subscription, Window, div, px,
};
use gpui_component::{
    ActiveTheme as _, Sizable as _,
    time_picker::{TimeFormat, TimePicker, TimePickerEvent, TimePickerState},
    v_flex,
};

use crate::section;

pub struct TimePickerStory {
    time_picker: Entity<TimePickerState>,
    time_picker_hm: Entity<TimePickerState>,
    without_appearance_picker: Entity<TimePickerState>,
    time_picker_value: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for TimePickerStory {
    fn title() -> &'static str {
        "TimePicker"
    }

    fn description() -> &'static str {
        "A time picker with hour, minute, and optional second columns."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl TimePickerStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let now = Local::now().time();
        let time_picker = cx.new(|cx| {
            let mut picker = TimePickerState::new(cx);
            picker.set_time(Some(now), false, window, cx);
            picker
        });
        let time_picker_hm = cx.new(|cx| {
            let mut picker = TimePickerState::new(cx).format(TimeFormat::Hm);
            picker.set_time(Some(now), false, window, cx);
            picker
        });
        let without_appearance_picker = cx.new(|cx| TimePickerState::new(cx));

        let _subscriptions = vec![
            cx.subscribe(&time_picker, |this, _, event: &TimePickerEvent, _| {
                if let TimePickerEvent::Change(time) = event {
                    this.time_picker_value = time.map(|value| value.format("%H:%M:%S").to_string());
                }
            }),
        ];

        Self {
            time_picker,
            time_picker_hm,
            without_appearance_picker,
            time_picker_value: None,
            _subscriptions,
        }
    }
}

impl Focusable for TimePickerStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.time_picker.focus_handle(cx)
    }
}

impl Render for TimePickerStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                section("HH:mm:ss")
                    .max_w_128()
                    .child(TimePicker::new(&self.time_picker).cleanable(true).w(px(180.))),
            )
            .child(
                section("HH:mm")
                    .max_w_128()
                    .child(TimePicker::new(&self.time_picker_hm).cleanable(true).w(px(180.))),
            )
            .child(
                section("Time Picker Value").max_w_128().child(
                    format!("Time picker value: {:?}", self.time_picker_value).into_element(),
                ),
            )
            .child(
                section("Without Appearance").max_w_128().child(
                    div().w_full().bg(cx.theme().secondary).child(
                        TimePicker::new(&self.without_appearance_picker)
                            .appearance(false)
                            .placeholder("Without appearance"),
                    ),
                ),
            )
    }
}