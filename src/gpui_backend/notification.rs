use std::{
    collections::{BTreeMap, VecDeque},
    time::Duration,
};

use gpui::{
    AppContext, Context, DismissEvent, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled, Subscription, Window, div,
};
use gpui_component::{
    notification::{Notification, NotificationType as GpuiNotificationType},
    v_flex,
};

use crate::common::channel::{NotificationCommandPayload, NotificationType, UiCommand};

pub struct RasterNotificationCenter {
    order: VecDeque<String>,
    notifications: BTreeMap<String, Entity<Notification>>,
    subscriptions: BTreeMap<String, Subscription>,
    next_id: u64,
}

impl RasterNotificationCenter {
    pub fn new() -> Self {
        Self {
            order: VecDeque::new(),
            notifications: BTreeMap::new(),
            subscriptions: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn apply_command(&mut self, command: UiCommand, cx: &mut Context<Self>) {
        match command {
            UiCommand::ShowNotification(payload) => self.show(payload, cx),
            UiCommand::DismissNotification { id } => self.dismiss(&id, cx),
            UiCommand::ClearNotifications => self.clear(cx),
            UiCommand::ChartAppendData { .. }
            | UiCommand::ChartReplaceData { .. }
            | UiCommand::ChartClearData { .. } => {}
        }
    }

    fn show(&mut self, payload: NotificationCommandPayload, cx: &mut Context<Self>) {
        let id = payload.id.unwrap_or_else(|| {
            let id = format!("__raster_notification_{}", self.next_id);
            self.next_id += 1;
            id
        });

        self.remove_entry(&id);

        let mut notification = Notification::new()
            .message(payload.message)
            .with_type(notification_type(payload.type_));
        if let Some(title) = payload.title {
            notification = notification.title(title);
        }

        let entity = cx.new(|_| notification);
        self.subscriptions.insert(
            id.clone(),
            cx.subscribe(&entity, {
                let id = id.clone();
                move |view, _, _: &DismissEvent, cx| {
                    view.remove_entry(&id);
                    cx.notify();
                }
            }),
        );
        self.order.push_back(id.clone());
        self.notifications.insert(id.clone(), entity.clone());

        if payload.autohide {
            cx.spawn(async move |view, cx| {
                cx.background_executor().timer(Duration::from_secs(5)).await;
                cx.update(|cx| {
                    if let Some(view) = view.upgrade() {
                        view.update(cx, |view, cx| {
                            view.remove_entity(&entity);
                            cx.notify();
                        });
                    }
                })
            })
            .detach();
        }

        cx.notify();
    }

    fn dismiss(&mut self, id: &str, cx: &mut Context<Self>) {
        self.remove_entry(id);
        cx.notify();
    }

    fn clear(&mut self, cx: &mut Context<Self>) {
        self.order.clear();
        self.notifications.clear();
        self.subscriptions.clear();
        cx.notify();
    }

    fn remove_entity(&mut self, entity: &Entity<Notification>) {
        let id = self
            .notifications
            .iter()
            .find_map(|(id, current)| (current == entity).then(|| id.clone()));
        if let Some(id) = id {
            self.remove_entry(&id);
        }
    }

    fn remove_entry(&mut self, id: &str) {
        self.order.retain(|current| current != id);
        self.notifications.remove(id);
        self.subscriptions.remove(id);
    }
}

impl Render for RasterNotificationCenter {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let size = window.viewport_size();
        let items = self
            .order
            .iter()
            .rev()
            .take(10)
            .rev()
            .filter_map(|id| self.notifications.get(id).cloned());

        div().absolute().top_4().right_4().child(
            v_flex()
                .id("raster-notification-list")
                .h(size.height - gpui::px(8.))
                .gap_3()
                .children(items),
        )
    }
}

fn notification_type(value: NotificationType) -> GpuiNotificationType {
    match value {
        NotificationType::Info => GpuiNotificationType::Info,
        NotificationType::Success => GpuiNotificationType::Success,
        NotificationType::Warning => GpuiNotificationType::Warning,
        NotificationType::Error => GpuiNotificationType::Error,
    }
}
