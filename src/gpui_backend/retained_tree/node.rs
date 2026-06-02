use crate::common::{
    ids::{NativeObjectId, SurfaceId},
    mount::{NodePayload, RetainedNodeKind},
};

use crate::gpui_backend::render_model::{build::build_render_model, model::RenderModel};

/// A long-lived Raster object owned by the GPUI app thread.
#[derive(Debug, Clone, PartialEq)]
pub struct RetainedNode {
    pub id: NativeObjectId,
    pub surface_id: SurfaceId,
    pub kind: RetainedNodeKind,
    pub name: String,
    pub key: Option<String>,
    pub parent: Option<NativeObjectId>,
    pub children: Vec<NativeObjectId>,
    pub payload: NodePayload,
    pub render_model: RenderModel,
}

impl RetainedNode {
    pub fn component_name(&self) -> &str {
        if matches!(
            self.kind,
            RetainedNodeKind::Input | RetainedNodeKind::Textarea
        ) {
            return &self.name;
        }
        if self.kind.is_widget() {
            return self
                .payload
                .props
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or(&self.name);
        }
        &self.name
    }

    pub fn new(
        id: NativeObjectId,
        surface_id: SurfaceId,
        kind: RetainedNodeKind,
        name: impl Into<String>,
        key: Option<String>,
        payload: NodePayload,
    ) -> Self {
        let name = name.into();
        let render_model = build_render_model(&kind, &name, &payload);
        Self {
            id,
            surface_id,
            kind,
            name,
            key,
            parent: None,
            children: Vec::new(),
            payload,
            render_model,
        }
    }

    pub fn replace_payload(&mut self, payload: NodePayload) {
        self.render_model = build_render_model(&self.kind, &self.name, &payload);
        self.payload = payload;
    }

    pub fn update_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.payload.text = Some(text.clone());
        self.render_model = build_render_model(&self.kind, &self.name, &self.payload);
    }

    pub fn is_owner_boundary(&self) -> bool {
        matches!(
            self.kind,
            RetainedNodeKind::Input | RetainedNodeKind::Textarea
        ) || (self.kind == RetainedNodeKind::Widget
            && self.component_name() != "ConfigProvider"
            && self.component_name() != "Alert"
            && self.component_name() != "Dialog"
            && self.component_name() != "Sheet")
    }
}
