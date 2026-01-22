//! Minimal layout adjustment for text edits
//!
//! Handles bounding box adjustments and line wrapping when text content changes.
//! Applies minimal changes to preserve the original layout as much as possible.

use crate::text_edit::TextEdit;
use crate::text_layer::TextBoundingBox;

/// Layout adjustment strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutStrategy {
    /// Only adjust width, no line wrapping
    SingleLine,
    /// Allow line wrapping if text exceeds max width
    MultiLine,
}

/// Configuration for layout adjustments
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Strategy for handling text overflow
    pub strategy: LayoutStrategy,

    /// Maximum width before wrapping (in points)
    /// None means no maximum width
    pub max_width: Option<f32>,

    /// Line height multiplier (typically 1.2 for normal spacing)
    pub line_height_multiplier: f32,

    /// Average character width ratio (relative to font size)
    /// Used for estimating text width
    pub char_width_ratio: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            strategy: LayoutStrategy::SingleLine,
            max_width: None,
            line_height_multiplier: 1.2,
            char_width_ratio: 0.6, // Conservative estimate for proportional fonts
        }
    }
}

impl LayoutConfig {
    /// Create config for single-line layout (no wrapping)
    pub fn single_line() -> Self {
        Self {
            strategy: LayoutStrategy::SingleLine,
            ..Default::default()
        }
    }

    /// Create config for multi-line layout with specified max width
    pub fn multi_line(max_width: f32) -> Self {
        Self {
            strategy: LayoutStrategy::MultiLine,
            max_width: Some(max_width),
            ..Default::default()
        }
    }

    /// Create config that inherits max width from original bbox
    pub fn multi_line_preserve_width() -> Self {
        Self {
            strategy: LayoutStrategy::MultiLine,
            max_width: None, // Will use original bbox width
            ..Default::default()
        }
    }
}

/// Result of layout adjustment
#[derive(Debug, Clone)]
pub struct LayoutAdjustment {
    /// New bounding box for the text
    pub new_bbox: TextBoundingBox,

    /// Whether the text was wrapped into multiple lines
    pub is_wrapped: bool,

    /// Number of lines (1 for unwrapped text)
    pub line_count: usize,

    /// Text split into lines (for rendering)
    pub lines: Vec<String>,
}

/// Layout adjuster for text edits
pub struct TextLayoutAdjuster;

impl TextLayoutAdjuster {
    /// Adjust layout for a text edit based on the new text content
    ///
    /// This function calculates the minimal layout changes needed when text is edited.
    /// It preserves the original position and font size, only adjusting the bounding box
    /// and line breaks as necessary.
    pub fn adjust_layout(edit: &TextEdit, config: &LayoutConfig) -> LayoutAdjustment {
        let original_bbox = &edit.bbox;
        let text = &edit.edited_text;
        let font_size = edit.font_size;

        // Estimate character width based on font size
        let avg_char_width = font_size * config.char_width_ratio;

        // Calculate estimated text width
        let estimated_width = text.len() as f32 * avg_char_width;

        // Determine max width for wrapping
        let max_width = match config.strategy {
            LayoutStrategy::SingleLine => None,
            LayoutStrategy::MultiLine => config.max_width.or(Some(original_bbox.width)),
        };

        // Perform layout adjustment
        match (config.strategy, max_width) {
            (LayoutStrategy::SingleLine, _) => {
                // Simple case: just adjust width
                Self::adjust_single_line(original_bbox, estimated_width)
            }
            (LayoutStrategy::MultiLine, Some(max_w)) => {
                // Complex case: potentially wrap lines
                Self::adjust_multi_line(
                    original_bbox,
                    text,
                    font_size,
                    max_w,
                    avg_char_width,
                    config.line_height_multiplier,
                )
            }
            (LayoutStrategy::MultiLine, None) => {
                // No max width specified, treat as single line
                Self::adjust_single_line(original_bbox, estimated_width)
            }
        }
    }

