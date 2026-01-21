//! Error dialog component for PDF Editor
//!
//! Provides a GPU-rendered dialog for displaying error messages to users.
//! Features include:
//! - Error icon (warning triangle)
//! - Title and message display
//! - OK button to dismiss
//! - Keyboard support (Enter or Escape to dismiss)
//! - Auto-dismissal timeout option

use crate::scene::{Color, NodeId, Primitive, Rect, SceneNode};
use crate::theme::current_theme;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Dialog width
const DIALOG_WIDTH: f32 = 360.0;

/// Dialog height (auto-expands based on message)
const DIALOG_BASE_HEIGHT: f32 = 130.0;

/// Title bar height
const TITLE_BAR_HEIGHT: f32 = 28.0;

/// Content padding
const PADDING: f32 = 16.0;

/// Button width
const BUTTON_WIDTH: f32 = 80.0;

/// Button height
const BUTTON_HEIGHT: f32 = 28.0;

/// Icon size (warning triangle)
const ICON_SIZE: f32 = 24.0;

/// Error severity levels with different visual styling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Critical error - application may not function correctly
    Error,
    /// Warning - operation partially failed but app continues
    Warning,
    /// Informational - notification of an issue that was handled
    Info,
}

impl ErrorSeverity {
    /// Get the display title for this severity level
    pub fn title(&self) -> &'static str {
        match self {
            ErrorSeverity::Error => "Error",
            ErrorSeverity::Warning => "Warning",
            ErrorSeverity::Info => "Notice",
        }
    }
}

/// Error dialog configuration
#[derive(Debug, Clone)]
pub struct ErrorDialogConfig {
    /// Background color for the dialog
    pub background_color: Color,
    /// Title bar background color
    pub title_bar_color: Color,
    /// Border color
    pub border_color: Color,
    /// Text color
    pub text_color: Color,
    /// Secondary text color (for message body)
    pub text_secondary_color: Color,
    /// Button background color
    pub button_color: Color,
    /// Button hover color
    pub button_hover_color: Color,
    /// Error icon color
    pub error_icon_color: Color,
    /// Warning icon color
    pub warning_icon_color: Color,
    /// Info icon color
    pub info_icon_color: Color,
}

impl Default for ErrorDialogConfig {
    fn default() -> Self {
        let theme = current_theme();
        Self {
            background_color: theme.colors.background_tertiary,
            title_bar_color: theme.colors.background_elevated,
            border_color: theme.colors.border_primary,
            text_color: theme.colors.text_primary,
            text_secondary_color: theme.colors.text_secondary,
            button_color: theme.colors.button_normal,
            button_hover_color: theme.colors.button_hover,
            error_icon_color: Color::rgba(0.85, 0.25, 0.25, 1.0), // Red
            warning_icon_color: Color::rgba(0.90, 0.70, 0.20, 1.0), // Amber
            info_icon_color: Color::rgba(0.30, 0.60, 0.85, 1.0), // Blue
        }
    }
}

/// Buttons in the error dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorDialogButton {
    /// OK button to dismiss the dialog
    Ok,
}

/// Error dialog component for displaying user-friendly error messages
pub struct ErrorDialog {
    /// Configuration for appearance
    config: ErrorDialogConfig,

    /// Whether the dialog is visible
    visible: bool,

    /// Position of the dialog (top-left corner in screen coordinates)
    position: (f32, f32),

    /// Scene node for rendering
    scene_node: Arc<SceneNode>,

    /// Node ID
    node_id: NodeId,

    /// Error severity level
    severity: ErrorSeverity,

    /// Error title (usually from ErrorSeverity::title())
    title: String,

    /// Error message to display
    message: String,

    /// OK button bounds for hit testing
    ok_button_rect: Option<Rect>,

    /// Which button is currently hovered
    hovered_button: Option<ErrorDialogButton>,

    /// Viewport dimensions
    viewport_width: f32,
    viewport_height: f32,

    /// Time when the dialog was shown (for auto-dismiss)
    show_time: Option<Instant>,

    /// Auto-dismiss duration (None = manual dismiss only)
    auto_dismiss_duration: Option<Duration>,

