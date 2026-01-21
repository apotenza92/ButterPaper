//! RAM tile cache with LRU eviction
//!
//! Provides in-memory caching of rendered tiles with automatic eviction
//! based on Least Recently Used (LRU) policy when memory limits are reached.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// A cache key that uniquely identifies a tile
///
/// This is a simple u64 hash key. In practice, this would come from
/// TileId::cache_key() from the render crate.
pub type CacheKey = u64;

/// Cached tile data
///
/// Stores the raw pixel data and metadata for a cached tile.
#[derive(Debug, Clone)]
pub struct CachedTile {
    /// Cache key for this tile
    pub key: CacheKey,

    /// Raw pixel data (RGBA format)
    pub pixels: Vec<u8>,

    /// Width of the tile in pixels
    pub width: u32,

    /// Height of the tile in pixels
    pub height: u32,
}

impl CachedTile {
    /// Create a new cached tile
    pub fn new(key: CacheKey, pixels: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            key,
            pixels,
            width,
            height,
        }
    }

    /// Get the memory size of this tile in bytes
    pub fn memory_size(&self) -> usize {
        self.pixels.len()
    }
}

/// Statistics about cache usage
#[derive(Debug, Clone, Copy, Default)]
pub struct CacheStats {
    /// Number of tiles currently in cache
    pub tile_count: usize,

    /// Total memory used by cached tiles (bytes)
    pub memory_used: usize,

    /// Maximum memory allowed (bytes)
    pub memory_limit: usize,

    /// Number of cache hits
    pub hits: u64,

    /// Number of cache misses
    pub misses: u64,

    /// Number of tiles evicted due to memory pressure
    pub evictions: u64,
}

impl CacheStats {
    /// Calculate the cache hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Calculate memory utilization (0.0 to 1.0)
    pub fn memory_utilization(&self) -> f64 {
        if self.memory_limit == 0 {
            0.0
        } else {
            self.memory_used as f64 / self.memory_limit as f64
        }
    }
}

/// Internal cache state
struct CacheState {
    /// Map from cache key to tile data
    tiles: HashMap<CacheKey, CachedTile>,

    /// LRU queue (most recently used at back, least recently used at front)
    lru_queue: VecDeque<CacheKey>,

    /// Current memory usage in bytes
    memory_used: usize,

    /// Maximum memory allowed in bytes
    memory_limit: usize,

    /// Statistics
    stats: CacheStats,
}

