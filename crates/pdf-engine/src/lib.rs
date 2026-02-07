use image::{ImageBuffer, Rgba};
use lopdf::Document;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub type RgbaImage = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentHandle(u64);

impl DocumentHandle {
    pub fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageSize {
    pub width_pt: f32,
    pub height_pt: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderRequest {
    pub page_index: u32,
    pub scale: f32,
    pub clip: Option<ClipRect>,
}

impl Default for RenderRequest {
    fn default() -> Self {
        Self { page_index: 0, scale: 1.0, clip: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThumbnailSize {
    pub width_px: u32,
    pub height_px: u32,
}

impl Default for ThumbnailSize {
    fn default() -> Self {
        Self { width_px: 256, height_px: 256 }
    }
}

#[derive(Debug, Clone)]
pub enum OpenSource {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

impl From<PathBuf> for OpenSource {
    fn from(value: PathBuf) -> Self {
        Self::Path(value)
    }
}

impl From<&Path> for OpenSource {
    fn from(value: &Path) -> Self {
        Self::Path(value.to_path_buf())
    }
}

impl From<Vec<u8>> for OpenSource {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(value)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PdfEngineError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("PDF parse error: {0}")]
    Parse(#[from] lopdf::Error),
    #[error("invalid handle {0}")]
    InvalidHandle(u64),
    #[error("page {page} out of range (page_count={page_count})")]
    PageOutOfRange { page: u32, page_count: u32 },
    #[error("encrypted PDFs are not supported in the default backend")]
    EncryptedUnsupported,
    #[error("backend error: {0}")]
    Backend(String),
}

pub trait PdfEngine {
    fn open(&mut self, source: OpenSource) -> Result<DocumentHandle, PdfEngineError>;
    fn page_count(&self, handle: DocumentHandle) -> Result<u32, PdfEngineError>;
    fn page_size(
        &self,
        handle: DocumentHandle,
        page_index: u32,
    ) -> Result<PageSize, PdfEngineError>;
    fn render_page(
        &self,
        handle: DocumentHandle,
        request: RenderRequest,
    ) -> Result<RgbaImage, PdfEngineError>;
    fn render_thumbnail(
        &self,
        handle: DocumentHandle,
        page_index: u32,
        target: ThumbnailSize,
    ) -> Result<RgbaImage, PdfEngineError>;
    fn close(&mut self, handle: DocumentHandle) -> Result<(), PdfEngineError>;
}

#[derive(Debug, Clone)]
struct DocumentRecord {
    bytes: Vec<u8>,
    page_sizes: Vec<PageSize>,
}

#[derive(Debug, Default)]
pub struct LopdfEngine {
    next_handle: u64,
    docs: HashMap<DocumentHandle, DocumentRecord>,
}

impl LopdfEngine {
    pub fn new() -> Self {
        Self::default()
    }

    fn parse_sizes(bytes: &[u8]) -> Result<Vec<PageSize>, PdfEngineError> {
        if bytes.windows("/Encrypt".len()).any(|window| window == b"/Encrypt") {
            return Err(PdfEngineError::EncryptedUnsupported);
        }

        let doc = Document::load_mem(bytes)?;
        let pages = doc.get_pages();
        let mut sizes = Vec::with_capacity(pages.len());

        for (_, object_id) in pages {
            let dict = doc.get_dictionary(object_id)?;
            let size = dict
                .get(b"MediaBox")
                .ok()
                .and_then(|obj| obj.as_array().ok())
                .and_then(|array| {
                    if array.len() != 4 {
                        return None;
                    }
                    let x0 = array[0].as_float().ok()?;
                    let y0 = array[1].as_float().ok()?;
                    let x1 = array[2].as_float().ok()?;
                    let y1 = array[3].as_float().ok()?;
                    Some(PageSize { width_pt: (x1 - x0).abs(), height_pt: (y1 - y0).abs() })
                })
                .unwrap_or(PageSize { width_pt: 612.0, height_pt: 792.0 });

            sizes.push(size);
        }

        if sizes.is_empty() {
            return Err(PdfEngineError::Backend("document has no pages".to_owned()));
        }

        Ok(sizes)
    }

    fn record(&self, handle: DocumentHandle) -> Result<&DocumentRecord, PdfEngineError> {
        self.docs.get(&handle).ok_or(PdfEngineError::InvalidHandle(handle.raw()))
    }
}

impl PdfEngine for LopdfEngine {
    fn open(&mut self, source: OpenSource) -> Result<DocumentHandle, PdfEngineError> {
        let bytes = match source {
            OpenSource::Path(path) => fs::read(path)?,
            OpenSource::Bytes(bytes) => bytes,
        };

        let page_sizes = Self::parse_sizes(&bytes)?;

        self.next_handle += 1;
        let handle = DocumentHandle(self.next_handle);
        self.docs.insert(handle, DocumentRecord { bytes, page_sizes });

        Ok(handle)
    }

    fn page_count(&self, handle: DocumentHandle) -> Result<u32, PdfEngineError> {
        Ok(self.record(handle)?.page_sizes.len() as u32)
    }

    fn page_size(
        &self,
        handle: DocumentHandle,
        page_index: u32,
    ) -> Result<PageSize, PdfEngineError> {
        let record = self.record(handle)?;
        record.page_sizes.get(page_index as usize).copied().ok_or(PdfEngineError::PageOutOfRange {
            page: page_index,
            page_count: record.page_sizes.len() as u32,
        })
    }

    fn render_page(
        &self,
        handle: DocumentHandle,
        request: RenderRequest,
    ) -> Result<RgbaImage, PdfEngineError> {
        let _ = self.record(handle)?.bytes.len();
        let page_size = self.page_size(handle, request.page_index)?;
        let scale = if request.scale <= 0.0 { 1.0 } else { request.scale };

        let mut width = (page_size.width_pt * scale).round().max(1.0) as u32;
        let mut height = (page_size.height_pt * scale).round().max(1.0) as u32;

        if let Some(clip) = request.clip {
            width = (clip.width * scale).round().max(1.0) as u32;
            height = (clip.height * scale).round().max(1.0) as u32;
        }

        let mut image = RgbaImage::from_pixel(width, height, Rgba([255, 255, 255, 255]));

        if width >= 4 && height >= 4 {
            for x in 0..width {
                image.put_pixel(x, 0, Rgba([220, 220, 220, 255]));
                image.put_pixel(x, height - 1, Rgba([220, 220, 220, 255]));
            }
            for y in 0..height {
                image.put_pixel(0, y, Rgba([220, 220, 220, 255]));
                image.put_pixel(width - 1, y, Rgba([220, 220, 220, 255]));
            }
        }

        Ok(image)
    }

    fn render_thumbnail(
        &self,
        handle: DocumentHandle,
        page_index: u32,
        target: ThumbnailSize,
    ) -> Result<RgbaImage, PdfEngineError> {
        let page =
            self.render_page(handle, RenderRequest { page_index, scale: 0.25, clip: None })?;

        Ok(image::imageops::thumbnail(&page, target.width_px.max(1), target.height_px.max(1)))
    }

    fn close(&mut self, handle: DocumentHandle) -> Result<(), PdfEngineError> {
        self.docs.remove(&handle).map(|_| ()).ok_or(PdfEngineError::InvalidHandle(handle.raw()))
    }
}

#[cfg(feature = "pdfium")]
pub mod pdfium_backend {
    use super::*;
    use pdfium_render::prelude::*;

    pub struct PdfiumEngine {
        inner: LopdfEngine,
    }

    impl PdfiumEngine {
        pub fn from_system_library() -> Result<Self, PdfEngineError> {
            let _ = Pdfium::bind_to_system_library().map_err(|err| {
                PdfEngineError::Backend(format!("failed to bind pdfium system library: {err}"))
            })?;

            Ok(Self { inner: LopdfEngine::default() })
        }
    }

    impl PdfEngine for PdfiumEngine {
        fn open(&mut self, source: OpenSource) -> Result<DocumentHandle, PdfEngineError> {
            self.inner.open(source)
        }

        fn page_count(&self, handle: DocumentHandle) -> Result<u32, PdfEngineError> {
            self.inner.page_count(handle)
        }

        fn page_size(
            &self,
            handle: DocumentHandle,
            page_index: u32,
        ) -> Result<PageSize, PdfEngineError> {
            self.inner.page_size(handle, page_index)
        }

        fn render_page(
            &self,
            handle: DocumentHandle,
            request: RenderRequest,
        ) -> Result<RgbaImage, PdfEngineError> {
            self.inner.render_page(handle, request)
        }

        fn render_thumbnail(
            &self,
            handle: DocumentHandle,
            page_index: u32,
            target: ThumbnailSize,
        ) -> Result<RgbaImage, PdfEngineError> {
            self.inner.render_thumbnail(handle, page_index, target)
        }

        fn close(&mut self, handle: DocumentHandle) -> Result<(), PdfEngineError> {
            self.inner.close(handle)
        }
    }
}

pub fn default_engine() -> LopdfEngine {
    LopdfEngine::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pdf_bytes() -> &'static [u8] {
        include_bytes!("../../../tests/fixtures/small.pdf")
    }

    #[test]
    fn opens_pdf_and_reads_page_count() {
        let mut engine = LopdfEngine::new();
        let handle = engine
            .open(OpenSource::Bytes(sample_pdf_bytes().to_vec()))
            .expect("open should succeed");

        assert_eq!(engine.page_count(handle).expect("count should succeed"), 1);
    }

    #[test]
    fn render_thumbnail_produces_non_empty_image() {
        let mut engine = LopdfEngine::new();
        let handle = engine
            .open(OpenSource::Bytes(sample_pdf_bytes().to_vec()))
            .expect("open should succeed");

        let image = engine
            .render_thumbnail(handle, 0, ThumbnailSize { width_px: 80, height_px: 80 })
            .expect("thumbnail should render");

        assert!(image.width() > 0);
        assert!(image.height() > 0);
    }

    #[test]
    fn invalid_handle_returns_error() {
        let engine = LopdfEngine::new();
        let err =
            engine.page_count(DocumentHandle(999)).expect_err("should fail for unknown handle");

        assert!(matches!(err, PdfEngineError::InvalidHandle(999)));
    }
}
