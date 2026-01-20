//! Persistent disk cache for tile storage with content-addressed storage and LRU eviction.
//!
//! This module provides a disk-based cache for storing rendered tiles persistently.
//! Uses content-addressed storage where tiles are identified by their cache key hash.
//! Implements LRU eviction to maintain disk space within configured limits.

use std::collections::{HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Type alias for cache keys (typically hash from TileId)
pub type CacheKey = u64;

/// Statistics for monitoring disk cache performance
#[derive(Debug, Clone, Default)]
pub struct DiskCacheStats {
    /// Number of cache hits (successful retrievals)
    pub hits: u64,
    /// Number of cache misses (failed retrievals)
    pub misses: u64,
    /// Number of tiles evicted to free space
    pub evictions: u64,
    /// Total number of tiles in cache
    pub tile_count: usize,
    /// Total disk space used in bytes
    pub disk_used: usize,
}

impl DiskCacheStats {
    /// Calculate cache hit rate as a percentage (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Calculate disk utilization as a percentage (0.0 to 1.0)
    pub fn disk_utilization(&self, limit: usize) -> f64 {
        if limit == 0 {
            0.0
        } else {
            self.disk_used as f64 / limit as f64
        }
    }
}

/// A cached tile stored on disk
#[derive(Debug, Clone)]
pub struct DiskCachedTile {
    /// Cache key identifying this tile
    pub key: CacheKey,
    /// Pixel data (RGBA format)
    pub pixels: Vec<u8>,
    /// Tile width in pixels
    pub width: u32,
    /// Tile height in pixels
    pub height: u32,
}

impl DiskCachedTile {
    /// Calculate the size of this tile's pixel data in bytes
    pub fn byte_size(&self) -> usize {
        self.pixels.len()
    }
}

/// Internal cache state
struct CacheState {
    /// Map of cache keys to file paths
    entries: HashMap<CacheKey, PathBuf>,
    /// LRU queue: front = least recently used, back = most recently used
    lru_queue: VecDeque<CacheKey>,
    /// Statistics
    stats: DiskCacheStats,
    /// Disk space limit in bytes
    disk_limit: usize,
    /// Cache directory path
    cache_dir: PathBuf,
}

impl CacheState {
    /// Touch a cache entry (mark as recently used)
    fn touch(&mut self, key: CacheKey) {
        // Remove from current position and add to back (most recent)
        self.lru_queue.retain(|&k| k != key);
        self.lru_queue.push_back(key);
    }

    /// Evict the least recently used entry
    fn evict_lru(&mut self) -> io::Result<()> {
        if let Some(key) = self.lru_queue.pop_front() {
            if let Some(path) = self.entries.remove(&key) {
                // Get file size before removing
                let file_size = fs::metadata(&path)
                    .map(|m| m.len() as usize)
                    .unwrap_or(0);

                // Remove file
                if let Err(e) = fs::remove_file(&path) {
                    // If file doesn't exist, that's okay
                    if e.kind() != io::ErrorKind::NotFound {
                        return Err(e);
                    }
                }

                self.stats.disk_used = self.stats.disk_used.saturating_sub(file_size);
                self.stats.tile_count = self.stats.tile_count.saturating_sub(1);
                self.stats.evictions += 1;
            }
        }
        Ok(())
    }

    /// Evict entries until we have at least `needed_space` bytes available
    fn evict_until_space_available(&mut self, needed_space: usize) -> io::Result<()> {
        while self.stats.disk_used + needed_space > self.disk_limit && !self.lru_queue.is_empty() {
            self.evict_lru()?;
        }
        Ok(())
    }

    /// Calculate actual disk usage by scanning cache directory
    fn recalculate_disk_usage(&mut self) -> io::Result<()> {
        let mut total_size = 0;

        for path in self.entries.values() {
            if let Ok(metadata) = fs::metadata(path) {
                total_size += metadata.len() as usize;
            }
        }

        self.stats.disk_used = total_size;
        Ok(())
    }
}

/// Persistent disk cache for storing rendered tiles
///
/// Uses content-addressed storage with LRU eviction to manage disk space.
/// Thread-safe for concurrent access from multiple threads.
#[derive(Clone)]
pub struct DiskTileCache {
    state: Arc<Mutex<CacheState>>,
}

impl DiskTileCache {
    /// Create a new disk cache with specified cache directory and disk limit in bytes
    pub fn new<P: AsRef<Path>>(cache_dir: P, disk_limit: usize) -> io::Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();

        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir)?;

        let state = CacheState {
            entries: HashMap::new(),
            lru_queue: VecDeque::new(),
            stats: DiskCacheStats::default(),
            disk_limit,
            cache_dir,
        };

        Ok(Self {
            state: Arc::new(Mutex::new(state)),
        })
    }

    /// Create a new disk cache with disk limit specified in megabytes
    pub fn with_mb_limit<P: AsRef<Path>>(cache_dir: P, megabytes: usize) -> io::Result<Self> {
        Self::new(cache_dir, megabytes * 1024 * 1024)
    }

    /// Generate file path for a cache key
    fn key_to_path(cache_dir: &Path, key: CacheKey) -> PathBuf {
        // Use hex encoding for filename
        cache_dir.join(format!("{:016x}.tile", key))
    }

    /// Store a tile in the cache
    ///
    /// If the cache is full, evicts least recently used tiles to make space.
    /// If a tile with the same key already exists, it will be replaced.
    pub fn put(&self, key: CacheKey, pixels: Vec<u8>, width: u32, height: u32) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();

        let tile_size = pixels.len();

        // Remove existing entry if present
        if let Some(old_path) = state.entries.remove(&key) {
            let old_size = fs::metadata(&old_path)
                .map(|m| m.len() as usize)
                .unwrap_or(0);

            fs::remove_file(&old_path).ok(); // Ignore errors
            state.stats.disk_used = state.stats.disk_used.saturating_sub(old_size);
            state.stats.tile_count = state.stats.tile_count.saturating_sub(1);
            state.lru_queue.retain(|&k| k != key);
        }

        // Evict until we have space
        state.evict_until_space_available(tile_size)?;

        // Write tile to disk
        let path = Self::key_to_path(&state.cache_dir, key);
        let mut file = File::create(&path)?;

        // Write header: width (4 bytes) + height (4 bytes)
        file.write_all(&width.to_le_bytes())?;
        file.write_all(&height.to_le_bytes())?;

        // Write pixel data
        file.write_all(&pixels)?;
        file.sync_all()?;

        // Update cache state
        state.entries.insert(key, path);
        state.lru_queue.push_back(key);
        state.stats.disk_used += tile_size + 8; // +8 for header
        state.stats.tile_count += 1;

        Ok(())
    }

    /// Retrieve a tile from the cache
    ///
    /// Returns `None` if the tile is not in the cache.
    /// Updates LRU order on successful retrieval.
    ///
    /// This is a blocking operation that will wait if the cache is currently locked.
    /// For non-blocking access, use `try_get()`.
    pub fn get(&self, key: CacheKey) -> io::Result<Option<DiskCachedTile>> {
        let mut state = self.state.lock().unwrap();

        if let Some(path) = state.entries.get(&key) {
            // Read tile from disk
            let mut file = File::open(path)?;

            // Read header
            let mut width_bytes = [0u8; 4];
            let mut height_bytes = [0u8; 4];
            file.read_exact(&mut width_bytes)?;
            file.read_exact(&mut height_bytes)?;

            let width = u32::from_le_bytes(width_bytes);
            let height = u32::from_le_bytes(height_bytes);

            // Read pixel data
            let mut pixels = Vec::new();
            file.read_to_end(&mut pixels)?;

            // Update LRU
            state.touch(key);
            state.stats.hits += 1;

            Ok(Some(DiskCachedTile {
                key,
                pixels,
                width,
                height,
            }))
        } else {
            state.stats.misses += 1;
            Ok(None)
        }
    }

    /// Try to retrieve a tile from the cache without blocking
    ///
    /// Returns `Ok(Some(Some(tile)))` if the tile is in the cache and the lock was acquired,
    /// `Ok(Some(None))` if the lock was acquired but the tile was not found,
    /// or `Ok(None)` if the cache is currently locked and the operation would block.
    /// Returns `Err` if there was an I/O error reading the tile.
    ///
    /// This is a non-blocking operation that returns immediately if the cache is locked.
    /// Updates LRU order on successful retrieval.
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key for the tile to retrieve
    ///
    /// # Returns
    ///
    /// - `Ok(Some(Some(tile)))` - Cache hit, tile retrieved successfully
    /// - `Ok(Some(None))` - Cache miss, no tile with this key
    /// - `Ok(None)` - Could not acquire lock (cache is busy)
    /// - `Err(e)` - I/O error reading tile from disk
    pub fn try_get(&self, key: CacheKey) -> io::Result<Option<Option<DiskCachedTile>>> {
        let mut state = match self.state.try_lock() {
            Ok(guard) => guard,
            Err(_) => return Ok(None), // Cache is locked
        };

        if let Some(path) = state.entries.get(&key) {
            // Read tile from disk
            let mut file = File::open(path)?;

            // Read header
            let mut width_bytes = [0u8; 4];
            let mut height_bytes = [0u8; 4];
            file.read_exact(&mut width_bytes)?;
            file.read_exact(&mut height_bytes)?;

            let width = u32::from_le_bytes(width_bytes);
            let height = u32::from_le_bytes(height_bytes);

            // Read pixel data
            let mut pixels = Vec::new();
            file.read_to_end(&mut pixels)?;

            // Update LRU
            state.touch(key);
            state.stats.hits += 1;

            Ok(Some(Some(DiskCachedTile {
                key,
                pixels,
                width,
                height,
            })))
        } else {
            state.stats.misses += 1;
            Ok(Some(None))
        }
    }

    /// Check if a tile is in the cache without updating LRU order
    pub fn contains(&self, key: CacheKey) -> bool {
        let state = self.state.lock().unwrap();
        state.entries.contains_key(&key)
    }

    /// Remove a tile from the cache
    pub fn remove(&self, key: CacheKey) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();

        if let Some(path) = state.entries.remove(&key) {
            let file_size = fs::metadata(&path)
                .map(|m| m.len() as usize)
                .unwrap_or(0);

            fs::remove_file(&path)?;

            state.stats.disk_used = state.stats.disk_used.saturating_sub(file_size);
            state.stats.tile_count = state.stats.tile_count.saturating_sub(1);
            state.lru_queue.retain(|&k| k != key);
        }

        Ok(())
    }

    /// Clear all tiles from the cache
    pub fn clear(&self) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();

        // Remove all files
        for (_key, path) in state.entries.drain() {
            fs::remove_file(&path).ok(); // Ignore errors
        }

        state.lru_queue.clear();
        state.stats.tile_count = 0;
        state.stats.disk_used = 0;

        Ok(())
    }

    /// Get current cache statistics
    pub fn stats(&self) -> DiskCacheStats {
        let state = self.state.lock().unwrap();
        state.stats.clone()
    }

    /// Get disk space limit in bytes
    pub fn disk_limit(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.disk_limit
    }

    /// Get current disk usage in bytes
    pub fn disk_used(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.stats.disk_used
    }

    /// Get number of tiles in cache
    pub fn tile_count(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.stats.tile_count
    }

    /// Update disk space limit
    ///
    /// If the new limit is lower than current usage, evicts tiles until within limit.
    pub fn set_disk_limit(&self, new_limit: usize) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();
        state.disk_limit = new_limit;

        // Evict if over limit
        while state.stats.disk_used > state.disk_limit && !state.lru_queue.is_empty() {
            state.evict_lru()?;
        }

        Ok(())
    }

    /// Recalculate disk usage by scanning cache directory
    ///
    /// Useful for recovering from inconsistent state or external modifications.
    pub fn recalculate_disk_usage(&self) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();
        state.recalculate_disk_usage()
    }

    /// Load existing cache entries from disk directory
    ///
    /// Scans the cache directory and adds all valid tile files to the cache.
    /// Useful for restoring cache state after application restart.
    pub fn load_from_disk(&self) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();

        // Clear existing state
        state.entries.clear();
        state.lru_queue.clear();
        state.stats.tile_count = 0;
        state.stats.disk_used = 0;

        // Scan cache directory
        for entry in fs::read_dir(&state.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Check if it's a .tile file
            if path.extension().and_then(|s| s.to_str()) == Some("tile") {
                // Extract key from filename
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(key) = u64::from_str_radix(filename, 16) {
                        let file_size = entry.metadata()?.len() as usize;

                        state.entries.insert(key, path);
                        state.lru_queue.push_back(key);
                        state.stats.tile_count += 1;
                        state.stats.disk_used += file_size;
                    }
                }
            }
        }

        Ok(())
    }

    /// Get cache directory path
    pub fn cache_dir(&self) -> PathBuf {
        let state = self.state.lock().unwrap();
        state.cache_dir.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn create_test_cache() -> (DiskTileCache, PathBuf) {
        let cache_dir = env::temp_dir().join(format!("pdf-editor-test-{}", rand::random::<u32>()));
        let cache = DiskTileCache::with_mb_limit(&cache_dir, 1).unwrap();
        (cache, cache_dir)
    }

    fn cleanup_test_cache(cache_dir: PathBuf) {
        fs::remove_dir_all(cache_dir).ok();
    }

    #[test]
    fn test_basic_put_get() {
        let (cache, cache_dir) = create_test_cache();

        let pixels = vec![255u8; 256 * 256 * 4];
        cache.put(1, pixels.clone(), 256, 256).unwrap();

        let retrieved = cache.get(1).unwrap().unwrap();
        assert_eq!(retrieved.key, 1);
        assert_eq!(retrieved.width, 256);
        assert_eq!(retrieved.height, 256);
        assert_eq!(retrieved.pixels, pixels);

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_cache_miss() {
        let (cache, cache_dir) = create_test_cache();

        let result = cache.get(999).unwrap();
        assert!(result.is_none());

        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 0);

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_lru_eviction() {
        let (cache, cache_dir) = create_test_cache();

        // Create tiles that will exceed 1MB limit
        let pixels = vec![255u8; 300 * 1024]; // 300KB each

        cache.put(1, pixels.clone(), 256, 256).unwrap();
        cache.put(2, pixels.clone(), 256, 256).unwrap();
        cache.put(3, pixels.clone(), 256, 256).unwrap();
        cache.put(4, pixels.clone(), 256, 256).unwrap(); // Should trigger eviction

        // Tile 1 should have been evicted (least recently used)
        assert!(!cache.contains(1));
        assert!(cache.contains(2));
        assert!(cache.contains(3));
        assert!(cache.contains(4));

        let stats = cache.stats();
        assert!(stats.evictions > 0);

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_lru_ordering() {
        let (cache, cache_dir) = create_test_cache();

        let pixels = vec![255u8; 300 * 1024];

        cache.put(1, pixels.clone(), 256, 256).unwrap();
        cache.put(2, pixels.clone(), 256, 256).unwrap();
        cache.put(3, pixels.clone(), 256, 256).unwrap();

        // Access tile 1 to make it most recently used
        cache.get(1).unwrap();

        // Add tile 4, which should evict tile 2 (now least recently used)
        cache.put(4, pixels.clone(), 256, 256).unwrap();

        assert!(cache.contains(1)); // Still present (accessed recently)
        assert!(!cache.contains(2)); // Evicted
        assert!(cache.contains(3));
        assert!(cache.contains(4));

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_contains() {
        let (cache, cache_dir) = create_test_cache();

        let pixels = vec![255u8; 1024];
        cache.put(1, pixels, 256, 256).unwrap();

        assert!(cache.contains(1));
        assert!(!cache.contains(2));

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_remove() {
        let (cache, cache_dir) = create_test_cache();

        let pixels = vec![255u8; 1024];
        cache.put(1, pixels, 256, 256).unwrap();

        assert_eq!(cache.tile_count(), 1);

        cache.remove(1).unwrap();

        assert_eq!(cache.tile_count(), 0);
        assert!(!cache.contains(1));

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_clear() {
        let (cache, cache_dir) = create_test_cache();

        let pixels = vec![255u8; 1024];
        cache.put(1, pixels.clone(), 256, 256).unwrap();
        cache.put(2, pixels.clone(), 256, 256).unwrap();
        cache.put(3, pixels, 256, 256).unwrap();

        assert_eq!(cache.tile_count(), 3);

        cache.clear().unwrap();

        assert_eq!(cache.tile_count(), 0);
        assert_eq!(cache.disk_used(), 0);

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_stats() {
        let (cache, cache_dir) = create_test_cache();

        let pixels = vec![255u8; 1024];
        cache.put(1, pixels.clone(), 256, 256).unwrap();

        cache.get(1).unwrap();
        cache.get(2).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate(), 0.5);

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_disk_tracking() {
        let (cache, cache_dir) = create_test_cache();

        let pixels = vec![255u8; 1024];
        cache.put(1, pixels, 256, 256).unwrap();

        assert!(cache.disk_used() > 1024); // Should include header
        assert_eq!(cache.tile_count(), 1);

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_set_disk_limit() {
        let (cache, cache_dir) = create_test_cache();

        let pixels = vec![255u8; 100 * 1024]; // 100KB
        cache.put(1, pixels.clone(), 256, 256).unwrap();
        cache.put(2, pixels.clone(), 256, 256).unwrap();

        assert_eq!(cache.tile_count(), 2);

        // Lower limit should trigger eviction
        cache.set_disk_limit(150 * 1024).unwrap();

        assert_eq!(cache.tile_count(), 1);

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_update_existing_tile() {
        let (cache, cache_dir) = create_test_cache();

        let pixels1 = vec![255u8; 1024];
        let pixels2 = vec![128u8; 2048];

        cache.put(1, pixels1, 256, 256).unwrap();
        cache.put(1, pixels2.clone(), 512, 512).unwrap();

        let retrieved = cache.get(1).unwrap().unwrap();
        assert_eq!(retrieved.width, 512);
        assert_eq!(retrieved.height, 512);
        assert_eq!(retrieved.pixels, pixels2);

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_load_from_disk() {
        let (cache, cache_dir) = create_test_cache();

        let pixels = vec![255u8; 1024];
        cache.put(1, pixels.clone(), 256, 256).unwrap();
        cache.put(2, pixels.clone(), 256, 256).unwrap();

        // Create new cache instance pointing to same directory
        let cache2 = DiskTileCache::with_mb_limit(&cache_dir, 1).unwrap();
        cache2.load_from_disk().unwrap();

        assert_eq!(cache2.tile_count(), 2);
        assert!(cache2.contains(1));
        assert!(cache2.contains(2));

        let retrieved = cache2.get(1).unwrap().unwrap();
        assert_eq!(retrieved.pixels, pixels);

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_recalculate_disk_usage() {
        let (cache, cache_dir) = create_test_cache();

        let pixels = vec![255u8; 1024];
        cache.put(1, pixels, 256, 256).unwrap();

        let original_usage = cache.disk_used();

        cache.recalculate_disk_usage().unwrap();

        assert_eq!(cache.disk_used(), original_usage);

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_disk_utilization() {
        let (cache, cache_dir) = create_test_cache();

        let pixels = vec![255u8; 500 * 1024]; // 500KB
        cache.put(1, pixels, 256, 256).unwrap();

        let stats = cache.stats();
        let utilization = stats.disk_utilization(cache.disk_limit());

        assert!(utilization > 0.4); // Should be around 50%
        assert!(utilization < 0.6);

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_try_get_non_blocking() {
        let (cache, cache_dir) = create_test_cache();

        let pixels = vec![255u8; 256 * 256 * 4];
        cache.put(1, pixels.clone(), 256, 256).unwrap();

        // try_get should succeed when cache is not locked
        match cache.try_get(1).unwrap() {
            Some(Some(tile)) => {
                assert_eq!(tile.key, 1);
                assert_eq!(tile.width, 256);
                assert_eq!(tile.height, 256);
                assert_eq!(tile.pixels, pixels);
            }
            _ => panic!("Expected cache hit"),
        }

        // try_get should return None when key doesn't exist
        match cache.try_get(999).unwrap() {
            Some(None) => {
                // Expected: cache miss
            }
            _ => panic!("Expected cache miss"),
        }

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);

        cleanup_test_cache(cache_dir);
    }

    #[test]
    fn test_try_get_lru_update() {
        let (cache, cache_dir) = create_test_cache();

        let pixels = vec![255u8; 300 * 1024];
        cache.put(1, pixels.clone(), 256, 256).unwrap();
        cache.put(2, pixels.clone(), 256, 256).unwrap();
        cache.put(3, pixels.clone(), 256, 256).unwrap();

        // Access tile 1 via try_get (should update LRU)
        assert!(matches!(cache.try_get(1).unwrap(), Some(Some(_))));

        // Add tile 4, should evict tile 2 (now least recently used)
        cache.put(4, pixels.clone(), 256, 256).unwrap();

        assert!(cache.contains(1)); // Still present (accessed via try_get)
        assert!(!cache.contains(2)); // Evicted
        assert!(cache.contains(3)); // Present
        assert!(cache.contains(4)); // Present

        cleanup_test_cache(cache_dir);
    }
}