impl CacheState {
    fn new(memory_limit: usize) -> Self {
        Self {
            tiles: HashMap::new(),
            lru_queue: VecDeque::new(),
            memory_used: 0,
            memory_limit,
            stats: CacheStats {
                memory_limit,
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

    /// Evict the least recently used tile
    fn evict_lru(&mut self) -> Option<CachedTile> {
        if let Some(key) = self.lru_queue.pop_front() {
            if let Some(tile) = self.tiles.remove(&key) {
                self.memory_used = self.memory_used.saturating_sub(tile.memory_size());
                self.stats.tile_count = self.tiles.len();
                self.stats.memory_used = self.memory_used;
                self.stats.evictions += 1;
                return Some(tile);
            }
        }
        None
    }

    /// Evict tiles until memory usage is below the limit
    fn evict_to_fit(&mut self, required_size: usize) {
        while self.memory_used + required_size > self.memory_limit && !self.tiles.is_empty() {
            if self.evict_lru().is_none() {
                break;
            }
        }
    }
}

/// RAM tile cache with LRU eviction
///
/// Thread-safe in-memory cache for rendered tiles. When the cache reaches
/// its memory limit, the least recently used tiles are evicted automatically.
///
/// # Example
///
/// ```
/// use pdf_editor_cache::RamTileCache;
///
/// // Create a cache with 100MB limit
/// let cache = RamTileCache::new(100 * 1024 * 1024);
///
/// // Store a tile
/// let pixels = vec![0u8; 256 * 256 * 4]; // 256x256 RGBA
/// cache.put(12345, pixels, 256, 256);
///
/// // Retrieve a tile
/// if let Some(tile) = cache.get(12345) {
///     println!("Cache hit! Tile size: {}x{}", tile.width, tile.height);
/// }
///
/// // Check cache statistics
/// let stats = cache.stats();
/// println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
/// println!("Memory used: {} / {} bytes", stats.memory_used, stats.memory_limit);
/// ```
pub struct RamTileCache {
    state: Arc<Mutex<CacheState>>,
}

impl RamTileCache {
    /// Create a new RAM tile cache with the specified memory limit
    ///
    /// # Arguments
    ///
    /// * `memory_limit` - Maximum memory in bytes that can be used by the cache
    pub fn new(memory_limit: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(CacheState::new(memory_limit))),
        }
    }

    /// Create a new RAM tile cache with a memory limit in megabytes
    ///
    /// # Arguments
    ///
    /// * `megabytes` - Maximum memory in megabytes
    pub fn with_mb_limit(megabytes: usize) -> Self {
        Self::new(megabytes * 1024 * 1024)
    }

    /// Store a tile in the cache
    ///
    /// If storing this tile would exceed the memory limit, least recently used
    /// tiles will be evicted until there is enough space.
    ///
    /// # Arguments
    ///
    /// * `key` - Unique cache key for the tile
    /// * `pixels` - Raw pixel data (RGBA format)
    /// * `width` - Width of the tile in pixels
    /// * `height` - Height of the tile in pixels
    pub fn put(&self, key: CacheKey, pixels: Vec<u8>, width: u32, height: u32) {
        let mut state = self.state.lock().unwrap();

        let tile = CachedTile::new(key, pixels, width, height);
        let tile_size = tile.memory_size();

        // If this exact tile is already cached, remove it first
        if let Some(old_tile) = state.tiles.remove(&key) {
            state.memory_used = state.memory_used.saturating_sub(old_tile.memory_size());
            state.lru_queue.retain(|&k| k != key);
        }

        // Evict tiles if necessary to make room
        state.evict_to_fit(tile_size);

        // Add the new tile
        state.memory_used += tile_size;
        state.tiles.insert(key, tile);
        state.touch(key);

        // Update stats
        state.stats.tile_count = state.tiles.len();
        state.stats.memory_used = state.memory_used;
    }

    /// Retrieve a tile from the cache
    ///
    /// Returns `Some(tile)` if the tile is in the cache, or `None` if not found.
    /// Updates LRU tracking and statistics.
    ///
    /// This is a blocking operation that will wait if the cache is currently locked.
    /// For non-blocking access, use `try_get()`.
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key for the tile to retrieve
    pub fn get(&self, key: CacheKey) -> Option<CachedTile> {
        let mut state = self.state.lock().unwrap();

        if let Some(tile) = state.tiles.get(&key).cloned() {
            // Cache hit - update LRU and stats
            state.touch(key);
            state.stats.hits += 1;
            Some(tile)
        } else {
            // Cache miss
            state.stats.misses += 1;
            None
        }
    }

    /// Try to retrieve a tile from the cache without blocking
    ///
    /// Returns `Some(Some(tile))` if the tile is in the cache and the lock was acquired,
    /// `Some(None)` if the lock was acquired but the tile was not found,
    /// or `None` if the cache is currently locked and the operation would block.
    ///
    /// This is a non-blocking operation that returns immediately if the cache is locked.
    /// Updates LRU tracking and statistics on success.
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key for the tile to retrieve
    ///
    /// # Returns
    ///
    /// - `Some(Some(tile))` - Cache hit, tile retrieved successfully
    /// - `Some(None)` - Cache miss, no tile with this key
    /// - `None` - Could not acquire lock (cache is busy)
    pub fn try_get(&self, key: CacheKey) -> Option<Option<CachedTile>> {
        let mut state = self.state.try_lock().ok()?;

        if let Some(tile) = state.tiles.get(&key).cloned() {
            // Cache hit - update LRU and stats
            state.touch(key);
            state.stats.hits += 1;
            Some(Some(tile))
        } else {
            // Cache miss
            state.stats.misses += 1;
            Some(None)
        }
    }

    /// Check if a tile is in the cache without updating LRU tracking
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key to check
    pub fn contains(&self, key: CacheKey) -> bool {
        let state = self.state.lock().unwrap();
        state.tiles.contains_key(&key)
    }

    /// Remove a tile from the cache
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key for the tile to remove
    ///
    /// # Returns
    ///
    /// The removed tile, or `None` if it wasn't in the cache
    pub fn remove(&self, key: CacheKey) -> Option<CachedTile> {
        let mut state = self.state.lock().unwrap();

        if let Some(tile) = state.tiles.remove(&key) {
            state.memory_used = state.memory_used.saturating_sub(tile.memory_size());
            state.lru_queue.retain(|&k| k != key);
            state.stats.tile_count = state.tiles.len();
            state.stats.memory_used = state.memory_used;
            Some(tile)
        } else {
            None
        }
    }

    /// Clear all tiles from the cache
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        state.tiles.clear();
        state.lru_queue.clear();
        state.memory_used = 0;
        state.stats.tile_count = 0;
        state.stats.memory_used = 0;
    }