    /// Adjust layout for single-line text (no wrapping)
    fn adjust_single_line(
        original_bbox: &TextBoundingBox,
        estimated_width: f32,
    ) -> LayoutAdjustment {
        // Keep the same position and height, only adjust width
        let new_bbox = TextBoundingBox {
            x: original_bbox.x,
            y: original_bbox.y,
            width: estimated_width.max(1.0), // Ensure minimum width
            height: original_bbox.height,
        };

        LayoutAdjustment {
            new_bbox,
            is_wrapped: false,
            line_count: 1,
            lines: vec![],
        }
    }

    /// Adjust layout for multi-line text with wrapping
    fn adjust_multi_line(
        original_bbox: &TextBoundingBox,
        text: &str,
        font_size: f32,
        max_width: f32,
        avg_char_width: f32,
        line_height_multiplier: f32,
    ) -> LayoutAdjustment {
        // Calculate how many characters fit per line
        let chars_per_line = (max_width / avg_char_width).floor() as usize;

        if chars_per_line == 0 {
            // Max width is too small, fall back to single line
            return Self::adjust_single_line(original_bbox, text.len() as f32 * avg_char_width);
        }

        // Split text into lines with word wrapping
        let lines = Self::wrap_text(text, chars_per_line);
        let line_count = lines.len();

        // Calculate new height based on line count
        let line_height = font_size * line_height_multiplier;
        let total_height = line_height * line_count as f32;

        // Find the longest line to determine width
        let max_line_length = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let new_width = (max_line_length as f32 * avg_char_width).min(max_width);

        let new_bbox = TextBoundingBox {
            x: original_bbox.x,
            y: original_bbox.y,
            width: new_width,
            height: total_height,
        };

        LayoutAdjustment {
            new_bbox,
            is_wrapped: line_count > 1,
            line_count,
            lines,
        }
    }

    /// Wrap text into multiple lines, respecting word boundaries
    ///
    /// This implements a simple greedy word wrapping algorithm that tries to
    /// keep words together when possible.
    fn wrap_text(text: &str, chars_per_line: usize) -> Vec<String> {
        if text.is_empty() {
            return vec![String::new()];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            // Check if adding this word would exceed the line limit
            let would_exceed = if current_line.is_empty() {
                word.len() > chars_per_line
            } else {
                current_line.len() + 1 + word.len() > chars_per_line
            };

            if would_exceed && !current_line.is_empty() {
                // Start a new line
                lines.push(current_line);
                current_line = word.to_string();
            } else if word.len() > chars_per_line {
                // Word is too long, split it
                if !current_line.is_empty() {
                    lines.push(current_line);
                }

                // Split long word across multiple lines
                let mut remaining = word;
                while remaining.len() > chars_per_line {
                    let (chunk, rest) = remaining.split_at(chars_per_line);
                    lines.push(chunk.to_string());
                    remaining = rest;
                }
                current_line = remaining.to_string();
            } else {
                // Add word to current line
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            }
        }

        // Don't forget the last line
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // Handle case where input was only whitespace
        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }

    /// Update a text edit with adjusted layout
    ///
    /// This is a convenience method that applies the layout adjustment
    /// to the text edit's bounding box in place.
    pub fn apply_adjustment(edit: &mut TextEdit, adjustment: LayoutAdjustment) {
        edit.bbox = adjustment.new_bbox;
    }

