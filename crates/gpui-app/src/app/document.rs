//! Document tab representation for multi-document PDF editing.

use gpui::Entity;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::components::tab_bar::TabId as UiTabId;
use crate::preview_cache::SharedPreviewCache;
use crate::sidebar::ThumbnailSidebar;
use crate::viewport::PdfViewport;

/// A document tab containing the viewport and sidebar for a single PDF.
pub struct DocumentTab {
    pub id: UiTabId,
    /// File path to the PDF. None for welcome tabs.
    pub path: Option<PathBuf>,
    pub title: String,
    pub viewport: Entity<PdfViewport>,
    pub sidebar: Entity<ThumbnailSidebar>,
    pub preview_cache: Arc<Mutex<SharedPreviewCache>>,
    pub is_dirty: bool,
}

impl DocumentTab {
    /// Returns true if this is a welcome tab (no file loaded).
    pub fn is_welcome(&self) -> bool {
        self.path.is_none()
    }
}
