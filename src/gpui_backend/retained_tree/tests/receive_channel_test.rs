use crate::{
    common::{
        channel::{CommitQueue, NoopWakeSignal},
        ids::{NativeObjectId, SurfaceId},
        mount::{MountMutation, MountMutationBatch, NodePayload, RetainedNodeKind},
    },
    gpui_backend::{render_model::model::RenderModel, retained_tree::tree::RetainedTree},
};

#[test]
fn retained_tree_applies_batch_received_from_commit_channel() {
    let queue = CommitQueue::new();
    let sender = queue.sender();
    let wake = NoopWakeSignal;
    let surface = SurfaceId(1);
    let view = NativeObjectId(1);
    let text = NativeObjectId(2);

    CommitQueue::submit(
        &sender,
        MountMutationBatch {
            surface_id: surface,
            sequence: 1,
            mutations: vec![
                MountMutation::CreateNode {
                    id: view,
                    kind: RetainedNodeKind::View,
                    name: "View".to_owned(),
                    key: None,
                    payload: NodePayload::default(),
                },
                MountMutation::CreateText {
                    id: text,
                    text: "Hello".to_owned(),
                    payload: NodePayload::default(),
                },
                MountMutation::AppendChild {
                    parent: view,
                    child: text,
                },
                MountMutation::SetRootChildren {
                    surface_id: surface,
                    children: vec![view],
                },
            ],
        },
        &wake,
    )
    .expect("commit batch should be sent");

    let mut tree = RetainedTree::new();
    assert_eq!(tree.create_surface(), surface);
    let mut batches = queue.drain();
    assert_eq!(batches.len(), 1);

    let outcome = tree
        .apply_batch(batches.remove(0))
        .expect("backend should apply JS runtime batch");

    assert!(outcome.dirty_owners().any(|owner| {
        owner == crate::gpui_backend::retained_tree::mutation::OwnerId::Surface(surface)
    }));
    assert_eq!(
        tree.surface(surface).expect("surface should exist").roots,
        vec![view]
    );
    assert_eq!(
        tree.node(view).expect("view should exist").children,
        vec![text]
    );
    let text_node = tree.node(text).expect("text should exist");
    assert_eq!(text_node.parent, Some(view));
    assert_eq!(text_node.payload.text.as_deref(), Some("Hello"));
    assert!(matches!(text_node.render_model, RenderModel::Label(_)));
}
