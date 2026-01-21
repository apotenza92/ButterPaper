//! PDF Editor Core Library
//!
//! Document core and state model for the PDF editor.

pub mod annotation;
pub mod annotation_import;
pub mod checkpoint;
pub mod csv_export;
pub mod deferred;
pub mod document;
pub mod font_bridge;
pub mod loader;
pub mod manipulation;
pub mod measurement;
pub mod ocr;
pub mod page_switch;
pub mod pdf_export;
pub mod persistence;
pub mod preview;
pub mod progressive_ocr;
pub mod scale_detection;
pub mod snapping;
pub mod text_edit;
pub mod text_layer;
pub mod text_layout;
pub mod write_coordinator;

pub use annotation::{
    Annotation, AnnotationCollection, AnnotationGeometry, AnnotationId, AnnotationMetadata,
    AnnotationStyle, Color, PageCoordinate, SerializableAnnotation,
};
pub use checkpoint::{CheckpointManager, CheckpointMetadata};
pub use csv_export::{
    export_annotations_csv, export_measurements_csv, export_scales_csv, CsvExportConfig,
    CsvExportError, CsvExportResult,
};
pub use deferred::{DeferredJob, DeferredJobConfig, DeferredJobScheduler, DeferredJobType};
pub use document::{
    Document, DocumentError, DocumentId, DocumentManager, DocumentMetadata, DocumentResult,
    DocumentState,
};
pub use font_bridge::{extract_font_for_region, font_info_to_text_edit_font};
pub use loader::{DocumentLoader, LoaderConfig};
pub use manipulation::{generate_handles, HandleType, ManipulationHandle, ManipulationState};
pub use measurement::{
    Measurement, MeasurementCollection, MeasurementId, MeasurementMetadata, MeasurementType,
    ScaleSystem, ScaleSystemId, ScaleType,
};
pub use ocr::{OcrConfig, OcrEngine, OcrError, OcrResult, TextBlock};
pub use page_switch::{PageSwitchResult, PageSwitcher};
pub use pdf_export::{
    export_flattened_pdf, generate_appearance_stream, save_pdf_with_annotations, ExportOptions,
    PdfExportError, PdfExportResult,
};
pub use persistence::{
    delete_metadata, load_metadata, metadata_exists, metadata_path, save_metadata,
    PersistenceError, PersistenceResult,
};
pub use preview::{AsyncPreviewRenderer, PreviewHandle, PreviewRenderer, PreviewResult};
pub use progressive_ocr::{ProgressiveOcr, ProgressiveOcrStats};
pub use scale_detection::{detect_scales, get_best_scale, DetectedScale};
pub use snapping::{SnapConfig, SnapEngine, SnapTarget, SnapType};
pub use text_edit::{PageTextEdits, TextEdit, TextEditFont, TextEditId, TextEditManager};
pub use text_layer::{
    PageTextLayer, SearchMatch, TextBoundingBox, TextLayerManager, TextLayerStats, TextSpan,
};
pub use text_layout::{LayoutAdjustment, LayoutConfig, LayoutStrategy, TextLayoutAdjuster};
pub use write_coordinator::{WriteCoordinator, WriteCoordinatorConfig};
pub use annotation_import::{
    import_annotations, load_annotations_from_page, load_annotations_from_pdf,
    AnnotationImportError, AnnotationImportResult, ImportStats,
};
