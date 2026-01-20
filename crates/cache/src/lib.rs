//! PDF Editor Cache Library
//!
//! Tile cache system with RAM, VRAM, and disk storage with LRU eviction.

pub mod ram;
pub mod gpu;
pub mod disk;

pub use ram::{RamTileCache, CacheStats};
pub use gpu::{GpuTextureCache, GpuCacheStats, GpuTexture, TextureRef, TextureMetadata};
pub use disk::{DiskTileCache, DiskCacheStats, DiskCachedTile};
