use gpui::ImageSource;

use crate::gpui_backend::asset_context::current_render_image;

pub(in crate::gpui_backend) fn is_remote_uri(src: &str) -> bool {
    src.starts_with("http://") || src.starts_with("https://")
}

pub(in crate::gpui_backend) fn resolve_image_source(src: &str) -> Option<ImageSource> {
    if is_remote_uri(src) {
        current_render_image(src).map(ImageSource::Render)
    } else {
        Some(ImageSource::from(src))
    }
}