//! GPU texture cache with LRU eviction
//!
//! Provides GPU VRAM caching of rendered tiles as GPU textures with automatic eviction
//! based on Least Recently Used (LRU) policy when VRAM limits are reached.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// A cache key that uniquely identifies a tile
///
/// This is a simple u64 hash key. In practice, this would come from
/// TileId::cache_key() from the render crate.
pub type CacheKey = u64;

/// GPU texture handle
///
/// Stores a reference to a GPU texture with metadata for caching.
/// Uses trait object to support different GPU backends (Metal, DirectX, Vulkan).
pub struct GpuTexture {
    /// Cache key for this texture
    pub key: CacheKey,

    /// Opaque handle to the GPU texture (platform-specific)
    /// This is a Box<dyn Any> in practice, allowing storage of Metal::Texture, etc.
    texture_handle: Box<dyn std::any::Any + Send>,

    /// Width of the texture in pixels
    pub width: u32,

    /// Height of the texture in pixels
    pub height: u32,

    /// Estimated VRAM usage in bytes
    vram_size: usize,
}

impl GpuTexture {
    /// Create a new GPU texture
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key for this texture
    /// * `texture_handle` - Platform-specific GPU texture handle
    /// * `width` - Width in pixels
    /// * `height` - Height in pixels
    /// * `vram_size` - Estimated VRAM usage in bytes
    pub fn new<T: 'static + Send>(
        key: CacheKey,
        texture_handle: T,
        width: u32,
        height: u32,
        vram_size: usize,
    ) -> Self {
        Self {
            key,
            texture_handle: Box::new(texture_handle),
            width,
            height,
            vram_size,
        }
    }

    /// Get the estimated VRAM size of this texture in bytes
    pub fn vram_size(&self) -> usize {
        self.vram_size
    }

    /// Get a reference to the underlying texture handle
    ///
    /// Returns `None` if the type doesn't match.
    pub fn texture_handle<T: 'static>(&self) -> Option<&T> {
        self.texture_handle.downcast_ref::<T>()
    }
}

/// Statistics about GPU texture cache usage
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuCacheStats {
    /// Number of textures currently in cache
    pub texture_count: usize,

    /// Total VRAM used by cached textures (bytes)
    pub vram_used: usize,

    /// Maximum VRAM allowed (bytes)
    pub vram_limit: usize,

    /// Number of cache hits
    pub hits: u64,

    /// Number of cache misses
    pub misses: u64,

    /// Number of textures evicted due to VRAM pressure
    pub evictions: u64,
}

impl GpuCacheStats {
    /// Calculate the cache hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Calculate VRAM utilization (0.0 to 1.0)
    pub fn vram_utilization(&self) -> f64 {
        if self.vram_limit == 0 {
            0.0
        } else {
            self.vram_used as f64 / self.vram_limit as f64
        }
    }
}

/// Internal cache state
struct CacheState {
    /// Map from cache key to GPU texture
    textures: HashMap<CacheKey, GpuTexture>,

    /// LRU queue (most recently used at back, least recently used at front)
    lru_queue: VecDeque<CacheKey>,

    /// Current VRAM usage in bytes
    vram_used: usize,

    /// Maximum VRAM allowed in bytes
    vram_limit: usize,

    /// Statistics
    stats: GpuCacheStats,
}

impl CacheState {
    fn new(vram_limit: usize) -> Self {
        Self {
            textures: HashMap::new(),
            lru_queue: VecDeque::new(),
            vram_used: 0,
            vram_limit,
            stats: GpuCacheStats {
                vram_limit,
                ..Default::default()
            },
        }
    }

    /// Move a key to the back of the LRU queue (mark as most recently used)
    fn touch(&mut self, key: CacheKey) {
        // Remove the key from wherever it is in the queue
        self.lru_queue.retain(|&k| k != key);
        // Add it to the back (most recently used)
        self.lru_queue.push_back(key);
    }

    /// Evict the least recently used texture
    fn evict_lru(&mut self) -> Option<GpuTexture> {
        if let Some(key) = self.lru_queue.pop_front() {
            if let Some(texture) = self.textures.remove(&key) {
                self.vram_used = self.vram_used.saturating_sub(texture.vram_size());
                self.stats.texture_count = self.textures.len();
                self.stats.vram_used = self.vram_used;
                self.stats.evictions += 1;
                return Some(texture);
            }
        }
        None
    }

