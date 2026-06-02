use crate::gpui_backend::components::input::{TextControlSyncDecision, decide_text_control_sync};

#[test]
fn text_control_event_count_guards_native_value_sync() {
    assert_eq!(
        decide_text_control_sync(1, 2, "old retained value", "native edit"),
        TextControlSyncDecision::SkipStale
    );

    assert_eq!(
        decide_text_control_sync(2, 2, "native edit", "native edit"),
        TextControlSyncDecision::Ack
    );

    assert_eq!(
        decide_text_control_sync(3, 2, "external clear", "native edit"),
        TextControlSyncDecision::Apply
    );
}
