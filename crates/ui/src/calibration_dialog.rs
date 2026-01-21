//! Calibration dialog component for PDF Editor
//!
//! Provides a GPU-rendered dialog for scale calibration using two-point method.
//! Features include:
//! - Distance input field
//! - Unit selector (cycling through common units)
//! - OK/Cancel buttons
//! - Keyboard support (Enter to confirm, Escape to cancel)

use crate::scene::{Color, NodeId, Primitive, Rect, SceneNode};
use crate::theme::current_theme;
use std::sync::Arc;

/// Dialog width
const DIALOG_WIDTH: f32 = 280.0;

/// Dialog height
const DIALOG_HEIGHT: f32 = 140.0;

/// Title bar height
const TITLE_BAR_HEIGHT: f32 = 28.0;

/// Content padding
const PADDING: f32 = 12.0;

/// Input field height
const INPUT_HEIGHT: f32 = 24.0;

/// Button width
const BUTTON_WIDTH: f32 = 70.0;

/// Button height
const BUTTON_HEIGHT: f32 = 26.0;

/// Unit button width
const UNIT_BUTTON_WIDTH: f32 = 50.0;

/// Available measurement units for calibration
pub const CALIBRATION_UNITS: [&str; 6] = ["m", "ft", "cm", "mm", "in", "yd"];

/// Configuration for calibration dialog appearance
#[derive(Debug, Clone)]
pub struct CalibrationDialogConfig {
    /// Background color for the dialog
    pub background_color: Color,

    /// Title bar background color
    pub title_bar_color: Color,

    /// Border color
    pub border_color: Color,

    /// Input field background color
    pub input_background_color: Color,

    /// Input field border color
    pub input_border_color: Color,

    /// Input field focused border color
    pub input_focused_border_color: Color,

    /// Text color
    pub text_color: Color,

    /// Button background color
    pub button_color: Color,

    /// Button hover color
    pub button_hover_color: Color,

    /// OK button color
    pub ok_button_color: Color,

    /// Cancel button color
    pub cancel_button_color: Color,
}

impl Default for CalibrationDialogConfig {
    fn default() -> Self {
        let theme = current_theme();
        Self {
            background_color: theme.colors.background_tertiary,
            title_bar_color: theme.colors.background_elevated,
            border_color: theme.colors.border_primary,
            input_background_color: theme.colors.background_input,
            input_border_color: theme.colors.border_primary,
            input_focused_border_color: theme.colors.border_focused,
            text_color: theme.colors.text_primary,
            button_color: theme.colors.button_normal,
            button_hover_color: theme.colors.button_hover,
            ok_button_color: theme.colors.accent_success,
            cancel_button_color: theme.colors.accent_error,
        }
    }
}

/// Buttons in the calibration dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationDialogButton {
    /// OK button to confirm calibration
    Ok,
    /// Cancel button to dismiss dialog
    Cancel,
    /// Unit cycle button to change measurement unit
    UnitCycle,
}

/// Calibration dialog component for entering known distance
pub struct CalibrationDialog {
    /// Configuration for appearance
    config: CalibrationDialogConfig,

    /// Whether the dialog is visible
    visible: bool,

    /// Position of the dialog (top-left corner in screen coordinates)
    position: (f32, f32),

    /// Scene node for rendering
    scene_node: Arc<SceneNode>,

    /// Node ID
    node_id: NodeId,

    /// Current distance input text
    distance_input: String,

    /// Currently selected unit index
    selected_unit_index: usize,

    /// Whether the input field is focused
    input_focused: bool,

    /// OK button bounds for hit testing
    ok_button_rect: Option<Rect>,

    /// Cancel button bounds for hit testing
    cancel_button_rect: Option<Rect>,

    /// Unit button bounds for hit testing
    unit_button_rect: Option<Rect>,

    /// Input field bounds for hit testing
    input_rect: Option<Rect>,

    /// Which button is currently hovered
    hovered_button: Option<CalibrationDialogButton>,

    /// Viewport dimensions
    viewport_width: f32,
    viewport_height: f32,

