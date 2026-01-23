//! Document tab representation for multi-document PDF editing.

use gpui::Entity;
use std::path::PathBuf;

use crate::components::tab_bar::TabId as UiTabId;
use crate::sidebar::ThumbnailSidebar;
use crate::viewport::PdfViewport;

/// A document tab containing the viewport and sidebar for a single PDF.
pub struct DocumentTab {
    pub id: UiTabId,
    pub path: PathBuf,
    pub title: String,
    pub viewport: Entity<PdfViewport>,
    pub sidebar: Entity<ThumbnailSidebar>,
    pub is_dirty: bool,
}