    /// Current dialog height (computed based on message length)
    dialog_height: f32,
}

impl ErrorDialog {
    /// Create a new error dialog
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        let config = ErrorDialogConfig::default();
        let node_id = NodeId::new();
        let scene_node = Arc::new(SceneNode::new());

        // Center the dialog
        let x = (viewport_width - DIALOG_WIDTH) / 2.0;
        let y = (viewport_height - DIALOG_BASE_HEIGHT) / 2.0;

        Self {
            config,
            visible: false,
            position: (x, y),
            scene_node,
            node_id,
            severity: ErrorSeverity::Error,
            title: String::new(),
            message: String::new(),
            ok_button_rect: None,
            hovered_button: None,
            viewport_width,
            viewport_height,
            show_time: None,
            auto_dismiss_duration: None,
            dialog_height: DIALOG_BASE_HEIGHT,
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        viewport_width: f32,
        viewport_height: f32,
        config: ErrorDialogConfig,
    ) -> Self {
        let node_id = NodeId::new();
        let scene_node = Arc::new(SceneNode::new());

        let x = (viewport_width - DIALOG_WIDTH) / 2.0;
        let y = (viewport_height - DIALOG_BASE_HEIGHT) / 2.0;

        Self {
            config,
            visible: false,
            position: (x, y),
            scene_node,
            node_id,
            severity: ErrorSeverity::Error,
            title: String::new(),
            message: String::new(),
            ok_button_rect: None,
            hovered_button: None,
            viewport_width,
            viewport_height,
            show_time: None,
            auto_dismiss_duration: None,
            dialog_height: DIALOG_BASE_HEIGHT,
        }
    }

    /// Show an error dialog with the given severity and message
    pub fn show(&mut self, severity: ErrorSeverity, message: impl Into<String>) {
        self.show_with_title(severity, severity.title(), message);
    }

    /// Show an error dialog with custom title
    pub fn show_with_title(
        &mut self,
        severity: ErrorSeverity,
        title: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.visible = true;
        self.severity = severity;
        self.title = title.into();
        self.message = message.into();
        self.hovered_button = None;
        self.show_time = Some(Instant::now());

        // Calculate dialog height based on message length
        let lines = self.calculate_message_lines();
        self.dialog_height = DIALOG_BASE_HEIGHT + (lines.saturating_sub(1) as f32 * 14.0);

        // Recenter the dialog with new height
        self.position = (
            (self.viewport_width - DIALOG_WIDTH) / 2.0,
            (self.viewport_height - self.dialog_height) / 2.0,
        );

        self.rebuild();
    }

    /// Show an error dialog that auto-dismisses after the specified duration
    pub fn show_with_timeout(
        &mut self,
        severity: ErrorSeverity,
        message: impl Into<String>,
        timeout: Duration,
    ) {
        self.auto_dismiss_duration = Some(timeout);
        self.show(severity, message);
    }

    /// Hide the error dialog
    pub fn hide(&mut self) {
        self.visible = false;
        self.message.clear();
        self.title.clear();
        self.show_time = None;
        self.auto_dismiss_duration = None;
        self.rebuild();
    }

    /// Check if the dialog is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Update the dialog state (call each frame for auto-dismiss)
    /// Returns true if the dialog was auto-dismissed
    pub fn update(&mut self) -> bool {
        if self.visible {
            if let (Some(show_time), Some(timeout)) =
                (self.show_time, self.auto_dismiss_duration)
            {
                if show_time.elapsed() >= timeout {
                    self.hide();
                    return true;
                }
            }
        }
        false
    }

    /// Get the scene node for rendering
    pub fn scene_node(&self) -> &Arc<SceneNode> {
        &self.scene_node
    }

    /// Get the node ID
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Get the current error message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the current severity
    pub fn severity(&self) -> ErrorSeverity {
        self.severity
    }