    /// Evict textures until VRAM usage is below the limit
    fn evict_to_fit(&mut self, required_size: usize) {
        while self.vram_used + required_size > self.vram_limit && !self.textures.is_empty() {
            if self.evict_lru().is_none() {
                break;
            }
        }
    }
}

/// GPU texture cache with LRU eviction
///
/// Thread-safe VRAM cache for GPU textures. When the cache reaches
/// its VRAM limit, the least recently used textures are evicted automatically.
///
/// # Example
///
/// ```no_run
/// use pdf_editor_cache::GpuTextureCache;
///
/// // Create a cache with 512MB VRAM limit
/// let cache = GpuTextureCache::new(512 * 1024 * 1024);
///
/// // Store a texture (platform-specific handle)
/// // let texture_handle = create_metal_texture(...);
/// // cache.put(12345, texture_handle, 256, 256, 256 * 256 * 4);
///
/// // Retrieve a texture
/// if let Some(texture) = cache.get(12345) {
///     let metadata = texture.metadata();
///     println!("Cache hit! Texture size: {}x{}", metadata.width, metadata.height);
/// }
///
/// // Check cache statistics
/// let stats = cache.stats();
/// println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
/// println!("VRAM used: {} / {} bytes", stats.vram_used, stats.vram_limit);
/// ```
pub struct GpuTextureCache {
    state: Arc<Mutex<CacheState>>,
}

