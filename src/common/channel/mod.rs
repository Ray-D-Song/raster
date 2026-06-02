//! Thread communication primitives shared by the runtime and GPUI backend.

#![allow(dead_code, unused_imports)]

pub mod command;
pub mod commit;
pub mod queue;
pub mod ui;
pub mod wake;

pub use command::{QueryResponder, RuntimeCommand, RuntimeCommandQueue};
pub use commit::CommitQueue;
pub use queue::{ChannelReceiver, ChannelSender, channel};
pub use ui::{NotificationCommandPayload, NotificationType, UiCommand};
pub use wake::{NoopWakeSignal, WakeSignal};
