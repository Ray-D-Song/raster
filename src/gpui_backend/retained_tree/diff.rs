use crate::common::mount::NodePayload;

/// Result of comparing a retained node payload update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodePayloadChange {
    Noop,
    HandlerOnly,
    Visual,
}

/// Classifies whether an UpdateNode should repaint its owner.
pub fn diff_node_payload(old: &NodePayload, next: &NodePayload) -> NodePayloadChange {
    if old == next {
        return NodePayloadChange::Noop;
    }

    if old.props == next.props && old.style == next.style && old.text == next.text {
        return NodePayloadChange::HandlerOnly;
    }

    NodePayloadChange::Visual
}
