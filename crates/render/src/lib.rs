//! PDF Editor Render Library
//!
//! PDF render pipeline with tile-based rendering, preview and crisp profiles.

pub mod font_info;
pub mod pdf;
pub mod progressive;
pub mod tile;

pub use font_info::{
    extract_fonts_from_page, find_font_in_region, get_page_fonts, FontInfo, TextSpanWithFont,
};
pub use pdf::{detect_needs_ocr, PageDimensions, PdfDocument, PdfError, PdfMetadata, PdfResult, SaveError, TextSpanInfo};
pub use progressive::{ProgressCallback, ProgressiveTileLoader, TileState};
pub use tile::{RenderedTile, TileCoordinate, TileId, TileProfile, TileRenderer};
