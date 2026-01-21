//! Clipboard module for copying text to system clipboard.
//!
//! Uses the `arboard` crate for cross-platform clipboard access.

use arboard::Clipboard;

/// Error type for clipboard operations.
#[derive(Debug)]
pub enum ClipboardError {
    /// Failed to initialize clipboard access.
    InitializationFailed(String),
    /// Failed to copy text to clipboard.
    CopyFailed(String),
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClipboardError::InitializationFailed(msg) => {
                write!(f, "Failed to initialize clipboard: {}", msg)
            }
            ClipboardError::CopyFailed(msg) => {
                write!(f, "Failed to copy to clipboard: {}", msg)
            }
        }
    }
}

impl std::error::Error for ClipboardError {}

/// Copies the given text to the system clipboard.
///
/// # Arguments
///
/// * `text` - The text to copy to the clipboard.
///
/// # Returns
///
/// Returns `Ok(())` if the text was successfully copied, or a `ClipboardError`
/// if the operation failed.
///
/// # Example
///
/// ```no_run
/// use pdf_editor::clipboard::copy_to_clipboard;
///
/// let text = "Hello, World!";
/// match copy_to_clipboard(text) {
///     Ok(()) => println!("Text copied successfully"),
///     Err(e) => eprintln!("Failed to copy: {}", e),
/// }
/// ```
pub fn copy_to_clipboard(text: &str) -> Result<(), ClipboardError> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| ClipboardError::InitializationFailed(e.to_string()))?;

    clipboard
        .set_text(text)
        .map_err(|e| ClipboardError::CopyFailed(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Clipboard tests that access the system clipboard are marked #[ignore]
    // because they can cause SIGSEGV in certain test environments (especially
    // when running in parallel or in headless CI environments on macOS).
    // Run them manually with: cargo test -p pdf-editor -- --ignored

    #[test]
    #[ignore = "Requires system clipboard access, may crash in CI"]
    fn test_copy_to_clipboard_success() {
        let test_text = "PDF Editor clipboard test - can be safely ignored";

        match copy_to_clipboard(test_text) {
            Ok(()) => {
                // Successfully copied, now verify by reading back
                if let Ok(mut clipboard) = Clipboard::new() {
                    if let Ok(contents) = clipboard.get_text() {
                        assert_eq!(contents, test_text);
                    }
                }
            }
            Err(ClipboardError::InitializationFailed(_)) => {
                // Clipboard not available (headless environment)
            }
            Err(e) => {
                panic!("Unexpected clipboard error: {}", e);
            }
        }
    }

    #[test]
    #[ignore = "Requires system clipboard access, may crash in CI"]
    fn test_copy_empty_string() {
        match copy_to_clipboard("") {
            Ok(()) => {}
            Err(ClipboardError::InitializationFailed(_)) => {}
            Err(e) => {
                panic!("Unexpected error copying empty string: {}", e);
            }
        }
    }

    #[test]
    #[ignore = "Requires system clipboard access, may crash in CI"]
    fn test_copy_unicode_text() {
        let unicode_text = "Unicode test: 日本語 中文 한국어 🎉 émojis";

        match copy_to_clipboard(unicode_text) {
            Ok(()) => {
                if let Ok(mut clipboard) = Clipboard::new() {
                    if let Ok(contents) = clipboard.get_text() {
                        assert_eq!(contents, unicode_text);
                    }
                }
            }
            Err(ClipboardError::InitializationFailed(_)) => {}
            Err(e) => {
                panic!("Unexpected error copying unicode: {}", e);
            }
        }
    }

    #[test]
    fn test_clipboard_error_display() {
        let init_error = ClipboardError::InitializationFailed("test init".to_string());
        assert!(init_error.to_string().contains("initialize"));
        assert!(init_error.to_string().contains("test init"));

        let copy_error = ClipboardError::CopyFailed("test copy".to_string());
        assert!(copy_error.to_string().contains("copy"));
        assert!(copy_error.to_string().contains("test copy"));
    }
}
