//! PDF Editor Cache Library
//!
//! Tile cache system with RAM, VRAM, and disk storage with LRU eviction.

pub mod ram;

pub use ram::{RamTileCache, CacheStats};
