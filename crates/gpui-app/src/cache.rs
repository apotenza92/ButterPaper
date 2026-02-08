//! Quality-aware, byte-bounded caches for rendered PDF surfaces.

#![allow(dead_code)]

use butterpaper_render::RenderQuality;
use image::{ImageBuffer, Rgba};
use serde::Serialize;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::sync::Arc;

const MB: u64 = 1024 * 1024;
const DEFAULT_TOTAL_RAM_GB_HINT: u64 = 16;
const MIN_ADAPTIVE_BUDGET_BYTES: u64 = 256 * MB;
const MAX_ADAPTIVE_BUDGET_BYTES: u64 = 4 * 1024 * MB;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MemoryPressureState {
    Normal,
    Warm,
    Hot,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdaptiveMemoryBudget {
    pub total_budget_bytes: u64,
    pub viewport_budget_bytes: u64,
    pub thumbnail_budget_bytes: u64,
    pub inflight_budget_bytes: u64,
}

impl AdaptiveMemoryBudget {
    fn tier_ratio(total_ram_bytes: u64) -> f64 {
        let gb = (total_ram_bytes as f64) / (1024.0 * 1024.0 * 1024.0);
        if gb <= 8.0 {
            0.12
        } else if gb <= 16.0 {
            0.14
        } else if gb <= 32.0 {
            0.16
        } else {
            0.18
        }
    }

    pub fn from_total_ram_bytes(total_ram_bytes: u64) -> Self {
        let ratio = Self::tier_ratio(total_ram_bytes);
        let proposed = (total_ram_bytes as f64 * ratio) as u64;
        let upper_from_ram = ((total_ram_bytes as f64) * 0.33) as u64;
        let upper_bound = upper_from_ram.min(MAX_ADAPTIVE_BUDGET_BYTES);
        let total_budget_bytes = proposed.clamp(MIN_ADAPTIVE_BUDGET_BYTES, upper_bound.max(1));

        let viewport_budget_bytes = (total_budget_bytes * 70) / 100;
        let thumbnail_budget_bytes = (total_budget_bytes * 20) / 100;
        let inflight_budget_bytes = total_budget_bytes
            .saturating_sub(viewport_budget_bytes)
            .saturating_sub(thumbnail_budget_bytes);

        Self {
            total_budget_bytes,
            viewport_budget_bytes,
            thumbnail_budget_bytes,
            inflight_budget_bytes,
        }
    }

    pub fn detect() -> Self {
        if let Ok(value) = std::env::var("BUTTERPAPER_TOTAL_RAM_GB") {
            if let Ok(gb) = value.parse::<u64>() {
                return Self::from_total_ram_bytes(gb.saturating_mul(1024 * 1024 * 1024));
            }
        }

        let guessed_bytes = system_total_ram_bytes()
            .unwrap_or(DEFAULT_TOTAL_RAM_GB_HINT.saturating_mul(1024 * 1024 * 1024));
        Self::from_total_ram_bytes(guessed_bytes)
    }
}

#[cfg(target_os = "macos")]
fn system_total_ram_bytes() -> Option<u64> {
    use std::ffi::CString;
    use std::mem::size_of;
    use std::ptr;

    let key = CString::new("hw.memsize").ok()?;
    let mut value: u64 = 0;
    let mut len = size_of::<u64>();
    let result = unsafe {
        libc::sysctlbyname(
            key.as_ptr(),
            &mut value as *mut u64 as *mut libc::c_void,
            &mut len,
            ptr::null_mut(),
            0,
        )
    };
    if result == 0 && len == size_of::<u64>() {
        Some(value)
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn system_total_ram_bytes() -> Option<u64> {
    let mut info = std::mem::MaybeUninit::<libc::sysinfo>::uninit();
    let rc = unsafe { libc::sysinfo(info.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }
    let info = unsafe { info.assume_init() };
    Some((info.totalram as u64).saturating_mul(info.mem_unit as u64))
}

#[cfg(target_os = "windows")]
fn system_total_ram_bytes() -> Option<u64> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn system_total_ram_bytes() -> Option<u64> {
    None
}

/// Cache key for rendered surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderCacheKey {
    pub doc_fingerprint: u64,
    pub page_index: u16,
    pub zoom_bucket: u32,
    pub rotation: u16,
    pub quality: RenderQuality,
    pub dpr_bucket: u16,
}

impl RenderCacheKey {
    pub fn new(
        doc_fingerprint: u64,
        page_index: u16,
        zoom_bucket: u32,
        rotation: u16,
        quality: RenderQuality,
        dpr_bucket: u16,
    ) -> Self {
        Self { doc_fingerprint, page_index, zoom_bucket, rotation, quality, dpr_bucket }
    }
}

/// Adaptive cache budget split between viewport and thumbnail surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheBudget {
    pub max_bytes: u64,
    pub viewport_bytes: u64,
    pub thumbnail_bytes: u64,
}

impl CacheBudget {
    pub fn adaptive() -> Self {
        let adaptive = AdaptiveMemoryBudget::detect();
        Self {
            max_bytes: adaptive.total_budget_bytes,
            viewport_bytes: adaptive.viewport_budget_bytes,
            thumbnail_bytes: adaptive.thumbnail_budget_bytes,
        }
    }
}

/// A cached render surface and its accounting data.
#[derive(Clone)]
pub struct CachedImage {
    pub image: Arc<gpui::RenderImage>,
    pub display_width: u32,
    pub display_height: u32,
    pub decoded_bytes: u64,
    pub texture_bytes: u64,
}

impl CachedImage {
    pub fn from_image(
        image: Arc<gpui::RenderImage>,
        pixel_width: u32,
        pixel_height: u32,
        display_width: u32,
        display_height: u32,
    ) -> Self {
        let bytes = pixel_width as u64 * pixel_height as u64 * 4;
        Self { image, display_width, display_height, decoded_bytes: bytes, texture_bytes: bytes }
    }
}

struct CacheEntry {
    value: CachedImage,
    last_access: u64,
}

/// Byte-bounded LRU cache with optional quality-biased eviction.
pub struct ByteLruCache {
    entries: HashMap<RenderCacheKey, CacheEntry>,
    access_counter: u64,
    max_bytes: u64,
    current_decoded_bytes: u64,
    current_texture_bytes: u64,
}

impl ByteLruCache {
    pub fn new(max_bytes: u64) -> Self {
        Self {
            entries: HashMap::new(),
            access_counter: 0,
            max_bytes,
            current_decoded_bytes: 0,
            current_texture_bytes: 0,
        }
    }

    pub fn max_bytes(&self) -> u64 {
        self.max_bytes
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn decoded_bytes(&self) -> u64 {
        self.current_decoded_bytes
    }

    pub fn texture_bytes(&self) -> u64 {
        self.current_texture_bytes
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.access_counter = 0;
        self.current_decoded_bytes = 0;
        self.current_texture_bytes = 0;
    }

    pub fn contains(&self, key: &RenderCacheKey) -> bool {
        self.entries.contains_key(key)
    }

    pub fn keys(&self) -> Vec<RenderCacheKey> {
        self.entries.keys().copied().collect()
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&RenderCacheKey, &CachedImage) -> bool) {
        self.entries.retain(|key, entry| keep(key, &entry.value));
        self.current_decoded_bytes = self
            .entries
            .values()
            .map(|entry| entry.value.decoded_bytes)
            .fold(0_u64, |acc, bytes| acc.saturating_add(bytes));
        self.current_texture_bytes = self
            .entries
            .values()
            .map(|entry| entry.value.texture_bytes)
            .fold(0_u64, |acc, bytes| acc.saturating_add(bytes));
    }

    /// Evict the least-recently used entry matching a predicate.
    pub fn evict_one_where(
        &mut self,
        mut predicate: impl FnMut(&RenderCacheKey, &CachedImage) -> bool,
    ) -> bool {
        let key_to_remove = self
            .entries
            .iter()
            .filter(|(key, entry)| predicate(key, &entry.value))
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(key, _)| *key);

        let Some(key_to_remove) = key_to_remove else {
            return false;
        };

        if let Some(removed) = self.entries.remove(&key_to_remove) {
            self.current_decoded_bytes =
                self.current_decoded_bytes.saturating_sub(removed.value.decoded_bytes);
            self.current_texture_bytes =
                self.current_texture_bytes.saturating_sub(removed.value.texture_bytes);
            return true;
        }

        false
    }

    pub fn get(&mut self, key: &RenderCacheKey) -> Option<CachedImage> {
        let entry = self.entries.get_mut(key)?;
        self.access_counter = self.access_counter.wrapping_add(1);
        entry.last_access = self.access_counter;
        Some(entry.value.clone())
    }

    /// Insert an entry. Returns `true` when inserted, `false` if it cannot fit.
    pub fn insert(
        &mut self,
        key: RenderCacheKey,
        value: CachedImage,
        preferred_evict_quality: Option<RenderQuality>,
    ) -> bool {
        if let Some(previous) = self.entries.remove(&key) {
            self.current_decoded_bytes =
                self.current_decoded_bytes.saturating_sub(previous.value.decoded_bytes);
            self.current_texture_bytes =
                self.current_texture_bytes.saturating_sub(previous.value.texture_bytes);
        }

        let incoming = value.decoded_bytes;
        if incoming > self.max_bytes {
            // Single entry cannot fit even after full eviction.
            self.clear();
            return false;
        }

        while self.current_decoded_bytes + incoming > self.max_bytes {
            if !self.evict_one(preferred_evict_quality) {
                break;
            }
        }

        if self.current_decoded_bytes + incoming > self.max_bytes {
            return false;
        }

        self.access_counter = self.access_counter.wrapping_add(1);
        self.current_decoded_bytes += value.decoded_bytes;
        self.current_texture_bytes += value.texture_bytes;
        self.entries.insert(key, CacheEntry { value, last_access: self.access_counter });
        true
    }

    fn evict_one(&mut self, preferred_evict_quality: Option<RenderQuality>) -> bool {
        let preferred_key = preferred_evict_quality.and_then(|quality| {
            self.entries
                .iter()
                .filter(|(key, _)| key.quality == quality)
                .min_by_key(|(_, entry)| entry.last_access)
                .map(|(key, _)| *key)
        });

        let fallback_key =
            self.entries.iter().min_by_key(|(_, entry)| entry.last_access).map(|(key, _)| *key);

        let key_to_remove = preferred_key.or(fallback_key);
        let Some(key_to_remove) = key_to_remove else {
            return false;
        };

        if let Some(removed) = self.entries.remove(&key_to_remove) {
            self.current_decoded_bytes =
                self.current_decoded_bytes.saturating_sub(removed.value.decoded_bytes);
            self.current_texture_bytes =
                self.current_texture_bytes.saturating_sub(removed.value.texture_bytes);
            return true;
        }

        false
    }
}

impl Default for ByteLruCache {
    fn default() -> Self {
        Self::new((256 * MB * 70) / 100)
    }
}

/// Convert RGBA pixels to BGRA and create a RenderImage.
pub fn create_render_image(
    rgba_pixels: Vec<u8>,
    width: u32,
    height: u32,
) -> Option<Arc<gpui::RenderImage>> {
    let mut bgra_pixels = rgba_pixels;
    for pixel in bgra_pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }

    let buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(width, height, bgra_pixels)?;
    let frame = image::Frame::new(buffer);
    Some(Arc::new(gpui::RenderImage::new(SmallVec::from_elem(frame, 1))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_image(width: u32, height: u32) -> Arc<gpui::RenderImage> {
        let rgba = vec![255u8; (width * height * 4) as usize];
        create_render_image(rgba, width, height).expect("dummy image should be creatable")
    }

    #[test]
    fn cache_key_distinguishes_quality_and_dpr() {
        let base = RenderCacheKey::new(1, 0, 100, 0, RenderQuality::LqScroll, 100);
        let different_quality = RenderCacheKey::new(1, 0, 100, 0, RenderQuality::HqFinal, 100);
        let different_dpr = RenderCacheKey::new(1, 0, 100, 0, RenderQuality::LqScroll, 200);

        assert_ne!(base, different_quality);
        assert_ne!(base, different_dpr);
    }

    #[test]
    fn lru_prefers_hq_eviction_when_requested() {
        let mut cache = ByteLruCache::new(32 * 1024);

        let hq_key = RenderCacheKey::new(1, 0, 100, 0, RenderQuality::HqFinal, 100);
        let lq_key = RenderCacheKey::new(1, 0, 100, 0, RenderQuality::LqScroll, 100);
        let new_lq_key = RenderCacheKey::new(1, 1, 100, 0, RenderQuality::LqScroll, 100);

        let cached = CachedImage::from_image(dummy_image(64, 64), 64, 64, 64, 64);
        assert!(cache.insert(hq_key, cached.clone(), Some(RenderQuality::HqFinal)));
        assert!(cache.insert(lq_key, cached.clone(), Some(RenderQuality::HqFinal)));

        // Inserting a third entry should evict HQ first due to the preference.
        assert!(cache.insert(new_lq_key, cached, Some(RenderQuality::HqFinal)));

        assert!(!cache.contains(&hq_key));
        assert!(cache.contains(&lq_key));
        assert!(cache.contains(&new_lq_key));
    }

    #[test]
    fn byte_budget_is_enforced_under_insert_churn() {
        let mut cache = ByteLruCache::new(48 * 1024);
        let image = CachedImage::from_image(dummy_image(64, 64), 64, 64, 64, 64);
        let per_entry_bytes = image.decoded_bytes;
        assert_eq!(per_entry_bytes, 16_384);

        for i in 0..64u16 {
            let quality = if i % 2 == 0 { RenderQuality::LqScroll } else { RenderQuality::HqFinal };
            let key = RenderCacheKey::new(99, i, 100, 0, quality, 100);
            let inserted = cache.insert(key, image.clone(), Some(RenderQuality::HqFinal));
            assert!(inserted, "entry should fit within budget");
            assert!(cache.decoded_bytes() <= cache.max_bytes());
            assert!(cache.texture_bytes() <= cache.max_bytes());
        }

        // 48KB budget with ~16KB entries should keep at most 3 entries resident.
        assert!(cache.len() <= 3);
    }

    #[test]
    fn adaptive_budget_tiers_match_expectations() {
        let b8 = AdaptiveMemoryBudget::from_total_ram_bytes(8 * 1024 * 1024 * 1024);
        let b16 = AdaptiveMemoryBudget::from_total_ram_bytes(16 * 1024 * 1024 * 1024);
        let b32 = AdaptiveMemoryBudget::from_total_ram_bytes(32 * 1024 * 1024 * 1024);
        let b64 = AdaptiveMemoryBudget::from_total_ram_bytes(64 * 1024 * 1024 * 1024);

        assert!(b8.total_budget_bytes >= 256 * MB);
        assert!(b16.total_budget_bytes >= b8.total_budget_bytes);
        assert!(b32.total_budget_bytes >= b16.total_budget_bytes);
        assert!(b64.total_budget_bytes >= b32.total_budget_bytes);
        assert!(b64.total_budget_bytes <= MAX_ADAPTIVE_BUDGET_BYTES);
        assert_eq!(
            b32.total_budget_bytes,
            b32.viewport_budget_bytes + b32.thumbnail_budget_bytes + b32.inflight_budget_bytes
        );
    }

    #[test]
    fn retain_recomputes_accounting() {
        let mut cache = ByteLruCache::new(256 * 1024);
        let k1 = RenderCacheKey::new(1, 0, 100, 0, RenderQuality::LqScroll, 100);
        let k2 = RenderCacheKey::new(1, 1, 100, 0, RenderQuality::HqFinal, 100);
        let v = CachedImage::from_image(dummy_image(64, 64), 64, 64, 64, 64);
        assert!(cache.insert(k1, v.clone(), None));
        assert!(cache.insert(k2, v.clone(), None));
        assert!(cache.decoded_bytes() >= v.decoded_bytes * 2);
        cache.retain(|key, _| key.page_index == 0);
        assert!(cache.contains(&k1));
        assert!(!cache.contains(&k2));
        assert_eq!(cache.decoded_bytes(), v.decoded_bytes);
        assert_eq!(cache.texture_bytes(), v.texture_bytes);
    }

    #[test]
    fn evict_one_where_removes_oldest_matching_entry() {
        let mut cache = ByteLruCache::new(256 * 1024);
        let v = CachedImage::from_image(dummy_image(64, 64), 64, 64, 64, 64);
        let hq1 = RenderCacheKey::new(1, 0, 100, 0, RenderQuality::HqFinal, 100);
        let lq = RenderCacheKey::new(1, 1, 100, 0, RenderQuality::LqScroll, 100);
        let hq2 = RenderCacheKey::new(1, 2, 100, 0, RenderQuality::HqFinal, 100);

        assert!(cache.insert(hq1, v.clone(), None));
        assert!(cache.insert(lq, v.clone(), None));
        assert!(cache.insert(hq2, v.clone(), None));

        let evicted = cache.evict_one_where(|key, _| matches!(key.quality, RenderQuality::HqFinal));
        assert!(evicted);
        assert!(!cache.contains(&hq1));
        assert!(cache.contains(&lq));
        assert!(cache.contains(&hq2));
    }
}
