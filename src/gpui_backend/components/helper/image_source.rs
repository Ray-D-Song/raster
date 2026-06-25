use std::path::PathBuf;
use std::sync::Arc;

use gpui::{ImageSource, Resource};

use crate::gpui_backend::asset_context::current_render_image;

#[derive(Debug, PartialEq, Eq)]
pub(in crate::gpui_backend) enum ImageSrcKind {
    Remote,
    File(PathBuf),
    Embedded(String),
    Data(String),
    Invalid,
}

pub(in crate::gpui_backend) fn is_remote_uri(src: &str) -> bool {
    matches!(classify_image_src(src), ImageSrcKind::Remote)
}

pub(in crate::gpui_backend) fn classify_image_src(src: &str) -> ImageSrcKind {
    if src.starts_with("http://") || src.starts_with("https://") {
        return ImageSrcKind::Remote;
    }

    if let Some(rest) = src.strip_prefix("file://") {
        return file_url_to_path(rest)
            .map(ImageSrcKind::File)
            .unwrap_or(ImageSrcKind::Invalid);
    }

    if let Some(path) = src.strip_prefix("embed://") {
        if path.is_empty() {
            return ImageSrcKind::Invalid;
        }
        return ImageSrcKind::Embedded(path.to_owned());
    }

    if src.starts_with("data:") {
        return ImageSrcKind::Data(src.to_owned());
    }

    if let Some((scheme, _)) = src.split_once("://") {
        if is_known_uri_scheme(scheme) {
            return ImageSrcKind::Invalid;
        }
    }

    ImageSrcKind::Embedded(src.to_owned())
}

pub(in crate::gpui_backend) fn resource_from_kind(kind: &ImageSrcKind) -> Option<Resource> {
    match kind {
        ImageSrcKind::Remote => None,
        ImageSrcKind::File(path) => Some(Resource::Path(Arc::from(path.as_path()))),
        ImageSrcKind::Embedded(path) => Some(Resource::Embedded(path.clone().into())),
        ImageSrcKind::Data(url) => Some(Resource::Embedded(url.clone().into())),
        ImageSrcKind::Invalid => None,
    }
}

pub(in crate::gpui_backend) fn resource_from_src(src: &str) -> Option<Resource> {
    resource_from_kind(&classify_image_src(src))
}

pub(in crate::gpui_backend) fn resolve_image_source(src: &str) -> Option<ImageSource> {
    match classify_image_src(src) {
        ImageSrcKind::Remote => current_render_image(src).map(ImageSource::Render),
        ImageSrcKind::File(_) | ImageSrcKind::Embedded(_) | ImageSrcKind::Data(_) => {
            resource_from_src(src).map(ImageSource::Resource)
        }
        ImageSrcKind::Invalid => None,
    }
}

fn is_known_uri_scheme(scheme: &str) -> bool {
    !scheme.is_empty()
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

fn file_url_to_path(url_path: &str) -> Option<PathBuf> {
    let decoded = percent_decode(url_path);
    let normalized = normalize_file_url_path(&decoded);
    if normalized.is_empty() {
        return None;
    }
    Some(PathBuf::from(normalized))
}

fn normalize_file_url_path(path: &str) -> String {
    let path = path.trim_start_matches("//localhost");
    let path = path.trim_start_matches("//");

    #[cfg(windows)]
    {
        if let Some(rest) = path.strip_prefix('/') {
            if rest.len() >= 2 && rest.as_bytes()[1] == b':' {
                return rest.to_owned();
            }
        }
    }

    path.to_owned()
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&input[index + 1..index + 3], 16) {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_remote_uri() {
        assert_eq!(
            classify_image_src("https://example.com/a.png"),
            ImageSrcKind::Remote
        );
    }

    #[test]
    fn classifies_file_uri() {
        assert!(matches!(
            classify_image_src("file:///tmp/x.png"),
            ImageSrcKind::File(path) if path == PathBuf::from("/tmp/x.png")
        ));
    }

    #[test]
    fn classifies_embed_uri() {
        assert_eq!(
            classify_image_src("embed://icons/foo.svg"),
            ImageSrcKind::Embedded("icons/foo.svg".to_owned())
        );
    }

    #[test]
    fn classifies_bare_path_as_embedded() {
        assert_eq!(
            classify_image_src("icons/foo.svg"),
            ImageSrcKind::Embedded("icons/foo.svg".to_owned())
        );
    }

    #[test]
    fn classifies_unknown_scheme_as_invalid() {
        assert_eq!(
            classify_image_src("unknown://foo"),
            ImageSrcKind::Invalid
        );
    }

    #[test]
    fn classifies_data_uri() {
        assert!(matches!(
            classify_image_src("data:image/png;base64,abc"),
            ImageSrcKind::Data(_)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn classifies_windows_file_uri() {
        assert!(matches!(
            classify_image_src("file:///C:/Users/x.png"),
            ImageSrcKind::File(path) if path == PathBuf::from("C:/Users/x.png")
        ));
    }
}