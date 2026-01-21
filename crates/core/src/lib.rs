//! PDF Editor Core Library
//!
//! Document core and state model for the PDF editor.

pub mod deferred;
pub mod document;
pub mod loader;
pub mod preview;

pub use deferred::{DeferredJob, DeferredJobConfig, DeferredJobScheduler, DeferredJobType};
pub use document::{
    Document, DocumentError, DocumentId, DocumentManager, DocumentMetadata, DocumentResult,
    DocumentState,
};
pub use loader::{DocumentLoader, LoaderConfig};
pub use preview::{AsyncPreviewRenderer, PreviewRenderer, PreviewResult, PreviewHandle};
