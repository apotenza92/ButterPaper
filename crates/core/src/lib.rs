//! PDF Editor Core Library
//!
//! Document core and state model for the PDF editor.

pub mod document;
pub mod loader;

pub use document::{
    Document, DocumentError, DocumentId, DocumentManager, DocumentMetadata, DocumentResult,
    DocumentState,
};
pub use loader::{DocumentLoader, LoaderConfig};
