use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use gpui::RenderImage;
use image::Frame;
use smallvec::SmallVec;

use crate::common::utils::logger;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetCacheStats {
    pub count: usize,
    pub total_bytes: usize,
    pub max_bytes: usize,
}

#[derive(Debug)]
struct CacheEntry {
    image: Arc<RenderImage>,
    decoded_bytes: usize,
    last_used: u64,
}

#[derive(Debug)]
pub struct AssetStore {
    entries: HashMap<String, CacheEntry>,
    total_bytes: usize,
    max_bytes: usize,
    touch_generation: u64,
}

impl Default for AssetStore {
    fn default() -> Self {
        Self::new(default_max_bytes())
    }
}

impl AssetStore {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            total_bytes: 0,
            max_bytes,
            touch_generation: 0,
        }
    }

    pub fn load_image(&mut self, uri: impl Into<String>, bytes: &[u8]) -> anyhow::Result<()> {
        let uri = uri.into();
        if self.entries.contains_key(&uri) {
            self.touch(&uri);
            return Ok(());
        }

        let (image, decoded_bytes) = decode_image_bytes(bytes)?;
        self.evict_lru_until_fits(decoded_bytes);
        self.touch_generation = self.touch_generation.saturating_add(1);
        self.total_bytes = self.total_bytes.saturating_add(decoded_bytes);
        self.entries.insert(
            uri,
            CacheEntry {
                image: Arc::new(image),
                decoded_bytes,
                last_used: self.touch_generation,
            },
        );
        Ok(())
    }

    pub fn remove(&mut self, uri: &str) -> bool {
        self.remove_entry(uri).is_some()
    }

    pub fn image(&mut self, uri: &str) -> Option<Arc<RenderImage>> {
        if !self.entries.contains_key(uri) {
            return None;
        }
        self.touch(uri);
        self.entries.get(uri).map(|entry| entry.image.clone())
    }

    pub fn stats(&self) -> AssetCacheStats {
        AssetCacheStats {
            count: self.entries.len(),
            total_bytes: self.total_bytes,
            max_bytes: self.max_bytes,
        }
    }

    fn touch(&mut self, uri: &str) {
        if !self.entries.contains_key(uri) {
            return;
        }
        self.touch_generation = self.touch_generation.saturating_add(1);
        if let Some(entry) = self.entries.get_mut(uri) {
            entry.last_used = self.touch_generation;
        }
    }

    fn evict_lru_until_fits(&mut self, incoming_bytes: usize) {
        while self.total_bytes.saturating_add(incoming_bytes) > self.max_bytes {
            let Some(uri) = self.lru_uri() else {
                break;
            };
            let freed = self.remove_entry(&uri).unwrap_or(0);
            logger::info(format!(
                "host.assets evicted uri={uri} freed={freed} total={}",
                self.total_bytes
            ));
        }
    }

    fn lru_uri(&self) -> Option<String> {
        self.entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(uri, _)| uri.clone())
    }

    fn remove_entry(&mut self, uri: &str) -> Option<usize> {
        let entry = self.entries.remove(uri)?;
        self.total_bytes = self.total_bytes.saturating_sub(entry.decoded_bytes);
        Some(entry.decoded_bytes)
    }
}

pub type SharedAssetStore = Arc<Mutex<AssetStore>>;

pub fn new_asset_store() -> SharedAssetStore {
    Arc::new(Mutex::new(AssetStore::default()))
}

fn default_max_bytes() -> usize {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        32 * 1024 * 1024
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        64 * 1024 * 1024
    }
}

fn decode_image_bytes(bytes: &[u8]) -> anyhow::Result<(RenderImage, usize)> {
    let format = image::guess_format(bytes)?;
    let mut rgba = image::load_from_memory_with_format(bytes, format)?.into_rgba8();
    let decoded_bytes = rgba.width() as usize * rgba.height() as usize * 4;
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Ok((
        RenderImage::new(SmallVec::from_elem(Frame::new(rgba), 1)),
        decoded_bytes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba([8, 16, 32, 255]));
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .expect("encode png");
        bytes
    }

    fn decoded_size(width: u32, height: u32) -> usize {
        width as usize * height as usize * 4
    }

    #[test]
    fn keeps_entries_under_budget() {
        let mut store = AssetStore::new(decoded_size(10, 10) * 2);
        store
            .load_image("a", &png_bytes(10, 10))
            .expect("load a");
        store
            .load_image("b", &png_bytes(10, 10))
            .expect("load b");
        assert_eq!(store.stats().count, 2);
        assert!(store.image("a").is_some());
        assert!(store.image("b").is_some());
    }

    #[test]
    fn evicts_least_recently_used_entry() {
        let entry_bytes = decoded_size(10, 10);
        let mut store = AssetStore::new(entry_bytes * 2);
        store
            .load_image("old", &png_bytes(10, 10))
            .expect("load old");
        store
            .load_image("keep", &png_bytes(10, 10))
            .expect("load keep");
        store.image("keep");
        store
            .load_image("new", &png_bytes(10, 10))
            .expect("load new");

        assert!(store.image("old").is_none());
        assert!(store.image("keep").is_some());
        assert!(store.image("new").is_some());
    }

    #[test]
    fn reload_same_uri_does_not_grow_cache() {
        let mut store = AssetStore::new(decoded_size(10, 10) * 4);
        let bytes = png_bytes(10, 10);
        store.load_image("same", &bytes).expect("first load");
        let stats = store.stats();
        store.load_image("same", &bytes).expect("second load");
        assert_eq!(store.stats().count, stats.count);
        assert_eq!(store.stats().total_bytes, stats.total_bytes);
    }

    #[test]
    fn remove_drops_entry() {
        let mut store = AssetStore::new(decoded_size(10, 10) * 2);
        store
            .load_image("gone", &png_bytes(10, 10))
            .expect("load");
        assert!(store.remove("gone"));
        assert!(store.image("gone").is_none());
        assert_eq!(store.stats().count, 0);
    }
}