    /// Get current cache statistics
    pub fn stats(&self) -> CacheStats {
        let state = self.state.lock().unwrap();
        state.stats
    }

    /// Update the memory limit
    ///
    /// If the new limit is smaller than current usage, tiles will be evicted
    /// until usage is below the new limit.
    ///
    /// # Arguments
    ///
    /// * `new_limit` - New memory limit in bytes
    pub fn set_memory_limit(&self, new_limit: usize) {
        let mut state = self.state.lock().unwrap();
        state.memory_limit = new_limit;
        state.stats.memory_limit = new_limit;

        // Evict tiles if we're now over the limit
        if state.memory_used > new_limit {
            state.evict_to_fit(0);
        }
    }

    /// Get the current memory limit in bytes
    pub fn memory_limit(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.memory_limit
    }

    /// Get the current memory usage in bytes
    pub fn memory_used(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.memory_used
    }

    /// Get the number of tiles currently in the cache
    pub fn tile_count(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.tiles.len()
    }
}

impl Default for RamTileCache {
    /// Create a cache with a default 256MB limit
    fn default() -> Self {
        Self::with_mb_limit(256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_put_get() {
        let cache = RamTileCache::new(1024 * 1024); // 1MB limit

        let pixels = vec![0u8; 256 * 256 * 4]; // 256KB tile
        cache.put(1, pixels.clone(), 256, 256);

        let tile = cache.get(1).expect("Tile should be in cache");
        assert_eq!(tile.key, 1);
        assert_eq!(tile.pixels, pixels);
        assert_eq!(tile.width, 256);
        assert_eq!(tile.height, 256);
    }

    #[test]
    fn test_cache_miss() {
        let cache = RamTileCache::new(1024 * 1024);

        assert!(cache.get(999).is_none());

        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 0);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = RamTileCache::new(512 * 1024); // 512KB limit

        // Add three 256KB tiles (total 768KB, exceeds limit)
        let pixels = vec![0u8; 256 * 256 * 4];
        cache.put(1, pixels.clone(), 256, 256);
        cache.put(2, pixels.clone(), 256, 256);
        cache.put(3, pixels.clone(), 256, 256); // Should evict tile 1

        // Tile 1 should be evicted (least recently used)
        assert!(cache.get(1).is_none());
        // Tiles 2 and 3 should still be present
        assert!(cache.get(2).is_some());
        assert!(cache.get(3).is_some());

        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
    }

    #[test]
    fn test_lru_ordering() {
        let cache = RamTileCache::new(512 * 1024); // 512KB limit

        let pixels = vec![0u8; 256 * 256 * 4];
        cache.put(1, pixels.clone(), 256, 256);
        cache.put(2, pixels.clone(), 256, 256);

        // Access tile 1 to make it more recently used
        assert!(cache.get(1).is_some());

        // Add tile 3, should evict tile 2 (now least recently used)
        cache.put(3, pixels.clone(), 256, 256);

        assert!(cache.get(1).is_some()); // Still present
        assert!(cache.get(2).is_none()); // Evicted
        assert!(cache.get(3).is_some()); // Present
    }

    #[test]
    fn test_contains() {
        let cache = RamTileCache::new(1024 * 1024);

        let pixels = vec![0u8; 256 * 256 * 4];
        cache.put(1, pixels, 256, 256);

        assert!(cache.contains(1));
        assert!(!cache.contains(999));
    }

    #[test]
    fn test_remove() {
        let cache = RamTileCache::new(1024 * 1024);

        let pixels = vec![0u8; 256 * 256 * 4];
        cache.put(1, pixels, 256, 256);

        assert!(cache.contains(1));
        let removed = cache.remove(1);
        assert!(removed.is_some());
        assert!(!cache.contains(1));

        // Removing again should return None
        assert!(cache.remove(1).is_none());
    }

    #[test]
    fn test_clear() {
        let cache = RamTileCache::new(1024 * 1024);

        let pixels = vec![0u8; 256 * 256 * 4];
        cache.put(1, pixels.clone(), 256, 256);
        cache.put(2, pixels.clone(), 256, 256);
        cache.put(3, pixels, 256, 256);

        assert_eq!(cache.tile_count(), 3);

        cache.clear();

        assert_eq!(cache.tile_count(), 0);
        assert_eq!(cache.memory_used(), 0);
        assert!(!cache.contains(1));
        assert!(!cache.contains(2));
        assert!(!cache.contains(3));
    }

    #[test]
    fn test_stats() {
        let cache = RamTileCache::new(1024 * 1024);

        let pixels = vec![0u8; 256 * 256 * 4];
        cache.put(1, pixels, 256, 256);

        // One hit
        let _ = cache.get(1);
        // Two misses
        let _ = cache.get(2);
        let _ = cache.get(3);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 2);
        assert_eq!(stats.tile_count, 1);
        assert!(stats.memory_used > 0);

        // Hit rate should be 1/3
        let hit_rate = stats.hit_rate();
        assert!((hit_rate - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_memory_tracking() {
        let cache = RamTileCache::new(1024 * 1024);

        let pixels = vec![0u8; 256 * 256 * 4]; // 256KB
        cache.put(1, pixels.clone(), 256, 256);

        let expected_size = 256 * 256 * 4;
        assert_eq!(cache.memory_used(), expected_size);

        cache.put(2, pixels, 256, 256);
        assert_eq!(cache.memory_used(), expected_size * 2);

        cache.remove(1);
        assert_eq!(cache.memory_used(), expected_size);
    }

    #[test]
    fn test_set_memory_limit() {
        let cache = RamTileCache::new(1024 * 1024); // 1MB

        let pixels = vec![0u8; 256 * 256 * 4]; // 256KB each
        cache.put(1, pixels.clone(), 256, 256);
        cache.put(2, pixels.clone(), 256, 256);
        cache.put(3, pixels, 256, 256);

        assert_eq!(cache.tile_count(), 3);

        // Reduce limit to 512KB (should evict one tile)
        cache.set_memory_limit(512 * 1024);

        assert_eq!(cache.tile_count(), 2);
        assert!(cache.memory_used() <= 512 * 1024);
    }

    #[test]
    fn test_update_existing_tile() {
        let cache = RamTileCache::new(1024 * 1024);

        let pixels1 = vec![1u8; 256 * 256 * 4];
        cache.put(1, pixels1, 256, 256);

        let pixels2 = vec![2u8; 256 * 256 * 4];
        cache.put(1, pixels2.clone(), 256, 256); // Update same key

        // Should only have one tile
        assert_eq!(cache.tile_count(), 1);

        // Should have the new pixel data
        let tile = cache.get(1).unwrap();
        assert_eq!(tile.pixels, pixels2);
    }

    #[test]
    fn test_default_cache() {
        let cache = RamTileCache::default();
        assert_eq!(cache.memory_limit(), 256 * 1024 * 1024); // 256MB
    }

    #[test]
    fn test_with_mb_limit() {
        let cache = RamTileCache::with_mb_limit(100);
        assert_eq!(cache.memory_limit(), 100 * 1024 * 1024);
    }

    #[test]
    fn test_try_get_non_blocking() {
        let cache = RamTileCache::new(1024 * 1024);

        let pixels = vec![0u8; 256 * 256 * 4];
        cache.put(1, pixels.clone(), 256, 256);

        // try_get should succeed when cache is not locked
        match cache.try_get(1) {
            Some(Some(tile)) => {
                assert_eq!(tile.key, 1);
                assert_eq!(tile.pixels, pixels);
            }
            _ => panic!("Expected cache hit"),
        }

        // try_get should return None when key doesn't exist
        match cache.try_get(999) {
            Some(None) => {
                // Expected: cache miss
            }
            _ => panic!("Expected cache miss"),
        }

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_try_get_lru_update() {
        let cache = RamTileCache::new(512 * 1024);

        let pixels = vec![0u8; 256 * 256 * 4];
        cache.put(1, pixels.clone(), 256, 256);
        cache.put(2, pixels.clone(), 256, 256);

        // Access tile 1 via try_get (should update LRU)
        assert!(matches!(cache.try_get(1), Some(Some(_))));

        // Add tile 3, should evict tile 2 (now least recently used)
        cache.put(3, pixels.clone(), 256, 256);

        assert!(cache.contains(1)); // Still present (accessed via try_get)
        assert!(!cache.contains(2)); // Evicted
        assert!(cache.contains(3)); // Present
    }

    // ============================================================================
    // Large PDF Handling Tests (Phase 4.2)
    // ============================================================================

    #[test]
    fn test_memory_bounded_for_500_page_pdf_simulation() {
        // Simulate caching tiles for a 500+ page PDF
        // Cache limit: 50MB (reasonable for a PDF viewer)
        let cache_limit_mb = 50;
        let cache = RamTileCache::with_mb_limit(cache_limit_mb);

        let tile_size = 256 * 256 * 4; // 256KB per tile
        let page_count = 500;
        let tiles_per_page = 12;

        // Insert tiles for all pages (simulating navigation through entire document)
        for page_index in 0..page_count {
            for tile_index in 0..tiles_per_page {
                let key = (page_index * tiles_per_page + tile_index) as u64;
                let pixels = vec![0u8; tile_size];
                cache.put(key, pixels, 256, 256);
            }
        }

        // Memory should stay bounded
        let stats = cache.stats();
        assert!(
            stats.memory_used <= cache_limit_mb * 1024 * 1024,
            "Cache exceeded memory limit: {} > {}",
            stats.memory_used,
            cache_limit_mb * 1024 * 1024
        );

        // Should have evicted many tiles
        assert!(stats.evictions > 0, "No evictions occurred");

        // Cache should only hold ~200 tiles (50MB / 256KB ≈ 200)
        let expected_max_tiles = (cache_limit_mb * 1024 * 1024) / tile_size;
        assert!(
            stats.tile_count <= expected_max_tiles,
            "Too many tiles in cache: {} > {}",
            stats.tile_count,
            expected_max_tiles
        );
    }

    #[test]
    fn test_lru_eviction_preserves_recent_tiles_for_large_pdf() {
        // Cache can hold ~10 tiles
        let cache = RamTileCache::new(10 * 256 * 256 * 4);

        let tile_size = 256 * 256 * 4;
        let pixels = vec![0u8; tile_size];

        // Insert tiles 0-9 (fills cache)
        for i in 0..10 {
            cache.put(i, pixels.clone(), 256, 256);
        }

        // Access tiles 5-9 (make them most recently used)
        for i in 5..10 {
            cache.get(i);
        }

        // Insert 5 more tiles (should evict tiles 0-4)
        for i in 10..15 {
            cache.put(i, pixels.clone(), 256, 256);
        }

        // Tiles 0-4 should be evicted (least recently used)
        for i in 0..5 {
            assert!(
                !cache.contains(i),
                "Tile {} should have been evicted",
                i
            );
        }

        // Tiles 5-9 should still be present (recently accessed)
        for i in 5..10 {
            assert!(
                cache.contains(i),
                "Tile {} should still be in cache",
                i
            );
        }

        // Tiles 10-14 should be present (newly added)
        for i in 10..15 {
            assert!(
                cache.contains(i),
                "Tile {} should be in cache",
                i
            );
        }
    }

    #[test]
    fn test_cache_performance_under_high_load() {
        use std::time::Instant;

        let cache = RamTileCache::with_mb_limit(50);
        let tile_size = 256 * 256 * 4;
        let operation_count = 10000;

        let start = Instant::now();

        // Perform many put operations
        for i in 0..operation_count {
            let pixels = vec![0u8; tile_size];
            cache.put(i as u64, pixels, 256, 256);
        }

        let put_time = start.elapsed();

        // Perform many get operations (mix of hits and misses)
        let get_start = Instant::now();
        for i in 0..operation_count {
            let _ = cache.get(i as u64);
        }
        let get_time = get_start.elapsed();

        // Performance assertions
        // 10000 puts should complete in under 5 seconds
        assert!(
            put_time.as_secs() < 5,
            "Put operations too slow: {:?}",
            put_time
        );

        // 10000 gets should complete in under 1 second
        assert!(
            get_time.as_secs() < 1,
            "Get operations too slow: {:?}",
            get_time
        );

        let stats = cache.stats();
        // Should have high hit rate for recent tiles
        assert!(stats.hits > 0, "No cache hits recorded");
    }

    #[test]
    fn test_cache_hit_rate_with_viewport_simulation() {
        // Simulate user scrolling through a document with a viewport
        // The viewport shows ~6 tiles at a time
        let cache = RamTileCache::with_mb_limit(10); // Can hold ~40 tiles
        let tile_size = 256 * 256 * 4;
        let pixels = vec![0u8; tile_size];

        // Simulate scrolling through 100 pages, accessing 6 tiles per page
        // With cache holding ~40 tiles, we should see good hit rate for
        // adjacent page access patterns
        for page in 0..100 {
            // Pre-populate cache with current page tiles
            for tile in 0..6 {
                let key = (page * 6 + tile) as u64;
                cache.put(key, pixels.clone(), 256, 256);
            }

            // Simulate re-accessing current page tiles (common pattern)
            for tile in 0..6 {
                let key = (page * 6 + tile) as u64;
                let _ = cache.get(key);
            }
        }

        let stats = cache.stats();

        // Calculate hit rate
        let hit_rate = stats.hit_rate();
        // With good temporal locality, we should see reasonable hit rate
        assert!(
            hit_rate > 0.4,
            "Hit rate too low for sequential access: {:.2}%",
            hit_rate * 100.0
        );
    }

    #[test]
    fn test_memory_limit_reduction_evicts_correctly() {
        let cache = RamTileCache::with_mb_limit(100);
        let tile_size = 256 * 256 * 4;
        let pixels = vec![0u8; tile_size];

        // Fill with 100 tiles (~25MB)
        for i in 0..100 {
            cache.put(i, pixels.clone(), 256, 256);
        }

        assert_eq!(cache.tile_count(), 100);

        // Reduce limit to 5MB (can hold ~20 tiles)
        cache.set_memory_limit(5 * 1024 * 1024);

        // Should have evicted tiles to fit new limit
        assert!(
            cache.tile_count() <= 20,
            "Too many tiles after limit reduction: {}",
            cache.tile_count()
        );

        assert!(
            cache.memory_used() <= 5 * 1024 * 1024,
            "Memory exceeds new limit after reduction"
        );
    }

    #[test]
    fn test_concurrent_access_simulation() {
        use std::sync::Arc;
        use std::thread;

        let cache = Arc::new(RamTileCache::with_mb_limit(50));
        let tile_size = 256 * 256 * 4;

        // Spawn multiple threads that read and write to the cache
        let mut handles = vec![];

        for thread_id in 0..4 {
            let cache_clone = Arc::clone(&cache);
            let handle = thread::spawn(move || {
                let pixels = vec![0u8; tile_size];

                // Each thread works on its own key range
                let start_key = thread_id * 1000;
                let end_key = start_key + 500;

                // Write 500 tiles
                for i in start_key..end_key {
                    cache_clone.put(i as u64, pixels.clone(), 256, 256);
                }

                // Read 500 tiles (some will be evicted)
                let mut hits = 0;
                for i in start_key..end_key {
                    if cache_clone.get(i as u64).is_some() {
                        hits += 1;
                    }
                }

                hits
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        let total_hits: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();

        // Cache should still be consistent
        let stats = cache.stats();
        assert!(
            stats.memory_used <= 50 * 1024 * 1024,
            "Cache exceeded memory limit during concurrent access"
        );

        // Should have some hits across all threads
        assert!(total_hits > 0, "No cache hits during concurrent access");
    }

    // ================================================================================
    // Tests for 100MB+ PDF handling (Phase 4.2)
    // ================================================================================
    //
    // These tests validate RAM cache behavior for very large PDFs (100MB+ file size).
    // A 100MB+ PDF typically has 1000-2000 pages, requiring careful memory management.

    #[test]
    fn test_memory_bounded_for_1000_page_pdf_simulation() {
        // Simulate caching tiles for a 1000+ page PDF (100MB+ file)
        // Cache limit: 100MB (reasonable for a PDF viewer on modern hardware)
        let cache_limit_mb = 100;
        let cache = RamTileCache::with_mb_limit(cache_limit_mb);

        let tile_size = 256 * 256 * 4; // 256KB per tile
        let _page_count = 1000; // Document has 1000 pages
        let tiles_per_page = 12;

        // Simulate scrolling through a portion of the document
        // (can't cache all 12,000 tiles in 100MB)
        let pages_to_view = 200; // Simulate viewing 200 pages

        for page_index in 0..pages_to_view {
            for tile_index in 0..tiles_per_page {
                let key = (page_index * tiles_per_page + tile_index) as u64;
                let pixels = vec![0u8; tile_size];
                cache.put(key, pixels, 256, 256);
            }
        }

        // Memory should stay bounded at 100MB
        let stats = cache.stats();
        assert!(
            stats.memory_used <= cache_limit_mb * 1024 * 1024,
            "Cache exceeded memory limit: {} > {}",
            stats.memory_used,
            cache_limit_mb * 1024 * 1024
        );

        // Should have evicted many tiles (200 pages * 12 tiles = 2400 tiles)
        // But 100MB can only hold ~400 tiles
        assert!(stats.evictions > 0, "Expected evictions for large PDF");

        // Cache should only hold ~400 tiles (100MB / 256KB ≈ 400)
        let expected_max_tiles = (cache_limit_mb * 1024 * 1024) / tile_size;
        assert!(
            stats.tile_count <= expected_max_tiles,
            "Too many tiles in cache: {} > {}",
            stats.tile_count,
            expected_max_tiles
        );
    }

    #[test]
    fn test_working_set_efficiency_for_100mb_pdf() {
        // Test that the cache efficiently maintains a working set
        // for viewport-based rendering of large documents
        let cache = RamTileCache::with_mb_limit(50); // 50MB cache

        let tile_size = 256 * 256 * 4;

        // Simulate rendering viewport pages with prefetching
        // Working set: current page (12 tiles) + 2 adjacent pages (24 tiles) = 36 tiles
        let working_set_size = 36;
        let pixels = vec![0u8; tile_size];

        // First, populate working set for pages 0-2
        for page in 0..3 {
            for tile in 0..12 {
                let key = (page * 12 + tile) as u64;
                cache.put(key, pixels.clone(), 256, 256);
            }
        }

        // Now simulate scrolling: read current working set, add new page
        for current_page in 3..100 {
            // Read working set (should have high hit rate)
            let working_set_start = (current_page - 2) * 12;
            let mut working_set_hits = 0;

            for offset in 0..(working_set_size as i32) {
                let key = (working_set_start as i32 + offset) as u64;
                if cache.get(key).is_some() {
                    working_set_hits += 1;
                }
            }

            // Add tiles for next page (prefetch)
            for tile in 0..12 {
                let key = ((current_page + 1) * 12 + tile) as u64;
                cache.put(key, pixels.clone(), 256, 256);
            }

            // Working set should mostly be cached
            let hit_rate = working_set_hits as f64 / working_set_size as f64;
            assert!(
                hit_rate > 0.5,
                "Working set hit rate too low at page {}: {:.2}",
                current_page,
                hit_rate
            );
        }
    }

    #[test]
    fn test_rapid_page_navigation_for_large_pdf() {
        use std::time::Instant;

        // Simulate rapid page navigation (user jumping around in document)
        let cache = RamTileCache::with_mb_limit(100);
        let tile_size = 256 * 256 * 4;
        let pixels = vec![0u8; tile_size];

        let start = Instant::now();

        // Simulate jumping to random pages in a 1000-page document
        let page_jumps = [0, 500, 100, 800, 250, 999, 50, 750, 300, 600];

        for &page in &page_jumps {
            // Load tiles for this page
            for tile in 0..12 {
                let key = (page * 12 + tile) as u64;
                cache.put(key, pixels.clone(), 256, 256);
            }

            // Immediately try to read them back (simulating render)
            for tile in 0..12 {
                let key = (page * 12 + tile) as u64;
                let _ = cache.get(key);
            }
        }

        let elapsed = start.elapsed();

        // Should complete quickly (under 1 second for 120 tiles * 10 jumps)
        assert!(
            elapsed.as_secs() < 2,
            "Page navigation too slow: {:?}",
            elapsed
        );

        // Verify cache state is consistent
        let stats = cache.stats();
        assert!(stats.tile_count > 0, "Cache should have tiles");
        assert!(
            stats.memory_used <= 100 * 1024 * 1024,
            "Memory limit exceeded"
        );
    }

    #[test]
    fn test_memory_pressure_recovery_for_large_pdf() {
        // Test that cache properly recovers from memory pressure
        let cache = RamTileCache::with_mb_limit(100);
        let tile_size = 256 * 256 * 4;
        let pixels = vec![0u8; tile_size];

        // Fill cache to capacity
        let tiles_to_fill = (100 * 1024 * 1024) / tile_size + 50; // Overfill
        for i in 0..tiles_to_fill {
            cache.put(i as u64, pixels.clone(), 256, 256);
        }

        // Verify memory is bounded
        assert!(
            cache.memory_used() <= 100 * 1024 * 1024,
            "Memory not bounded after overfill"
        );

        // Simulate memory pressure: reduce limit
        cache.set_memory_limit(50 * 1024 * 1024);

        // Cache should evict to meet new limit
        assert!(
            cache.memory_used() <= 50 * 1024 * 1024,
            "Failed to reduce memory after limit change: {}",
            cache.memory_used()
        );

        // New tiles should still work
        cache.put(999999, pixels.clone(), 256, 256);
        assert!(cache.contains(999999), "Failed to add new tile after limit reduction");
    }
}
