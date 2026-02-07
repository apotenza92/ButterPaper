//! GPU texture cache for rendered PDF pages
//!
//! Provides LRU-based caching of rendered page textures to avoid re-rendering
//! during scroll and zoom operations.

#![allow(dead_code)]

use image::{ImageBuffer, Rgba};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::sync::Arc;

/// Maximum number of cached pages (at full resolution)
const MAX_CACHE_ENTRIES: usize = 50;

/// Cache key: (page_index, zoom_level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub page_index: u16,
    pub zoom_level: u32,
}

impl CacheKey {
    pub fn new(page_index: u16, zoom_level: u32) -> Self {
        Self { page_index, zoom_level }
    }
}

/// Cached page entry with usage tracking
struct CacheEntry {
    /// The rendered image
    image: Arc<gpui::RenderImage>,
    /// Rendered dimensions
    width: u32,
    height: u32,
    /// Access counter for LRU eviction
    last_access: u64,
}

/// GPU texture cache for rendered PDF pages
pub struct PageCache {
    /// Cached pages indexed by (page, zoom)
    entries: HashMap<CacheKey, CacheEntry>,
    /// Global access counter
    access_counter: u64,
    /// Maximum entries
    max_entries: usize,
}

impl PageCache {
    pub fn new() -> Self {
        Self { entries: HashMap::new(), access_counter: 0, max_entries: MAX_CACHE_ENTRIES }
    }

    /// Get a cached page if available
    pub fn get(&mut self, key: CacheKey) -> Option<(Arc<gpui::RenderImage>, u32, u32)> {
        if let Some(entry) = self.entries.get_mut(&key) {
            self.access_counter += 1;
            entry.last_access = self.access_counter;
            Some((entry.image.clone(), entry.width, entry.height))
        } else {
            None
        }
    }

    /// Check if a page is cached without updating access time
    pub fn contains(&self, key: CacheKey) -> bool {
        self.entries.contains_key(&key)
    }

    /// Insert a rendered page into the cache
    pub fn insert(
        &mut self,
        key: CacheKey,
        image: Arc<gpui::RenderImage>,
        width: u32,
        height: u32,
    ) {
        // Evict if at capacity
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(&key) {
            self.evict_lru();
        }

        self.access_counter += 1;
        self.entries
            .insert(key, CacheEntry { image, width, height, last_access: self.access_counter });
    }

    /// Evict the least recently used entry
    fn evict_lru(&mut self) {
        if let Some((&lru_key, _)) = self.entries.iter().min_by_key(|(_, entry)| entry.last_access)
        {
            self.entries.remove(&lru_key);
        }
    }

    /// Clear all cached pages (e.g., when loading a new document)
    pub fn clear(&mut self) {
        self.entries.clear();
        self.access_counter = 0;
    }

    /// Clear pages for a specific zoom level (used when zoom changes)
    pub fn clear_zoom(&mut self, zoom_level: u32) {
        self.entries.retain(|key, _| key.zoom_level != zoom_level);
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.entries.len(), self.max_entries)
    }
}

impl Default for PageCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert RGBA pixels to BGRA and create a RenderImage
pub fn create_render_image(
    rgba_pixels: Vec<u8>,
    width: u32,
    height: u32,
) -> Option<Arc<gpui::RenderImage>> {
    // Convert RGBA to BGRA (GPUI requirement)
    let mut bgra_pixels = rgba_pixels;
    for pixel in bgra_pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }

    let buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(width, height, bgra_pixels)?;
    let frame = image::Frame::new(buffer);
    Some(Arc::new(gpui::RenderImage::new(SmallVec::from_elem(frame, 1))))
}
