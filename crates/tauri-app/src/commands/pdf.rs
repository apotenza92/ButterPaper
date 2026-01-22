use crate::state::AppState;
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{ImageBuffer, Rgba};
use pdf_editor_render::PdfDocument;
use serde::Serialize;
use std::io::Cursor;
use std::path::Path;
use tauri::State;

// Response types

#[derive(Serialize)]
pub struct OpenPdfResponse {
    pub success: bool,
    pub page_count: u32,
    pub title: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct RenderResponse {
    pub success: bool,
    pub image_base64: String, // PNG encoded as base64
    pub width: u32,
    pub height: u32,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct NavigateResponse {
    pub success: bool,
    pub current_page: u32,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ZoomResponse {
    pub success: bool,
    pub zoom_percent: u32,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct PageDimensionsResponse {
    pub success: bool,
    pub width: f32,  // PDF points (1/72 inch)
    pub height: f32,
    pub error: Option<String>,
}

// Helper function to convert RGBA bytes to base64-encoded PNG
fn rgba_to_base64_png(rgba: &[u8], width: u32, height: u32) -> Result<String, String> {
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, rgba.to_vec())
        .ok_or_else(|| "Invalid image dimensions".to_string())?;

    let mut png_bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| format!("PNG encoding failed: {}", e))?;

    Ok(STANDARD.encode(&png_bytes))
}

// Helper to get current file path from state
fn get_file_path(state: &State<'_, AppState>) -> Option<String> {
    state.file_path.lock().ok().and_then(|p| p.clone())
}

// Helper to open document from state (returns error message on failure)
fn open_document_from_state(state: &State<'_, AppState>) -> Result<PdfDocument, String> {
    let path = get_file_path(state).ok_or_else(|| "No document open".to_string())?;
    PdfDocument::open(&path).map_err(|e| format!("Failed to open document: {}", e))
}

// IPC Commands

#[tauri::command]
pub fn open_pdf(path: String, state: State<'_, AppState>) -> OpenPdfResponse {
    // Check if file exists
    if !Path::new(&path).exists() {
        return OpenPdfResponse {
            success: false,
            page_count: 0,
            title: None,
            error: Some(format!("File not found: {}", path)),
        };
    }

    // Try to open the PDF
    match PdfDocument::open(&path) {
        Ok(doc) => {
            let page_count = doc.page_count() as u32;
            let title = doc.metadata().title;

            // Store path in state
            if let Ok(mut file_path) = state.file_path.lock() {
                *file_path = Some(path);
            }

            // Cache page count
            if let Ok(mut pc) = state.page_count.lock() {
                *pc = page_count;
            }

            // Cache title
            if let Ok(mut t) = state.document_title.lock() {
                *t = title.clone();
            }

            // Reset current page to 0
            if let Ok(mut current_page) = state.current_page.lock() {
                *current_page = 0;
            }

            OpenPdfResponse {
                success: true,
                page_count,
                title,
                error: None,
            }
        }
        Err(e) => {
            let error_msg = e.to_string();
            // Check if it's a format error
            let error = if error_msg.to_lowercase().contains("load")
                || error_msg.to_lowercase().contains("format")
                || error_msg.to_lowercase().contains("invalid")
            {
                "Invalid PDF format".to_string()
            } else {
                error_msg
            };

            OpenPdfResponse {
                success: false,
                page_count: 0,
                title: None,
                error: Some(error),
            }
        }
    }
}

#[tauri::command]
pub fn render_page(page: u32, width: u32, height: u32, state: State<'_, AppState>) -> RenderResponse {
    // Validate dimensions
    if width == 0 || height == 0 {
        return RenderResponse {
            success: false,
            image_base64: String::new(),
            width: 0,
            height: 0,
            error: Some("Invalid dimensions".to_string()),
        };
    }

    // Open document
    let doc = match open_document_from_state(&state) {
        Ok(d) => d,
        Err(e) => {
            return RenderResponse {
                success: false,
                image_base64: String::new(),
                width: 0,
                height: 0,
                error: Some(e),
            };
        }
    };

    // Validate page index
    let page_count = doc.page_count() as u32;
    if page >= page_count {
        return RenderResponse {
            success: false,
            image_base64: String::new(),
            width: 0,
            height: 0,
            error: Some(format!("Invalid page index: {}", page)),
        };
    }

    // Render the page
    match doc.render_page_rgba(page as u16, width, height) {
        Ok(rgba) => match rgba_to_base64_png(&rgba, width, height) {
            Ok(base64) => RenderResponse {
                success: true,
                image_base64: base64,
                width,
                height,
                error: None,
            },
            Err(e) => RenderResponse {
                success: false,
                image_base64: String::new(),
                width: 0,
                height: 0,
                error: Some(format!("Render failed: {}", e)),
            },
        },
        Err(e) => RenderResponse {
            success: false,
            image_base64: String::new(),
            width: 0,
            height: 0,
            error: Some(format!("Render failed: {}", e)),
        },
    }
}

#[tauri::command]
pub fn render_thumbnail(page: u32, state: State<'_, AppState>) -> RenderResponse {
    // Fixed max dimensions for thumbnails
    const MAX_WIDTH: u32 = 150;
    const MAX_HEIGHT: u32 = 200;

    // Open document
    let doc = match open_document_from_state(&state) {
        Ok(d) => d,
        Err(e) => {
            return RenderResponse {
                success: false,
                image_base64: String::new(),
                width: 0,
                height: 0,
                error: Some(e),
            };
        }
    };

    // Validate page index
    let page_count = doc.page_count() as u32;
    if page >= page_count {
        return RenderResponse {
            success: false,
            image_base64: String::new(),
            width: 0,
            height: 0,
            error: Some(format!("Invalid page index: {}", page)),
        };
    }

    // Render the page scaled to fit within max dimensions
    match doc.render_page_scaled(page as u16, MAX_WIDTH, MAX_HEIGHT) {
        Ok((rgba, actual_width, actual_height)) => {
            match rgba_to_base64_png(&rgba, actual_width, actual_height) {
                Ok(base64) => RenderResponse {
                    success: true,
                    image_base64: base64,
                    width: actual_width,
                    height: actual_height,
                    error: None,
                },
                Err(e) => RenderResponse {
                    success: false,
                    image_base64: String::new(),
                    width: 0,
                    height: 0,
                    error: Some(format!("Render failed: {}", e)),
                },
            }
        }
        Err(e) => RenderResponse {
            success: false,
            image_base64: String::new(),
            width: 0,
            height: 0,
            error: Some(format!("Render failed: {}", e)),
        },
    }
}

#[tauri::command]
pub fn navigate_page(page: u32, state: State<'_, AppState>) -> NavigateResponse {
    // Check if document is open by checking file path
    let file_path = get_file_path(&state);
    if file_path.is_none() {
        return NavigateResponse {
            success: false,
            current_page: 0,
            error: Some("No document open".to_string()),
        };
    }

    // Get cached page count
    let page_count = state.page_count.lock().map(|p| *p).unwrap_or(0);

    // Validate page index
    if page >= page_count {
        // Return current page on failure
        let current = state.current_page.lock().map(|p| *p).unwrap_or(0);
        return NavigateResponse {
            success: false,
            current_page: current,
            error: Some(format!("Page {} out of range (0-{})", page, page_count - 1)),
        };
    }

    // Update current page
    match state.current_page.lock() {
        Ok(mut current_page) => {
            *current_page = page;
            NavigateResponse {
                success: true,
                current_page: page,
                error: None,
            }
        }
        Err(_) => NavigateResponse {
            success: false,
            current_page: 0,
            error: Some("Failed to update page state".to_string()),
        },
    }
}

#[tauri::command]
pub fn set_zoom(percent: u32, state: State<'_, AppState>) -> ZoomResponse {
    // Validate zoom range: 10% to 500%
    if !(10..=500).contains(&percent) {
        let current = state.zoom_percent.lock().map(|z| *z).unwrap_or(100);
        return ZoomResponse {
            success: false,
            zoom_percent: current,
            error: Some("Zoom must be between 10 and 500".to_string()),
        };
    }

    // Update zoom (allowed even without document open - prepares state)
    match state.zoom_percent.lock() {
        Ok(mut zoom) => {
            *zoom = percent;
            ZoomResponse {
                success: true,
                zoom_percent: percent,
                error: None,
            }
        }
        Err(_) => ZoomResponse {
            success: false,
            zoom_percent: 100,
            error: Some("Failed to update zoom state".to_string()),
        },
    }
}

#[tauri::command]
pub fn get_page_dimensions(page: u32, state: State<'_, AppState>) -> PageDimensionsResponse {
    // Open document
    let doc = match open_document_from_state(&state) {
        Ok(d) => d,
        Err(e) => {
            return PageDimensionsResponse {
                success: false,
                width: 0.0,
                height: 0.0,
                error: Some(e),
            };
        }
    };

    // Validate page index
    let page_count = doc.page_count() as u32;
    if page >= page_count {
        return PageDimensionsResponse {
            success: false,
            width: 0.0,
            height: 0.0,
            error: Some(format!("Invalid page index: {}", page)),
        };
    }

    // Get page dimensions
    let result = match doc.get_page(page as u16) {
        Ok(pdf_page) => {
            let width = pdf_page.width().value;
            let height = pdf_page.height().value;
            PageDimensionsResponse {
                success: true,
                width,
                height,
                error: None,
            }
        }
        Err(e) => PageDimensionsResponse {
            success: false,
            width: 0.0,
            height: 0.0,
            error: Some(format!("Failed to get page: {}", e)),
        },
    };
    result
}