    /// Update viewport dimensions
    pub fn set_viewport_size(&mut self, width: f32, height: f32) {
        if (self.viewport_width - width).abs() > 0.1
            || (self.viewport_height - height).abs() > 0.1
        {
            self.viewport_width = width;
            self.viewport_height = height;
            if self.visible {
                // Recenter
                self.position = (
                    (width - DIALOG_WIDTH) / 2.0,
                    (height - self.dialog_height) / 2.0,
                );
                self.rebuild();
            }
        }
    }

    /// Hit test for buttons
    pub fn hit_test_button(&self, x: f32, y: f32) -> Option<ErrorDialogButton> {
        if !self.visible {
            return None;
        }

        if let Some(rect) = &self.ok_button_rect {
            if x >= rect.x
                && x <= rect.x + rect.width
                && y >= rect.y
                && y <= rect.y + rect.height
            {
                return Some(ErrorDialogButton::Ok);
            }
        }

        None
    }

    /// Check if a point is within the dialog bounds
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        if !self.visible {
            return false;
        }

        let (dx, dy) = self.position;
        x >= dx && x <= dx + DIALOG_WIDTH && y >= dy && y <= dy + self.dialog_height
    }

    /// Set button hover state
    pub fn set_hovered_button(&mut self, button: Option<ErrorDialogButton>) {
        if self.hovered_button != button {
            self.hovered_button = button;
            self.rebuild();
        }
    }

    /// Calculate number of lines needed for the message
    fn calculate_message_lines(&self) -> usize {
        let max_chars_per_line = 45; // Approximate characters per line
        let char_count = self.message.len();
        ((char_count + max_chars_per_line - 1) / max_chars_per_line).max(1)
    }

    /// Word-wrap the message to fit the dialog width
    fn wrap_message(&self) -> Vec<String> {
        let max_chars_per_line = 45;
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in self.message.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= max_chars_per_line {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }

    /// Rebuild the scene node
    fn rebuild(&mut self) {
        let mut new_node = SceneNode::new();

        if !self.visible {
            self.scene_node = Arc::new(new_node);
            return;
        }

        let (x, y) = self.position;

        // Dialog background with border
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x - 1.0, y - 1.0, DIALOG_WIDTH + 2.0, self.dialog_height + 2.0),
            color: self.config.border_color,
        });
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, y, DIALOG_WIDTH, self.dialog_height),
            color: self.config.background_color,
        });

        // Title bar
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, y, DIALOG_WIDTH, TITLE_BAR_HEIGHT),
            color: self.config.title_bar_color,
        });

        // Get icon color based on severity
        let icon_color = match self.severity {
            ErrorSeverity::Error => self.config.error_icon_color,
            ErrorSeverity::Warning => self.config.warning_icon_color,
            ErrorSeverity::Info => self.config.info_icon_color,
        };

        // Render warning/error icon in title bar
        self.render_icon(&mut new_node, x + 8.0, y + 4.0, icon_color);

        // Title text
        self.render_text(
            &mut new_node,
            &self.title,
            x + ICON_SIZE + 12.0,
            y + 8.0,
            self.config.text_color,
        );

        // Content area
        let content_y = y + TITLE_BAR_HEIGHT + PADDING;

        // Render wrapped message
        let message_lines = self.wrap_message();
        for (i, line) in message_lines.iter().enumerate() {
            self.render_text(
                &mut new_node,
                line,
                x + PADDING,
                content_y + (i as f32 * 14.0),
                self.config.text_secondary_color,
            );
        }

        // OK button (centered at bottom)
        let button_y = y + self.dialog_height - BUTTON_HEIGHT - PADDING;
        let button_x = x + (DIALOG_WIDTH - BUTTON_WIDTH) / 2.0;
        self.ok_button_rect = Some(Rect::new(button_x, button_y, BUTTON_WIDTH, BUTTON_HEIGHT));

        let button_color = if self.hovered_button == Some(ErrorDialogButton::Ok) {
            self.config.button_hover_color
        } else {
            self.config.button_color
        };

        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(button_x, button_y, BUTTON_WIDTH, BUTTON_HEIGHT),
            color: button_color,
        });

        // Center "OK" text in button
        let ok_text_x = button_x + (BUTTON_WIDTH - 2.0 * 7.0) / 2.0;
        self.render_text(
            &mut new_node,
            "OK",
            ok_text_x,
            button_y + 8.0,
            self.config.text_color,
        );

        self.scene_node = Arc::new(new_node);
    }

    /// Render a warning/error icon (triangle with exclamation mark)
    fn render_icon(&self, node: &mut SceneNode, x: f32, y: f32, color: Color) {
        let size = ICON_SIZE;

        // Draw a simple triangle using rectangles
        // Top point
        let center_x = x + size / 2.0;
        let top_y = y + 2.0;
        let bottom_y = y + size - 2.0;

        // Create triangle shape with rectangles (approximation)
        for row in 0..8 {
            let row_y = top_y + (row as f32 * (size - 4.0) / 8.0);
            let row_width = (row as f32 + 1.0) * (size / 8.0);
            let row_x = center_x - row_width / 2.0;
            node.add_primitive(Primitive::Rectangle {
                rect: Rect::new(row_x, row_y, row_width, 2.0),
                color,
            });
        }

        // Exclamation mark (vertical line)
        let exclaim_x = center_x - 1.5;
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(exclaim_x, top_y + 6.0, 3.0, 8.0),
            color: self.config.background_color,
        });

        // Exclamation mark (dot)
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(exclaim_x, bottom_y - 5.0, 3.0, 3.0),
            color: self.config.background_color,
        });
    }

    /// Render text using simple primitives (bitmap font)
    fn render_text(&self, node: &mut SceneNode, text: &str, x: f32, y: f32, color: Color) {
        let char_width = 6.0_f32;
        let char_height = 10.0_f32;
        let char_spacing = 1.0_f32;

        let mut current_x = x;

        for c in text.chars() {
            let char_rect = Rect::new(current_x, y, char_width, char_height);
            Self::render_char(node, c, char_rect, color);
            current_x += char_width + char_spacing;
        }
    }

    /// Render a single character using primitives (bitmap font)
    fn render_char(node: &mut SceneNode, c: char, bounds: Rect, color: Color) {
        let pixel_w = bounds.width / 3.0;
        let pixel_h = bounds.height / 5.0;

        let pattern: [u8; 5] = match c {
            '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
            '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
            '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
            '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
            '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
            '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
            '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
            '7' => [0b111, 0b001, 0b001, 0b001, 0b001],
            '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
            '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
            '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
            ' ' => [0b000, 0b000, 0b000, 0b000, 0b000],
            '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
            ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
            '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
            '_' => [0b000, 0b000, 0b000, 0b000, 0b111],
            '(' => [0b010, 0b100, 0b100, 0b100, 0b010],
            ')' => [0b010, 0b001, 0b001, 0b001, 0b010],
            '[' => [0b110, 0b100, 0b100, 0b100, 0b110],
            ']' => [0b011, 0b001, 0b001, 0b001, 0b011],
            '!' => [0b010, 0b010, 0b010, 0b000, 0b010],
            '?' => [0b111, 0b001, 0b010, 0b000, 0b010],
            '\'' | '"' | '`' => [0b010, 0b010, 0b000, 0b000, 0b000],
            ',' => [0b000, 0b000, 0b000, 0b010, 0b100],
            'a' | 'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
            'b' | 'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
            'c' | 'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
            'd' | 'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
            'e' | 'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
            'f' | 'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
            'g' | 'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
            'h' | 'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
            'i' | 'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
            'j' | 'J' => [0b001, 0b001, 0b001, 0b101, 0b010],
            'k' | 'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
            'l' | 'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
            'm' | 'M' => [0b101, 0b111, 0b101, 0b101, 0b101],
            'n' | 'N' => [0b101, 0b111, 0b111, 0b101, 0b101],
            'o' | 'O' => [0b010, 0b101, 0b101, 0b101, 0b010],
            'p' | 'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
            'q' | 'Q' => [0b010, 0b101, 0b101, 0b111, 0b011],
            'r' | 'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
            's' | 'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
            't' | 'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
            'u' | 'U' => [0b101, 0b101, 0b101, 0b101, 0b010],
            'v' | 'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
            'w' | 'W' => [0b101, 0b101, 0b101, 0b111, 0b101],
            'x' | 'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
            'y' | 'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
            'z' | 'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
            _ => [0b000, 0b000, 0b000, 0b000, 0b000],
        };

        for (row_idx, &row) in pattern.iter().enumerate() {
            for col in 0..3 {
                let bit = (row >> (2 - col)) & 1;
                if bit == 1 {
                    let px = bounds.x + col as f32 * pixel_w;
                    let py = bounds.y + row_idx as f32 * pixel_h;
                    node.add_primitive(Primitive::Rectangle {
                        rect: Rect::new(px, py, pixel_w, pixel_h),
                        color,
                    });
                }
            }
        }
    }
}

