//! PDF Editor Core Library
//!
//! Document core and state model for the PDF editor.

pub mod annotation;
pub mod deferred;
pub mod document;
pub mod loader;
pub mod manipulation;
pub mod measurement;
pub mod page_switch;
pub mod preview;
pub mod snapping;

pub use annotation::{
    Annotation, AnnotationCollection, AnnotationGeometry, AnnotationId, AnnotationMetadata,
    AnnotationStyle, Color, PageCoordinate,
};
pub use deferred::{DeferredJob, DeferredJobConfig, DeferredJobScheduler, DeferredJobType};
pub use document::{
    Document, DocumentError, DocumentId, DocumentManager, DocumentMetadata, DocumentResult,
    DocumentState,
};
pub use loader::{DocumentLoader, LoaderConfig};
pub use manipulation::{
    generate_handles, HandleType, ManipulationHandle, ManipulationState,
};
pub use measurement::{
    Measurement, MeasurementCollection, MeasurementId, MeasurementMetadata, MeasurementType,
    ScaleSystem, ScaleSystemId, ScaleType,
};
pub use page_switch::{PageSwitchResult, PageSwitcher};
pub use preview::{AsyncPreviewRenderer, PreviewRenderer, PreviewHandle, PreviewResult};
pub use snapping::{SnapConfig, SnapEngine, SnapTarget, SnapType};
