use std::borrow::Cow;

use anyhow::anyhow;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use gpui::{AssetSource, Result, SharedString};

pub struct RasterAssets {
    base: gpui_component_assets::Assets,
}

impl RasterAssets {
    pub fn new(base: gpui_component_assets::Assets) -> Self {
        Self { base }
    }
}

impl AssetSource for RasterAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = load_data_url(path)? {
            return Ok(Some(bytes));
        }

        self.base.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        self.base.list(path)
    }
}

fn load_data_url(path: &str) -> Result<Option<Cow<'static, [u8]>>> {
    let Some(payload) = path.strip_prefix("data:") else {
        return Ok(None);
    };

    let (metadata, data) = payload
        .split_once(',')
        .ok_or_else(|| anyhow!("invalid data URL: missing comma separator"))?;

    let bytes = if metadata.ends_with(";base64") {
        Cow::Owned(STANDARD.decode(data)?)
    } else {
        Cow::Owned(percent_decode(data).into_bytes())
    };

    Ok(Some(bytes))
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
    fn raster_assets_loads_percent_encoded_svg_data_url() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        let path = format!("data:image/svg+xml;charset=utf-8,{}", url_encode(svg));
        let assets = RasterAssets::new(gpui_component_assets::Assets::new(""));

        let loaded = assets
            .load(&path)
            .expect("load should succeed")
            .expect("data url should exist");
        assert_eq!(loaded.as_ref(), svg.as_bytes());
    }

    #[test]
    fn raster_assets_loads_base64_svg_data_url() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        let encoded = STANDARD.encode(svg.as_bytes());
        let path = format!("data:image/svg+xml;base64,{encoded}");
        let assets = RasterAssets::new(gpui_component_assets::Assets::new(""));

        let loaded = assets
            .load(&path)
            .expect("load should succeed")
            .expect("data url should exist");
        assert_eq!(loaded.as_ref(), svg.as_bytes());
    }

    fn url_encode(input: &str) -> String {
        input
            .bytes()
            .map(|byte| match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    (byte as char).to_string()
                }
                _ => format!("%{byte:02X}"),
            })
            .collect()
    }
}