impl GpuTextureCache {
    /// Create a new GPU texture cache with the specified VRAM limit
    ///
    /// # Arguments
    ///
    /// * `vram_limit` - Maximum VRAM in bytes that can be used by the cache
    pub fn new(vram_limit: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(CacheState::new(vram_limit))),
        }
    }

    /// Create a new GPU texture cache with a VRAM limit in megabytes
    ///
    /// # Arguments
    ///
    /// * `megabytes` - Maximum VRAM in megabytes
    pub fn with_mb_limit(megabytes: usize) -> Self {
        Self::new(megabytes * 1024 * 1024)
    }

    /// Store a texture in the cache
    ///
    /// If storing this texture would exceed the VRAM limit, least recently used
    /// textures will be evicted until there is enough space.
    ///
    /// # Arguments
    ///
    /// * `key` - Unique cache key for the texture
    /// * `texture_handle` - Platform-specific GPU texture handle
    /// * `width` - Width of the texture in pixels
    /// * `height` - Height of the texture in pixels
    /// * `vram_size` - Estimated VRAM usage in bytes
    pub fn put<T: 'static + Send>(
        &self,
        key: CacheKey,
        texture_handle: T,
        width: u32,
        height: u32,
        vram_size: usize,
    ) {
        let mut state = self.state.lock().unwrap();

        let texture = GpuTexture::new(key, texture_handle, width, height, vram_size);
        let texture_vram_size = texture.vram_size();

        // If this exact texture is already cached, remove it first
        if let Some(old_texture) = state.textures.remove(&key) {
            state.vram_used = state.vram_used.saturating_sub(old_texture.vram_size());
            state.lru_queue.retain(|&k| k != key);
        }

        // Evict textures if necessary to make room
        state.evict_to_fit(texture_vram_size);

        // Add the new texture
        state.vram_used += texture_vram_size;
        state.textures.insert(key, texture);
        state.touch(key);

        // Update stats
        state.stats.texture_count = state.textures.len();
        state.stats.vram_used = state.vram_used;
    }

    /// Retrieve a texture from the cache
    ///
    /// Returns a reference to the texture if found, or `None` if not in cache.
    /// Updates LRU tracking and statistics.
    ///
    /// This is a blocking operation that will wait if the cache is currently locked.
    /// For non-blocking access, use `try_get()`.
    ///
    /// Note: This returns a reference held within a MutexGuard. For long-lived
    /// access, consider extracting the needed data immediately.
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key for the texture to retrieve
    pub fn get(&self, key: CacheKey) -> Option<TextureRef<'_>> {
        let mut state = self.state.lock().unwrap();

        if state.textures.contains_key(&key) {
            // Cache hit - update LRU and stats
            state.touch(key);
            state.stats.hits += 1;

            // Return a reference wrapper that holds the lock
            Some(TextureRef { _guard: state, key })
        } else {
            // Cache miss
            state.stats.misses += 1;
            None
        }
    }

    /// Try to retrieve a texture from the cache without blocking
    ///
    /// Returns a reference to the texture if found and the lock was acquired,
    /// or `None` if the cache is locked or the texture was not found.
    ///
    /// This is a non-blocking operation that returns immediately if the cache is locked.
    /// Updates LRU tracking and statistics on success.
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key for the texture to retrieve
    ///
    /// # Returns
    ///
    /// - `Some(TextureRef)` - Cache hit, texture retrieved successfully
    /// - `None` - Either the cache is busy or the texture was not found
    ///
    /// Note: To distinguish between "cache busy" and "texture not found",
    /// you can first use `contains()` which doesn't update LRU.
    pub fn try_get(&self, key: CacheKey) -> Option<TextureRef<'_>> {
        let mut state = self.state.try_lock().ok()?;

        if state.textures.contains_key(&key) {
            // Cache hit - update LRU and stats
            state.touch(key);
            state.stats.hits += 1;

            // Return a reference wrapper that holds the lock
            Some(TextureRef { _guard: state, key })
        } else {
            // Cache miss
            state.stats.misses += 1;
            None
        }
    }

    /// Check if a texture is in the cache without updating LRU tracking
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key to check
    pub fn contains(&self, key: CacheKey) -> bool {
        let state = self.state.lock().unwrap();
        state.textures.contains_key(&key)
    }

    /// Remove a texture from the cache
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key for the texture to remove
    ///
    /// # Returns
    ///
    /// The removed texture, or `None` if it wasn't in the cache
    pub fn remove(&self, key: CacheKey) -> Option<GpuTexture> {
        let mut state = self.state.lock().unwrap();

        if let Some(texture) = state.textures.remove(&key) {
            state.vram_used = state.vram_used.saturating_sub(texture.vram_size());
            state.lru_queue.retain(|&k| k != key);
            state.stats.texture_count = state.textures.len();
            state.stats.vram_used = state.vram_used;
            Some(texture)
        } else {
            None
        }
    }

    /// Clear all textures from the cache
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        state.textures.clear();
        state.lru_queue.clear();
        state.vram_used = 0;
        state.stats.texture_count = 0;
        state.stats.vram_used = 0;
    }

    /// Get current cache statistics
    pub fn stats(&self) -> GpuCacheStats {
        let state = self.state.lock().unwrap();
        state.stats
    }

    /// Update the VRAM limit
    ///
    /// If the new limit is smaller than current usage, textures will be evicted
    /// until usage is below the new limit.
    ///
    /// # Arguments
    ///
    /// * `new_limit` - New VRAM limit in bytes
    pub fn set_vram_limit(&self, new_limit: usize) {
        let mut state = self.state.lock().unwrap();
        state.vram_limit = new_limit;
        state.stats.vram_limit = new_limit;

        // Evict textures if we're now over the limit
        if state.vram_used > new_limit {
            state.evict_to_fit(0);
        }
    }

    /// Get the current VRAM limit in bytes
    pub fn vram_limit(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.vram_limit
    }

    /// Get the current VRAM usage in bytes
    pub fn vram_used(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.vram_used
    }

    /// Get the number of textures currently in the cache
    pub fn texture_count(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.textures.len()
    }
}

impl Default for GpuTextureCache {
    /// Create a cache with a default 512MB VRAM limit
    fn default() -> Self {
        Self::with_mb_limit(512)
    }
}

/// Reference to a cached GPU texture
///
/// This holds the mutex guard to ensure thread-safe access to the texture.
/// The texture data can be accessed through methods on this type.
pub struct TextureRef<'a> {
    _guard: std::sync::MutexGuard<'a, CacheState>,
    key: CacheKey,
}

impl<'a> TextureRef<'a> {
    /// Get the texture metadata
    pub fn metadata(&self) -> TextureMetadata {
        let texture = self
            ._guard
            .textures
            .get(&self.key)
            .expect("Texture must exist");
        TextureMetadata {
            key: texture.key,
            width: texture.width,
            height: texture.height,
            vram_size: texture.vram_size,
        }
    }

    /// Get a reference to the underlying texture handle
    ///
    /// Returns `None` if the type doesn't match.
    pub fn texture_handle<T: 'static>(&self) -> Option<&T> {
        let texture = self
            ._guard
            .textures
            .get(&self.key)
            .expect("Texture must exist");
        texture.texture_handle()
    }
}