    /// Calculate layout adjustment and apply it to the edit
    ///
    /// This combines `adjust_layout` and `apply_adjustment` into a single call.
    pub fn adjust_and_apply(edit: &mut TextEdit, config: &LayoutConfig) -> LayoutAdjustment {
        let adjustment = Self::adjust_layout(edit, config);
        Self::apply_adjustment(edit, adjustment.clone());
        adjustment
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_edit(text: &str, bbox_width: f32) -> TextEdit {
        let bbox = TextBoundingBox {
            x: 100.0,
            y: 200.0,
            width: bbox_width,
            height: 20.0,
        };

        TextEdit::new(0, bbox, "".to_string(), text.to_string(), 12.0)
    }

    #[test]
    fn test_single_line_adjustment() {
        let edit = create_test_edit("Short", 100.0);
        let config = LayoutConfig::single_line();

        let adjustment = TextLayoutAdjuster::adjust_layout(&edit, &config);

        assert!(!adjustment.is_wrapped);
        assert_eq!(adjustment.line_count, 1);
        assert_eq!(adjustment.new_bbox.x, 100.0);
        assert_eq!(adjustment.new_bbox.y, 200.0);
        assert_eq!(adjustment.new_bbox.height, 20.0);
        // Width should be estimated based on text length
        assert!(adjustment.new_bbox.width > 0.0);
    }

    #[test]
    fn test_single_line_longer_text() {
        let short_edit = create_test_edit("Hi", 100.0);
        let long_edit = create_test_edit("This is much longer text", 100.0);
        let config = LayoutConfig::single_line();

        let short_adjustment = TextLayoutAdjuster::adjust_layout(&short_edit, &config);
        let long_adjustment = TextLayoutAdjuster::adjust_layout(&long_edit, &config);

        // Longer text should have wider bbox
        assert!(long_adjustment.new_bbox.width > short_adjustment.new_bbox.width);

        // Both should maintain original height and position
        assert_eq!(short_adjustment.new_bbox.height, 20.0);
        assert_eq!(long_adjustment.new_bbox.height, 20.0);
    }

    #[test]
    fn test_multi_line_wrapping() {
        let long_text = "This is a very long line of text that should wrap";
        let edit = create_test_edit(long_text, 100.0);
        let config = LayoutConfig::multi_line(100.0);

        let adjustment = TextLayoutAdjuster::adjust_layout(&edit, &config);

        // Should wrap into multiple lines
        assert!(adjustment.is_wrapped);
        assert!(adjustment.line_count > 1);
        assert!(adjustment.lines.len() > 1);

        // Height should increase with line count
        assert!(adjustment.new_bbox.height > 20.0);

        // Width should not exceed max
        assert!(adjustment.new_bbox.width <= 100.0);
    }

    #[test]
    fn test_multi_line_preserve_width() {
        let long_text = "This is a very long line of text that should wrap to multiple lines";
        let edit = create_test_edit(long_text, 150.0);
        let config = LayoutConfig::multi_line_preserve_width();

        let adjustment = TextLayoutAdjuster::adjust_layout(&edit, &config);

        // Should use original bbox width as max
        assert!(adjustment.new_bbox.width <= 150.0);
        assert!(adjustment.is_wrapped);
    }

    #[test]
    fn test_wrap_text_simple() {
        let lines = TextLayoutAdjuster::wrap_text("Hello world", 6);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Hello");
        assert_eq!(lines[1], "world");
    }

    #[test]
    fn test_wrap_text_long_word() {
        let lines = TextLayoutAdjuster::wrap_text("Supercalifragilisticexpialidocious", 10);
        assert!(lines.len() > 1);
        // Long word should be split
        assert_eq!(lines[0].len(), 10);
    }

    #[test]
    fn test_wrap_text_fits_one_line() {
        let lines = TextLayoutAdjuster::wrap_text("Hello", 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "Hello");
    }

    #[test]
    fn test_wrap_text_empty() {
        let lines = TextLayoutAdjuster::wrap_text("", 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "");
    }

    #[test]
    fn test_wrap_text_whitespace_only() {
        let lines = TextLayoutAdjuster::wrap_text("   ", 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "");
    }

    #[test]
    fn test_apply_adjustment() {
        let mut edit = create_test_edit("Test", 100.0);
        let original_bbox = edit.bbox;

        let config = LayoutConfig::single_line();
        let adjustment = TextLayoutAdjuster::adjust_layout(&edit, &config);

        TextLayoutAdjuster::apply_adjustment(&mut edit, adjustment);

        // Bbox should have changed
        assert_ne!(edit.bbox.width, original_bbox.width);
    }

    #[test]
    fn test_adjust_and_apply() {
        let mut edit = create_test_edit("Test text", 100.0);
        let config = LayoutConfig::single_line();

        let adjustment = TextLayoutAdjuster::adjust_and_apply(&mut edit, &config);

        // Edit bbox should be updated
        assert_eq!(edit.bbox, adjustment.new_bbox);
    }

    #[test]
    fn test_minimum_width() {
        // Even empty text should have some minimum width
        let edit = create_test_edit("", 100.0);
        let config = LayoutConfig::single_line();

        let adjustment = TextLayoutAdjuster::adjust_layout(&edit, &config);

        assert!(adjustment.new_bbox.width >= 1.0);
    }
}
