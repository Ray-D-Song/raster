use std::{
    borrow::Cow,
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{Arc, LazyLock, RwLock},
};

use anyhow::anyhow;
use gpui::{AssetSource, Result, SharedString};

const INLINE_PREFIX: &str = "raster-inline://";
const MAX_INLINE_SVG_CACHE: usize = 256;

#[derive(Default)]
struct InlineSvgCache {
    entries: HashMap<String, Arc<[u8]>>,
    order: Vec<String>,
}

impl InlineSvgCache {
    fn get(&self, key: &str) -> Option<Cow<'static, [u8]>> {
        self.entries
            .get(key)
            .map(|bytes| Cow::Owned(bytes.as_ref().to_vec()))
    }

    fn insert(&mut self, key: String, bytes: Arc<[u8]>) {
        if self.entries.contains_key(&key) {
            return;
        }
        if self.entries.len() >= MAX_INLINE_SVG_CACHE {
            if let Some(oldest) = self.order.first().cloned() {
                self.order.remove(0);
                self.entries.remove(&oldest);
            }
        }
        self.order.push(key.clone());
        self.entries.insert(key, bytes);
    }
}

static INLINE_SVG_CACHE: LazyLock<RwLock<InlineSvgCache>> =
    LazyLock::new(|| RwLock::new(InlineSvgCache::default()));

fn hash_svg(svg: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    svg.hash(&mut hasher);
    hasher.finish()
}

pub fn register_inline_svg(svg: &str) -> SharedString {
    let key = hash_svg(svg).to_string();
    let bytes: Arc<[u8]> = Arc::from(svg.as_bytes());
    INLINE_SVG_CACHE
        .write()
        .expect("inline svg cache lock poisoned")
        .insert(key.clone(), bytes);
    format!("{INLINE_PREFIX}{key}").into()
}

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
        if let Some(key) = path.strip_prefix(INLINE_PREFIX) {
            let cache = INLINE_SVG_CACHE
                .read()
                .map_err(|_| anyhow!("inline svg cache lock poisoned"))?;
            return cache
                .get(key)
                .map(Some)
                .ok_or_else(|| anyhow!("inline svg not found for path \"{path}\""));
        }

        self.base.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        self.base.list(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_inline_svg_deduplicates_by_content() {
        let first = register_inline_svg("<svg>a</svg>");
        let second = register_inline_svg("<svg>a</svg>");
        assert_eq!(first, second);
    }

    #[test]
    fn raster_assets_loads_inline_svg() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        let path = register_inline_svg(svg);
        let assets = RasterAssets::new(gpui_component_assets::Assets::new(""));

        let loaded = assets
            .load(path.as_ref())
            .expect("load should succeed")
            .expect("inline svg should exist");
        assert_eq!(loaded.as_ref(), svg.as_bytes());
    }
}