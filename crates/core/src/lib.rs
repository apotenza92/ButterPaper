//! PDF Editor Core Library
//!
//! Document core and state model for the PDF editor.

pub mod deferred;
pub mod document;
pub mod loader;
pub mod page_switch;
pub mod preview;

pub use deferred::{DeferredJob, DeferredJobConfig, DeferredJobScheduler, DeferredJobType};
pub use document::{
    Document, DocumentError, DocumentId, DocumentManager, DocumentMetadata, DocumentResult,
    DocumentState,
};
pub use loader::{DocumentLoader, LoaderConfig};
pub use page_switch::{PageSwitchResult, PageSwitcher};
pub use preview::{AsyncPreviewRenderer, PreviewRenderer, PreviewResult, PreviewHandle};
