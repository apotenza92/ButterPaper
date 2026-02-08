//! Application core components: editor, document tabs, and menus.

mod document;
mod editor;
mod menus;

#[allow(unused_imports)]
pub use document::DocumentTab;
pub use editor::{BenchmarkPerfSnapshot, PdfEditor};
pub use menus::set_menus;
