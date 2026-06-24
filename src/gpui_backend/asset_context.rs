use std::cell::RefCell;
use std::sync::Arc;

use gpui::RenderImage;

use crate::bridge::SharedAssetStore;

thread_local! {
    static RENDER_ASSETS: RefCell<Option<SharedAssetStore>> = const { RefCell::new(None) };
}

pub fn with_render_assets<T>(assets: SharedAssetStore, render: impl FnOnce() -> T) -> T {
    RENDER_ASSETS.with(|slot| {
        *slot.borrow_mut() = Some(assets);
        let result = render();
        *slot.borrow_mut() = None;
        result
    })
}

pub fn current_render_image(id: &str) -> Option<Arc<RenderImage>> {
    RENDER_ASSETS.with(|slot| {
        let assets = slot.borrow().clone()?;
        let mut store = assets.lock().ok()?;
        store.image(id)
    })
}