    /// The distance in page coordinates between calibration points
    /// (shown as reference to the user)
    page_distance: f32,
}

impl CalibrationDialog {
    /// Create a new calibration dialog
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        let config = CalibrationDialogConfig::default();
        let node_id = NodeId::new();
        let scene_node = Arc::new(SceneNode::new());

        // Center the dialog
        let x = (viewport_width - DIALOG_WIDTH) / 2.0;
        let y = (viewport_height - DIALOG_HEIGHT) / 2.0;

        Self {
            config,
            visible: false,
            position: (x, y),
            scene_node,
            node_id,
            distance_input: String::new(),
            selected_unit_index: 0, // Default to meters
            input_focused: true,
            ok_button_rect: None,
            cancel_button_rect: None,
            unit_button_rect: None,
            input_rect: None,
            hovered_button: None,
            viewport_width,
            viewport_height,
            page_distance: 0.0,
        }
    }

    /// Create with custom configuration
    pub fn with_config(viewport_width: f32, viewport_height: f32, config: CalibrationDialogConfig) -> Self {
        let node_id = NodeId::new();
        let scene_node = Arc::new(SceneNode::new());

        let x = (viewport_width - DIALOG_WIDTH) / 2.0;
        let y = (viewport_height - DIALOG_HEIGHT) / 2.0;

        Self {
            config,
            visible: false,
            position: (x, y),
            scene_node,
            node_id,
            distance_input: String::new(),
            selected_unit_index: 0,
            input_focused: true,
            ok_button_rect: None,
            cancel_button_rect: None,
            unit_button_rect: None,
            input_rect: None,
            hovered_button: None,
            viewport_width,
            viewport_height,
            page_distance: 0.0,
        }
    }

    /// Show the calibration dialog
    pub fn show(&mut self, page_distance: f32) {
        self.visible = true;
        self.distance_input.clear();
        self.input_focused = true;
        self.hovered_button = None;
        self.page_distance = page_distance;

        // Recenter the dialog
        self.position = (
            (self.viewport_width - DIALOG_WIDTH) / 2.0,
            (self.viewport_height - DIALOG_HEIGHT) / 2.0,
        );

        self.rebuild();
    }

    /// Hide the calibration dialog
    pub fn hide(&mut self) {
        self.visible = false;
        self.distance_input.clear();
        self.rebuild();
    }

    /// Check if the dialog is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get the scene node for rendering
    pub fn scene_node(&self) -> &Arc<SceneNode> {
        &self.scene_node
    }

    /// Get the node ID
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Get the current distance input
    pub fn distance_input(&self) -> &str {
        &self.distance_input
    }

    /// Get the selected unit
    pub fn selected_unit(&self) -> &str {
        CALIBRATION_UNITS[self.selected_unit_index]
    }

    /// Set the selected unit by name
    pub fn set_unit(&mut self, unit: &str) {
        if let Some(index) = CALIBRATION_UNITS.iter().position(|&u| u == unit) {
            self.selected_unit_index = index;
            self.rebuild();
        }
    }

    /// Cycle to the next unit
    pub fn cycle_unit(&mut self) {
        self.selected_unit_index = (self.selected_unit_index + 1) % CALIBRATION_UNITS.len();
        self.rebuild();
    }

    /// Parse the distance input as a float
    pub fn parse_distance(&self) -> Option<f32> {
        self.distance_input.parse::<f32>().ok().filter(|&d| d > 0.0)
    }

    /// Append a character to the distance input (only allows digits and decimal point)
    pub fn append_char(&mut self, c: char) {
        // Only allow digits and one decimal point
        let should_append = c.is_ascii_digit() || (c == '.' && !self.distance_input.contains('.'));
        if should_append {
            self.distance_input.push(c);
            self.rebuild();
        }
    }

    /// Remove the last character (backspace)
    pub fn backspace(&mut self) {
        if !self.distance_input.is_empty() {
            self.distance_input.pop();
            self.rebuild();
        }
    }

    /// Clear the distance input
    pub fn clear_input(&mut self) {
        self.distance_input.clear();
        self.rebuild();
    }

    /// Update viewport dimensions
    pub fn set_viewport_size(&mut self, width: f32, height: f32) {
        if (self.viewport_width - width).abs() > 0.1 || (self.viewport_height - height).abs() > 0.1 {
            self.viewport_width = width;
            self.viewport_height = height;
            if self.visible {
                // Recenter
                self.position = (
                    (width - DIALOG_WIDTH) / 2.0,
                    (height - DIALOG_HEIGHT) / 2.0,
                );
                self.rebuild();
            }
        }
    }

    /// Hit test for buttons
    pub fn hit_test_button(&self, x: f32, y: f32) -> Option<CalibrationDialogButton> {
        if !self.visible {
            return None;
        }

        if let Some(rect) = &self.ok_button_rect {
            if x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height {
                return Some(CalibrationDialogButton::Ok);
            }
        }

        if let Some(rect) = &self.cancel_button_rect {
            if x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height {
                return Some(CalibrationDialogButton::Cancel);
            }
        }

        if let Some(rect) = &self.unit_button_rect {
            if x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height {
                return Some(CalibrationDialogButton::UnitCycle);
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
        x >= dx && x <= dx + DIALOG_WIDTH && y >= dy && y <= dy + DIALOG_HEIGHT
    }

    /// Check if a point is within the input field
    pub fn hit_test_input(&self, x: f32, y: f32) -> bool {
        if let Some(rect) = &self.input_rect {
            x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height
        } else {
            false
        }
    }

    /// Set button hover state
    pub fn set_hovered_button(&mut self, button: Option<CalibrationDialogButton>) {
        if self.hovered_button != button {
            self.hovered_button = button;
            self.rebuild();
        }
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
            rect: Rect::new(x - 1.0, y - 1.0, DIALOG_WIDTH + 2.0, DIALOG_HEIGHT + 2.0),
            color: self.config.border_color,
        });
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, y, DIALOG_WIDTH, DIALOG_HEIGHT),
            color: self.config.background_color,
        });

        // Title bar
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, y, DIALOG_WIDTH, TITLE_BAR_HEIGHT),
            color: self.config.title_bar_color,
        });

        // Title text
        self.render_text(&mut new_node, "Calibrate Scale", x + PADDING, y + 8.0, self.config.text_color);

        // Content area
        let content_y = y + TITLE_BAR_HEIGHT + PADDING;

        // Label: "Enter known distance:"
        self.render_text(&mut new_node, "Enter known distance:", x + PADDING, content_y, self.config.text_color);

        // Distance input field
        let input_y = content_y + 18.0;
        let input_width = DIALOG_WIDTH - PADDING * 3.0 - UNIT_BUTTON_WIDTH;
        self.input_rect = Some(Rect::new(x + PADDING, input_y, input_width, INPUT_HEIGHT));

        // Input field background
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x + PADDING, input_y, input_width, INPUT_HEIGHT),
            color: self.config.input_background_color,
        });

        // Input field border (highlight if focused)
        let border_color = if self.input_focused {
            self.config.input_focused_border_color
        } else {
            self.config.input_border_color
        };
        // Top border
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x + PADDING, input_y, input_width, 1.0),
            color: border_color,
        });
        // Bottom border
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x + PADDING, input_y + INPUT_HEIGHT - 1.0, input_width, 1.0),
            color: border_color,
        });
        // Left border
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x + PADDING, input_y, 1.0, INPUT_HEIGHT),
            color: border_color,
        });
        // Right border
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x + PADDING + input_width - 1.0, input_y, 1.0, INPUT_HEIGHT),
            color: border_color,
        });

        // Input text or placeholder
        if self.distance_input.is_empty() {
            let placeholder_color = Color::rgba(0.5, 0.5, 0.5, 1.0);
            self.render_text(&mut new_node, "e.g. 10.5", x + PADDING + 4.0, input_y + 6.0, placeholder_color);
        } else {
            self.render_text(&mut new_node, &self.distance_input, x + PADDING + 4.0, input_y + 6.0, self.config.text_color);
        }

        // Cursor if focused and input is active
        if self.input_focused {
            let cursor_x = x + PADDING + 4.0 + self.distance_input.len() as f32 * 7.0;
            new_node.add_primitive(Primitive::Rectangle {
                rect: Rect::new(cursor_x, input_y + 4.0, 1.5, INPUT_HEIGHT - 8.0),
                color: self.config.text_color,
            });
        }

        // Unit button
        let unit_button_x = x + DIALOG_WIDTH - PADDING - UNIT_BUTTON_WIDTH;
        self.unit_button_rect = Some(Rect::new(unit_button_x, input_y, UNIT_BUTTON_WIDTH, INPUT_HEIGHT));

        let unit_button_color = if self.hovered_button == Some(CalibrationDialogButton::UnitCycle) {
            self.config.button_hover_color
        } else {
            self.config.button_color
        };
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(unit_button_x, input_y, UNIT_BUTTON_WIDTH, INPUT_HEIGHT),
            color: unit_button_color,
        });

        // Unit text (centered)
        let unit_text = self.selected_unit();
        let unit_text_x = unit_button_x + (UNIT_BUTTON_WIDTH - unit_text.len() as f32 * 7.0) / 2.0;
        self.render_text(&mut new_node, unit_text, unit_text_x, input_y + 6.0, self.config.text_color);

        // Page distance info (small text)
        let info_y = input_y + INPUT_HEIGHT + 6.0;
        let info_text = format!("Page distance: {:.1} pts", self.page_distance);
        self.render_text(&mut new_node, &info_text, x + PADDING, info_y, Color::rgba(0.6, 0.6, 0.6, 1.0));

        // Buttons row
        let button_y = y + DIALOG_HEIGHT - BUTTON_HEIGHT - PADDING;
        let button_spacing = 10.0;

        // Cancel button
        let cancel_x = x + DIALOG_WIDTH - PADDING - BUTTON_WIDTH;
        self.cancel_button_rect = Some(Rect::new(cancel_x, button_y, BUTTON_WIDTH, BUTTON_HEIGHT));

        let cancel_color = if self.hovered_button == Some(CalibrationDialogButton::Cancel) {
            Color::rgba(
                self.config.cancel_button_color.r * 1.2,
                self.config.cancel_button_color.g * 1.2,
                self.config.cancel_button_color.b * 1.2,
                self.config.cancel_button_color.a,
            )
        } else {
            self.config.cancel_button_color
        };
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(cancel_x, button_y, BUTTON_WIDTH, BUTTON_HEIGHT),
            color: cancel_color,
        });
        self.render_text(&mut new_node, "Cancel", cancel_x + 12.0, button_y + 7.0, self.config.text_color);

        // OK button
        let ok_x = cancel_x - BUTTON_WIDTH - button_spacing;
        self.ok_button_rect = Some(Rect::new(ok_x, button_y, BUTTON_WIDTH, BUTTON_HEIGHT));

        let ok_color = if self.hovered_button == Some(CalibrationDialogButton::Ok) {
            Color::rgba(
                self.config.ok_button_color.r * 1.2,
                self.config.ok_button_color.g * 1.2,
                self.config.ok_button_color.b * 1.2,
                self.config.ok_button_color.a,
            )
        } else {
            self.config.ok_button_color
        };
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(ok_x, button_y, BUTTON_WIDTH, BUTTON_HEIGHT),
            color: ok_color,
        });
        self.render_text(&mut new_node, "OK", ok_x + 26.0, button_y + 7.0, self.config.text_color);

        self.scene_node = Arc::new(new_node);
    }

    /// Render text using simple primitives
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
            '(' => [0b010, 0b100, 0b100, 0b100, 0b010],
            ')' => [0b010, 0b001, 0b001, 0b001, 0b010],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibration_dialog_creation() {
        let dialog = CalibrationDialog::new(1200.0, 800.0);

        assert!(!dialog.is_visible());
        assert!(dialog.distance_input().is_empty());
        assert_eq!(dialog.selected_unit(), "m");
    }

    #[test]
    fn test_calibration_dialog_show_hide() {
        let mut dialog = CalibrationDialog::new(1200.0, 800.0);

        dialog.show(100.0);
        assert!(dialog.is_visible());

        dialog.hide();
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_calibration_dialog_input() {
        let mut dialog = CalibrationDialog::new(1200.0, 800.0);
        dialog.show(100.0);

        dialog.append_char('1');
        dialog.append_char('0');
        dialog.append_char('.');
        dialog.append_char('5');

        assert_eq!(dialog.distance_input(), "10.5");
        assert_eq!(dialog.parse_distance(), Some(10.5));
    }

    #[test]
    fn test_calibration_dialog_input_validation() {
        let mut dialog = CalibrationDialog::new(1200.0, 800.0);
        dialog.show(100.0);

        // Should only allow one decimal point
        dialog.append_char('1');
        dialog.append_char('.');
        dialog.append_char('2');
        dialog.append_char('.'); // Should be ignored
        dialog.append_char('3');

        assert_eq!(dialog.distance_input(), "1.23");

        // Should not allow letters
        dialog.append_char('a');
        assert_eq!(dialog.distance_input(), "1.23");
    }

    #[test]
    fn test_calibration_dialog_backspace() {
        let mut dialog = CalibrationDialog::new(1200.0, 800.0);
        dialog.show(100.0);

        dialog.append_char('1');
        dialog.append_char('2');
        dialog.append_char('3');
        dialog.backspace();

        assert_eq!(dialog.distance_input(), "12");
    }

    #[test]
    fn test_calibration_dialog_unit_cycle() {
        let mut dialog = CalibrationDialog::new(1200.0, 800.0);

        assert_eq!(dialog.selected_unit(), "m");

        dialog.cycle_unit();
        assert_eq!(dialog.selected_unit(), "ft");

        dialog.cycle_unit();
        assert_eq!(dialog.selected_unit(), "cm");

        dialog.cycle_unit();
        assert_eq!(dialog.selected_unit(), "mm");

        dialog.cycle_unit();
        assert_eq!(dialog.selected_unit(), "in");

        dialog.cycle_unit();
        assert_eq!(dialog.selected_unit(), "yd");

        dialog.cycle_unit();
        assert_eq!(dialog.selected_unit(), "m"); // Back to start
    }

    #[test]
    fn test_calibration_dialog_set_unit() {
        let mut dialog = CalibrationDialog::new(1200.0, 800.0);

        dialog.set_unit("ft");
        assert_eq!(dialog.selected_unit(), "ft");

        dialog.set_unit("mm");
        assert_eq!(dialog.selected_unit(), "mm");

        // Invalid unit should not change selection
        dialog.set_unit("invalid");
        assert_eq!(dialog.selected_unit(), "mm");
    }

    #[test]
    fn test_calibration_dialog_parse_distance() {
        let mut dialog = CalibrationDialog::new(1200.0, 800.0);
        dialog.show(100.0);

        // Empty input returns None
        assert_eq!(dialog.parse_distance(), None);

        // Valid number
        dialog.append_char('5');
        assert_eq!(dialog.parse_distance(), Some(5.0));

        // Zero returns None (invalid)
        dialog.clear_input();
        dialog.append_char('0');
        assert_eq!(dialog.parse_distance(), None);
    }

    #[test]
    fn test_calibration_dialog_hit_test() {
        let mut dialog = CalibrationDialog::new(1200.0, 800.0);
        dialog.show(100.0);

        // Not visible = no hit
        dialog.hide();
        assert!(dialog.hit_test_button(600.0, 400.0).is_none());

        dialog.show(100.0);

        // Contains point should work
        assert!(dialog.contains_point(600.0, 400.0));

        // Outside dialog bounds
        assert!(!dialog.contains_point(0.0, 0.0));
    }

    #[test]
    fn test_calibration_dialog_viewport_resize() {
        let mut dialog = CalibrationDialog::new(1200.0, 800.0);
        dialog.show(100.0);

        dialog.set_viewport_size(1600.0, 1000.0);

        // Dialog should recenter
        assert!(dialog.contains_point(800.0, 500.0));
    }
}