/// Texture metadata without holding the lock
#[derive(Debug, Clone, Copy)]
pub struct TextureMetadata {
    /// Cache key
    pub key: CacheKey,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// VRAM size in bytes
    pub vram_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock GPU texture handle for testing
    #[derive(Debug, Clone, PartialEq)]
    struct MockTexture {
        id: u32,
    }

    #[test]
    fn test_basic_put_get() {
        let cache = GpuTextureCache::new(1024 * 1024); // 1MB limit

        let mock_texture = MockTexture { id: 42 };
        let vram_size = 256 * 256 * 4; // 256KB texture
        cache.put(1, mock_texture.clone(), 256, 256, vram_size);

        let texture_ref = cache.get(1).expect("Texture should be in cache");
        let metadata = texture_ref.metadata();
        assert_eq!(metadata.key, 1);
        assert_eq!(metadata.width, 256);
        assert_eq!(metadata.height, 256);
        assert_eq!(metadata.vram_size, vram_size);

        // Verify we can downcast to the original type
        let handle = texture_ref
            .texture_handle::<MockTexture>()
            .expect("Should downcast");
        assert_eq!(handle.id, 42);
    }

    #[test]
    fn test_cache_miss() {
        let cache = GpuTextureCache::new(1024 * 1024);

        assert!(cache.get(999).is_none());

        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 0);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = GpuTextureCache::new(512 * 1024); // 512KB limit

        // Add three 256KB textures (total 768KB, exceeds limit)
        let vram_size = 256 * 256 * 4;
        cache.put(1, MockTexture { id: 1 }, 256, 256, vram_size);
        cache.put(2, MockTexture { id: 2 }, 256, 256, vram_size);
        cache.put(3, MockTexture { id: 3 }, 256, 256, vram_size); // Should evict texture 1

