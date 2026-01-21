//! PDF Editor Render Library
//!
//! PDF render pipeline with tile-based rendering, preview and crisp profiles.

pub mod pdf;
pub mod progressive;
pub mod tile;

pub use pdf::{detect_needs_ocr, PageDimensions, PdfDocument, PdfError, PdfMetadata, PdfResult};
pub use progressive::{ProgressCallback, ProgressiveTileLoader, TileState};
pub use tile::{RenderedTile, TileCoordinate, TileId, TileProfile, TileRenderer};
