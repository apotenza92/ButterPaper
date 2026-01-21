//! PDF Editor Cache Library
//!
//! Tile cache system with RAM, VRAM, and disk storage with LRU eviction.
//!
//! ## Features
//!
//! - **RAM Cache**: Fast in-memory tile storage with LRU eviction
//! - **GPU Cache**: VRAM texture caching for GPU-rendered tiles
//! - **Disk Cache**: Persistent tile storage for large documents
//! - **Memory Budget**: Unified memory tracking across all cache tiers
//!
//! ## Memory Bounded Usage
//!
//! The cache system ensures memory usage stays bounded through:
//!
//! 1. Per-cache LRU eviction when individual limits are reached
//! 2. Unified memory budget tracking across RAM and GPU caches
//! 3. Memory pressure detection with configurable thresholds
//! 4. Proactive eviction recommendations when pressure is high
//!
//! ```
//! use pdf_editor_cache::{RamTileCache, GpuTextureCache};
//! use pdf_editor_cache::memory_budget::{CacheMonitor, MemoryBudgetConfig};
//!
//! // Create caches with individual limits
//! let ram_cache = RamTileCache::with_mb_limit(256);
//! let gpu_cache = GpuTextureCache::with_mb_limit(512);
//!
//! // Create a monitor to track combined usage
//! let monitor = CacheMonitor::with_limit_mb(768);
//!
//! // Check memory pressure before loading new content
//! if monitor.needs_eviction() {
//!     // Handle memory pressure
//! }
//! ```

pub mod config;
pub mod disk;
pub mod gpu;
pub mod memory_budget;
pub mod ram;

pub use config::{CacheConfig, ConfigError};
pub use disk::{DiskCacheStats, DiskCachedTile, DiskTileCache};
pub use gpu::{GpuCacheStats, GpuTexture, GpuTextureCache, TextureMetadata, TextureRef};
pub use memory_budget::{
    AggregatedCacheStats, CacheMonitor, MemoryBudget, MemoryBudgetConfig, MemoryCheckResult,
    MemoryPressure,
};
pub use ram::{CacheStats, RamTileCache};