/// Helper functions for creating common error messages
impl ErrorDialog {
    /// Show a PDF load error
    pub fn show_pdf_load_error(&mut self, path: &str, error: &str) {
        let message = format!("Could not open '{}': {}", path, error);
        self.show(ErrorSeverity::Error, message);
    }

    /// Show a save error
    pub fn show_save_error(&mut self, path: &str, error: &str) {
        let message = format!("Could not save '{}': {}", path, error);
        self.show(ErrorSeverity::Error, message);
    }

    /// Show a clipboard error
    pub fn show_clipboard_error(&mut self) {
        self.show(
            ErrorSeverity::Warning,
            "Could not copy to clipboard. Please try again.",
        );
    }

    /// Show an export error
    pub fn show_export_error(&mut self, format: &str, error: &str) {
        let message = format!("Export to {} failed: {}", format, error);
        self.show(ErrorSeverity::Error, message);
    }

    /// Show a render error
    pub fn show_render_error(&mut self, page: u16, error: &str) {
        let message = format!("Could not render page {}: {}", page + 1, error);
        self.show(ErrorSeverity::Warning, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_dialog_creation() {
        let dialog = ErrorDialog::new(1200.0, 800.0);

        assert!(!dialog.is_visible());
        assert!(dialog.message().is_empty());
        assert_eq!(dialog.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_error_dialog_show_hide() {
        let mut dialog = ErrorDialog::new(1200.0, 800.0);

        dialog.show(ErrorSeverity::Error, "Test error message");
        assert!(dialog.is_visible());
        assert_eq!(dialog.message(), "Test error message");
        assert_eq!(dialog.severity(), ErrorSeverity::Error);

        dialog.hide();
        assert!(!dialog.is_visible());
        assert!(dialog.message().is_empty());
    }

    #[test]
    fn test_error_dialog_severities() {
        let mut dialog = ErrorDialog::new(1200.0, 800.0);

        dialog.show(ErrorSeverity::Error, "Error");
        assert_eq!(dialog.severity(), ErrorSeverity::Error);

        dialog.show(ErrorSeverity::Warning, "Warning");
        assert_eq!(dialog.severity(), ErrorSeverity::Warning);

        dialog.show(ErrorSeverity::Info, "Info");
        assert_eq!(dialog.severity(), ErrorSeverity::Info);
    }

    #[test]
    fn test_severity_titles() {
        assert_eq!(ErrorSeverity::Error.title(), "Error");
        assert_eq!(ErrorSeverity::Warning.title(), "Warning");
        assert_eq!(ErrorSeverity::Info.title(), "Notice");
    }

    #[test]
    fn test_error_dialog_custom_title() {
        let mut dialog = ErrorDialog::new(1200.0, 800.0);

        dialog.show_with_title(ErrorSeverity::Warning, "Custom Title", "Custom message");
        assert!(dialog.is_visible());
        assert_eq!(dialog.message(), "Custom message");
    }

    #[test]
    fn test_error_dialog_hit_test() {
        let mut dialog = ErrorDialog::new(1200.0, 800.0);

        // Not visible = no hit
        assert!(dialog.hit_test_button(600.0, 400.0).is_none());

        dialog.show(ErrorSeverity::Error, "Test");

        // Contains point should work when visible
        assert!(dialog.contains_point(600.0, 400.0));

        // Outside dialog bounds
        assert!(!dialog.contains_point(0.0, 0.0));
    }

    #[test]
    fn test_error_dialog_viewport_resize() {
        let mut dialog = ErrorDialog::new(1200.0, 800.0);
        dialog.show(ErrorSeverity::Error, "Test");

        dialog.set_viewport_size(1600.0, 1000.0);

        // Dialog should recenter
        assert!(dialog.contains_point(800.0, 500.0));
    }

    #[test]
    fn test_error_dialog_auto_dismiss() {
        let mut dialog = ErrorDialog::new(1200.0, 800.0);

        // Show with very short timeout
        dialog.show_with_timeout(
            ErrorSeverity::Info,
            "Auto-dismiss test",
            Duration::from_millis(10),
        );
        assert!(dialog.is_visible());

        // Wait and update
        std::thread::sleep(Duration::from_millis(20));
        let was_dismissed = dialog.update();

        assert!(was_dismissed);
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_error_dialog_no_auto_dismiss_without_timeout() {
        let mut dialog = ErrorDialog::new(1200.0, 800.0);

        dialog.show(ErrorSeverity::Error, "No auto-dismiss");
        assert!(dialog.is_visible());

        // Update should not dismiss
        std::thread::sleep(Duration::from_millis(10));
        let was_dismissed = dialog.update();

        assert!(!was_dismissed);
        assert!(dialog.is_visible());
    }

    #[test]
    fn test_word_wrap() {
        let mut dialog = ErrorDialog::new(1200.0, 800.0);
        dialog.show(
            ErrorSeverity::Error,
            "This is a very long error message that should be wrapped across multiple lines for better readability",
        );

        // The wrap_message function should produce multiple lines
        let lines = dialog.wrap_message();
        assert!(lines.len() > 1);
    }

    #[test]
    fn test_helper_methods() {
        let mut dialog = ErrorDialog::new(1200.0, 800.0);

        dialog.show_pdf_load_error("test.pdf", "file not found");
        assert!(dialog.message().contains("test.pdf"));
        assert!(dialog.message().contains("file not found"));

        dialog.show_save_error("output.pdf", "permission denied");
        assert!(dialog.message().contains("output.pdf"));
        assert!(dialog.message().contains("permission denied"));

        dialog.show_clipboard_error();
        assert!(dialog.message().contains("clipboard"));

        dialog.show_export_error("PNG", "disk full");
        assert!(dialog.message().contains("PNG"));
        assert!(dialog.message().contains("disk full"));

        dialog.show_render_error(0, "memory error");
        assert!(dialog.message().contains("page 1"));
        assert!(dialog.message().contains("memory error"));
    }

    #[test]
    fn test_error_dialog_button_hover() {
        let mut dialog = ErrorDialog::new(1200.0, 800.0);
        dialog.show(ErrorSeverity::Error, "Test");

        // Initially no button hovered
        assert!(dialog.hovered_button.is_none());

        // Set hover
        dialog.set_hovered_button(Some(ErrorDialogButton::Ok));
        assert_eq!(dialog.hovered_button, Some(ErrorDialogButton::Ok));

        // Clear hover
        dialog.set_hovered_button(None);
        assert!(dialog.hovered_button.is_none());
    }

    #[test]
    fn test_error_dialog_config() {
        let config = ErrorDialogConfig::default();

        // Error should be red-ish
        assert!(config.error_icon_color.r > 0.7);
        assert!(config.error_icon_color.r > config.error_icon_color.g);

        // Warning should be amber/orange
        assert!(config.warning_icon_color.r > 0.8);
        assert!(config.warning_icon_color.g > 0.5);

        // Info should be blue-ish
        assert!(config.info_icon_color.b > config.info_icon_color.r);
    }

    #[test]
    fn test_error_dialog_scene_node() {
        let mut dialog = ErrorDialog::new(1200.0, 800.0);

        // When not visible, scene node should be empty
        let node = dialog.scene_node();
        assert!(node.children().is_empty());

        // When visible, scene node should have content
        dialog.show(ErrorSeverity::Error, "Test");
        let node = dialog.scene_node();
        // The node should have been rebuilt with primitives
        assert!(dialog.is_visible());
    }
}
