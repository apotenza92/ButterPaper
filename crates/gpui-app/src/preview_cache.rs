//! Shared ultra-low-quality preview cache for no-blank rendering fallbacks.

use crate::cache::CachedImage;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

const MB: u64 = 1024 * 1024;
const PREVIEW_BUDGET_MIN_BYTES: u64 = 32 * MB;
const PREVIEW_BUDGET_MAX_BYTES: u64 = 192 * MB;
const PREVIEW_BUDGET_RATIO: f64 = 0.10;

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct SharedPreviewSnapshot {
    pub current_preview_decoded_bytes: u64,
    pub peak_preview_decoded_bytes: u64,
    pub preview_hit_count: u64,
    pub preview_miss_count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PreviewKey {
    doc_fingerprint: u64,
    page_index: u16,
}

#[derive(Clone)]
struct PreviewEntry {
    cached: CachedImage,
    last_access: u64,
}

pub struct SharedPreviewCache {
    entries: HashMap<PreviewKey, PreviewEntry>,
    access_counter: u64,
    max_bytes: u64,
    current_decoded_bytes: u64,
    peak_preview_decoded_bytes: u64,
    preview_hit_count: u64,
    preview_miss_count: u64,
}

impl SharedPreviewCache {
    pub fn preview_budget_bytes(total_budget_bytes: u64) -> u64 {
        let proportional = ((total_budget_bytes as f64) * PREVIEW_BUDGET_RATIO).round() as u64;
        proportional.clamp(PREVIEW_BUDGET_MIN_BYTES, PREVIEW_BUDGET_MAX_BYTES)
    }

    pub fn new(max_bytes: u64) -> Self {
        Self {
            entries: HashMap::new(),
            access_counter: 0,
            max_bytes: max_bytes.max(1),
            current_decoded_bytes: 0,
            peak_preview_decoded_bytes: 0,
            preview_hit_count: 0,
            preview_miss_count: 0,
        }
    }

    pub fn contains(&self, doc_fingerprint: u64, page_index: u16) -> bool {
        self.entries.contains_key(&PreviewKey { doc_fingerprint, page_index })
    }

    pub fn get(&mut self, doc_fingerprint: u64, page_index: u16) -> Option<Arc<gpui::RenderImage>> {
        let key = PreviewKey { doc_fingerprint, page_index };
        self.access_counter = self.access_counter.wrapping_add(1);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_access = self.access_counter;
            self.preview_hit_count = self.preview_hit_count.saturating_add(1);
            Some(entry.cached.image.clone())
        } else {
            self.preview_miss_count = self.preview_miss_count.saturating_add(1);
            None
        }
    }

    pub fn insert(&mut self, doc_fingerprint: u64, page_index: u16, cached: CachedImage) {
        let key = PreviewKey { doc_fingerprint, page_index };
        self.access_counter = self.access_counter.wrapping_add(1);

        if let Some(existing) = self.entries.remove(&key) {
            self.current_decoded_bytes =
                self.current_decoded_bytes.saturating_sub(existing.cached.decoded_bytes);
        }

        self.current_decoded_bytes = self.current_decoded_bytes.saturating_add(cached.decoded_bytes);
        self.peak_preview_decoded_bytes =
            self.peak_preview_decoded_bytes.max(self.current_decoded_bytes);
        self.entries.insert(key, PreviewEntry { cached, last_access: self.access_counter });

        self.evict_to_budget(&HashSet::new(), 0);
    }

    pub fn trim_to_budget(&mut self, keep_pages: &HashSet<u16>, doc_fingerprint: u64) {
        self.evict_to_budget(keep_pages, doc_fingerprint);
    }

    pub fn snapshot(&self) -> SharedPreviewSnapshot {
        SharedPreviewSnapshot {
            current_preview_decoded_bytes: self.current_decoded_bytes,
            peak_preview_decoded_bytes: self.peak_preview_decoded_bytes,
            preview_hit_count: self.preview_hit_count,
            preview_miss_count: self.preview_miss_count,
        }
    }

    fn evict_to_budget(&mut self, keep_pages: &HashSet<u16>, doc_fingerprint: u64) {
        while self.current_decoded_bytes > self.max_bytes {
            let mut lru_key: Option<PreviewKey> = None;
            let mut lru_access = u64::MAX;

            for (key, entry) in &self.entries {
                let protected = key.doc_fingerprint == doc_fingerprint
                    && keep_pages.contains(&key.page_index);
                if protected {
                    continue;
                }

                if entry.last_access < lru_access {
                    lru_access = entry.last_access;
                    lru_key = Some(*key);
                }
            }

            let Some(key) = lru_key else {
                break;
            };

            if let Some(removed) = self.entries.remove(&key) {
                self.current_decoded_bytes =
                    self.current_decoded_bytes.saturating_sub(removed.cached.decoded_bytes);
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::create_render_image;

    fn mock_cached(bytes_side: u32) -> CachedImage {
        let pixels = vec![255_u8; (bytes_side * bytes_side * 4) as usize];
        let image = create_render_image(pixels, bytes_side, bytes_side).expect("image");
        CachedImage::from_image(image, bytes_side, bytes_side, bytes_side, bytes_side)
    }

    #[test]
    fn preview_budget_has_floor_and_ceiling() {
        assert_eq!(SharedPreviewCache::preview_budget_bytes(64 * MB), 32 * MB);
        assert_eq!(SharedPreviewCache::preview_budget_bytes(8 * 1024 * MB), 192 * MB);
    }

    #[test]
    fn insert_get_and_evict_work() {
        let mut cache = SharedPreviewCache::new(10 * 1024);
        cache.insert(1, 0, mock_cached(32));
        cache.insert(1, 1, mock_cached(48));
        assert!(cache.get(1, 1).is_some());
        // 32x32 and 48x48 cannot both fit in 10KB decoded budget.
        assert!(cache.contains(1, 1));
        assert!(!cache.contains(1, 0));
    }

    #[test]
    fn trim_respects_hotset_when_possible() {
        let mut cache = SharedPreviewCache::new(48 * 48 * 4 + 8);
        cache.insert(7, 10, mock_cached(48));
        cache.insert(7, 11, mock_cached(48));
        let keep = HashSet::from([11_u16]);
        cache.trim_to_budget(&keep, 7);
        assert!(cache.contains(7, 11));
    }
}