        // Texture 1 should be evicted (least recently used)
        assert!(cache.get(1).is_none());
        // Textures 2 and 3 should still be present
        assert!(cache.get(2).is_some());
        assert!(cache.get(3).is_some());

        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
    }

    #[test]
    fn test_lru_ordering() {
        let cache = GpuTextureCache::new(512 * 1024); // 512KB limit

        let vram_size = 256 * 256 * 4;
        cache.put(1, MockTexture { id: 1 }, 256, 256, vram_size);
        cache.put(2, MockTexture { id: 2 }, 256, 256, vram_size);

        // Access texture 1 to make it more recently used
        assert!(cache.get(1).is_some());

        // Add texture 3, should evict texture 2 (now least recently used)
        cache.put(3, MockTexture { id: 3 }, 256, 256, vram_size);

        assert!(cache.get(1).is_some()); // Still present
        assert!(cache.get(2).is_none()); // Evicted
        assert!(cache.get(3).is_some()); // Present
    }

    #[test]
    fn test_contains() {
        let cache = GpuTextureCache::new(1024 * 1024);

        cache.put(1, MockTexture { id: 1 }, 256, 256, 256 * 256 * 4);

        assert!(cache.contains(1));
        assert!(!cache.contains(999));
    }

    #[test]
    fn test_remove() {
        let cache = GpuTextureCache::new(1024 * 1024);

        cache.put(1, MockTexture { id: 1 }, 256, 256, 256 * 256 * 4);

        assert!(cache.contains(1));
        let removed = cache.remove(1);
        assert!(removed.is_some());
        assert!(!cache.contains(1));

        // Removing again should return None
        assert!(cache.remove(1).is_none());
    }

    #[test]
    fn test_clear() {
        let cache = GpuTextureCache::new(1024 * 1024);

        let vram_size = 256 * 256 * 4;
        cache.put(1, MockTexture { id: 1 }, 256, 256, vram_size);
        cache.put(2, MockTexture { id: 2 }, 256, 256, vram_size);
        cache.put(3, MockTexture { id: 3 }, 256, 256, vram_size);

        assert_eq!(cache.texture_count(), 3);

        cache.clear();

        assert_eq!(cache.texture_count(), 0);
        assert_eq!(cache.vram_used(), 0);
        assert!(!cache.contains(1));
        assert!(!cache.contains(2));
        assert!(!cache.contains(3));
    }

    #[test]
    fn test_stats() {
        let cache = GpuTextureCache::new(1024 * 1024);

        let vram_size = 256 * 256 * 4;
        cache.put(1, MockTexture { id: 1 }, 256, 256, vram_size);

        // One hit
        let _ = cache.get(1);
        // Two misses
        let _ = cache.get(2);
        let _ = cache.get(3);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 2);
        assert_eq!(stats.texture_count, 1);
        assert!(stats.vram_used > 0);

        // Hit rate should be 1/3
        let hit_rate = stats.hit_rate();
        assert!((hit_rate - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_vram_tracking() {
        let cache = GpuTextureCache::new(1024 * 1024);

        let vram_size = 256 * 256 * 4; // 256KB
        cache.put(1, MockTexture { id: 1 }, 256, 256, vram_size);

        assert_eq!(cache.vram_used(), vram_size);

        cache.put(2, MockTexture { id: 2 }, 256, 256, vram_size);
        assert_eq!(cache.vram_used(), vram_size * 2);

        cache.remove(1);
        assert_eq!(cache.vram_used(), vram_size);
    }

    #[test]
    fn test_set_vram_limit() {
        let cache = GpuTextureCache::new(1024 * 1024); // 1MB

        let vram_size = 256 * 256 * 4; // 256KB each
        cache.put(1, MockTexture { id: 1 }, 256, 256, vram_size);
        cache.put(2, MockTexture { id: 2 }, 256, 256, vram_size);
        cache.put(3, MockTexture { id: 3 }, 256, 256, vram_size);

        assert_eq!(cache.texture_count(), 3);

        // Reduce limit to 512KB (should evict one texture)
        cache.set_vram_limit(512 * 1024);

        assert_eq!(cache.texture_count(), 2);
        assert!(cache.vram_used() <= 512 * 1024);
    }

    #[test]
    fn test_update_existing_texture() {
        let cache = GpuTextureCache::new(1024 * 1024);

        let vram_size = 256 * 256 * 4;
        cache.put(1, MockTexture { id: 1 }, 256, 256, vram_size);
        cache.put(1, MockTexture { id: 2 }, 256, 256, vram_size); // Update same key

        // Should only have one texture
        assert_eq!(cache.texture_count(), 1);

        // Should have the new texture data
        let texture_ref = cache.get(1).unwrap();
        let handle = texture_ref.texture_handle::<MockTexture>().unwrap();
        assert_eq!(handle.id, 2);
    }

    #[test]
    fn test_default_cache() {
        let cache = GpuTextureCache::default();
        assert_eq!(cache.vram_limit(), 512 * 1024 * 1024); // 512MB
    }

    #[test]
    fn test_with_mb_limit() {
        let cache = GpuTextureCache::with_mb_limit(100);
        assert_eq!(cache.vram_limit(), 100 * 1024 * 1024);
    }

    #[test]
    fn test_vram_utilization() {
        let cache = GpuTextureCache::new(1024 * 1024); // 1MB

        let vram_size = 512 * 1024; // 512KB (50% of limit)
        cache.put(1, MockTexture { id: 1 }, 512, 512, vram_size);

        let stats = cache.stats();
        let utilization = stats.vram_utilization();
        assert!((utilization - 0.5).abs() < 0.01); // Should be ~50%
    }

    #[test]
    fn test_try_get_non_blocking() {
        let cache = GpuTextureCache::new(1024 * 1024);

        let vram_size = 256 * 256 * 4;
        cache.put(1, MockTexture { id: 42 }, 256, 256, vram_size);

        // try_get should succeed when cache is not locked
        if let Some(texture_ref) = cache.try_get(1) {
            let metadata = texture_ref.metadata();
            assert_eq!(metadata.key, 1);
            assert_eq!(metadata.width, 256);
            assert_eq!(metadata.height, 256);

            let handle = texture_ref.texture_handle::<MockTexture>().unwrap();
            assert_eq!(handle.id, 42);
        } else {
            panic!("Expected cache hit");
        }

        // try_get should return None when key doesn't exist
        assert!(cache.try_get(999).is_none());

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_try_get_lru_update() {
        let cache = GpuTextureCache::new(512 * 1024);

        let vram_size = 256 * 256 * 4;
        cache.put(1, MockTexture { id: 1 }, 256, 256, vram_size);
        cache.put(2, MockTexture { id: 2 }, 256, 256, vram_size);

        // Access texture 1 via try_get (should update LRU)
        assert!(cache.try_get(1).is_some());

        // Add texture 3, should evict texture 2 (now least recently used)
        cache.put(3, MockTexture { id: 3 }, 256, 256, vram_size);

        assert!(cache.contains(1)); // Still present (accessed via try_get)
        assert!(!cache.contains(2)); // Evicted
        assert!(cache.contains(3)); // Present
    }
}
