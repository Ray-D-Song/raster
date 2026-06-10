use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use gpui::{AnyElement, BoxShadow, IntoElement, div, hsla, point, prelude::*, px};

use crate::common::ids::NativeObjectId;

const HIGHLIGHT_TTL: Duration = Duration::from_millis(360);

#[derive(Debug, Clone, Copy)]
pub struct PerfHighlight {
    pub count: u32,
}

#[derive(Debug, Clone, Copy)]
struct PerfEntry {
    count: u32,
    expires_at: Instant,
}

/// Tracks recent owner refreshes for the opt-in perf overlay.
#[derive(Debug, Default)]
pub struct PerfMonitor {
    enabled: bool,
    nodes: BTreeMap<NativeObjectId, PerfEntry>,
}

impl PerfMonitor {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            nodes: BTreeMap::new(),
        }
    }

    pub fn record_dirty(&mut self, node_id: NativeObjectId) -> Option<Instant> {
        if !self.enabled {
            return None;
        }

        let now = Instant::now();
        let entry = self.nodes.entry(node_id).or_insert(PerfEntry {
            count: 0,
            expires_at: now,
        });
        if now > entry.expires_at {
            entry.count = 0;
        }
        entry.count = entry.count.saturating_add(1);
        entry.expires_at = now + HIGHLIGHT_TTL;
        Some(entry.expires_at)
    }

    pub fn highlight(&mut self, node_id: NativeObjectId) -> Option<PerfHighlight> {
        if !self.enabled {
            return None;
        }

        let now = Instant::now();
        let entry = self.nodes.get(&node_id).copied()?;
        if now >= entry.expires_at {
            self.nodes.remove(&node_id);
            return None;
        }
        Some(PerfHighlight { count: entry.count })
    }
}

pub fn decorate_refresh(content: AnyElement, highlight: Option<PerfHighlight>) -> AnyElement {
    let Some(highlight) = highlight else {
        return content;
    };

    let color = match highlight.count {
        0 | 1 => hsla(142.0 / 360.0, 0.72, 0.45, 0.9),
        2..=4 => hsla(42.0 / 360.0, 0.95, 0.55, 0.95),
        _ => hsla(0.0, 0.82, 0.58, 0.95),
    };

    div()
        .relative()
        .shadow(vec![BoxShadow {
            color,
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(2.0),
        }])
        .child(content)
        .into_any_element()
}
