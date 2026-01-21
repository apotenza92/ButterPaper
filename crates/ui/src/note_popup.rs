//! Note popup component for PDF Editor
//!
//! Provides a GPU-rendered popup UI that displays the content of a note annotation
//! when clicked. Features include:
//! - Note content display
//! - Author and timestamp information
//! - Close button

use crate::scene::{Color, NodeId, Primitive, Rect, SceneNode};
use crate::theme::current_theme;
use std::sync::Arc;

/// Default popup width
const POPUP_WIDTH: f32 = 250.0;

/// Minimum popup height
const MIN_POPUP_HEIGHT: f32 = 80.0;

/// Maximum popup height
const MAX_POPUP_HEIGHT: f32 = 300.0;

/// Title bar height
const TITLE_BAR_HEIGHT: f32 = 24.0;

/// Content padding
const PADDING: f32 = 8.0;

/// Close button size
const CLOSE_BUTTON_SIZE: f32 = 16.0;

/// Configuration for note popup appearance
#[derive(Debug, Clone)]
pub struct NotePopupConfig {
    /// Background color for the popup
    pub background_color: Color,

    /// Title bar background color
    pub title_bar_color: Color,

    /// Border color
    pub border_color: Color,

    /// Text color
    pub text_color: Color,

    /// Muted text color (for author/timestamp)
    pub muted_text_color: Color,

    /// Close button color
    pub close_button_color: Color,

    /// Close button hover color
    pub close_button_hover_color: Color,
}

impl Default for NotePopupConfig {
    fn default() -> Self {
        let theme = current_theme();
        Self {
            background_color: theme.colors.note_background,
            title_bar_color: theme.colors.note_title_bar,
            border_color: theme.colors.note_border,
            text_color: theme.colors.note_text,
            muted_text_color: theme.colors.text_disabled,
            close_button_color: Color::rgba(0.6, 0.55, 0.35, 1.0), // Note-specific close button
            close_button_hover_color: Color::rgba(0.8, 0.3, 0.3, 1.0), // Red on hover
        }
    }
}

/// Represents the data for a note to display
#[derive(Debug, Clone)]
pub struct NoteData {
    /// The note content/label
    pub content: String,

    /// Author of the note (optional)
    pub author: Option<String>,

    /// Creation timestamp as formatted string (optional)
    pub created_at: Option<String>,
}

impl NoteData {
    /// Create new note data
    pub fn new(content: String) -> Self {
        Self {
            content,
            author: None,
            created_at: None,
        }
    }

    /// Create note data with author and timestamp
    pub fn with_metadata(content: String, author: Option<String>, created_at: Option<String>) -> Self {
        Self {
            content,
            author,
            created_at,
        }
    }
}

/// Note popup component that displays note annotation content
pub struct NotePopup {
    /// Configuration for appearance
    config: NotePopupConfig,

    /// Whether the popup is visible
    visible: bool,

    /// Position of the popup (top-left corner in screen coordinates)
    position: (f32, f32),

    /// Current note data being displayed
    note_data: Option<NoteData>,

    /// Scene node for rendering
    scene_node: Arc<SceneNode>,

    /// Node ID
    node_id: NodeId,

    /// Close button bounds for hit testing
    close_button_rect: Option<Rect>,

    /// Whether close button is hovered
    close_button_hovered: bool,

    /// Viewport dimensions (for clamping popup position)
    viewport_width: f32,
    viewport_height: f32,
}

impl NotePopup {
    /// Create a new note popup
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        let config = NotePopupConfig::default();
        let node_id = NodeId::new();
        let scene_node = Arc::new(SceneNode::new());

        Self {
            config,
            visible: false,
            position: (0.0, 0.0),
            note_data: None,
            scene_node,
            node_id,
            close_button_rect: None,
            close_button_hovered: false,
            viewport_width,
            viewport_height,
        }
    }

    /// Create with custom configuration
    pub fn with_config(viewport_width: f32, viewport_height: f32, config: NotePopupConfig) -> Self {
        let node_id = NodeId::new();
        let scene_node = Arc::new(SceneNode::new());

        Self {
            config,
            visible: false,
            position: (0.0, 0.0),
            note_data: None,
            scene_node,
            node_id,
            close_button_rect: None,
            close_button_hovered: false,
            viewport_width,
            viewport_height,
        }
    }

    /// Update viewport dimensions
    pub fn set_viewport_dimensions(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
        if self.visible {
            self.rebuild();
        }
    }

    /// Show the popup with note data at a specific position
    pub fn show(&mut self, note_data: NoteData, x: f32, y: f32) {
        self.note_data = Some(note_data);
        self.visible = true;
        self.close_button_hovered = false;

        // Clamp position to keep popup within viewport
        let popup_height = self.calculate_height();
        let clamped_x = x.min(self.viewport_width - POPUP_WIDTH - PADDING);
        let clamped_y = y.min(self.viewport_height - popup_height - PADDING);
        self.position = (clamped_x.max(PADDING), clamped_y.max(PADDING));

        self.rebuild();
    }

    /// Hide the popup
    pub fn hide(&mut self) {
        if self.visible {
            self.visible = false;
            self.note_data = None;
            self.close_button_rect = None;
            self.rebuild();
        }
    }

    /// Check if the popup is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get the popup position
    pub fn position(&self) -> (f32, f32) {
        self.position
    }

    /// Get the scene node for rendering
    pub fn scene_node(&self) -> &Arc<SceneNode> {
        &self.scene_node
    }

    /// Get the node ID
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Get the current note data
    pub fn note_data(&self) -> Option<&NoteData> {
        self.note_data.as_ref()
    }

    /// Check if a point is within the popup bounds
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        if !self.visible {
            return false;
        }

        let popup_height = self.calculate_height();
        x >= self.position.0
            && x <= self.position.0 + POPUP_WIDTH
            && y >= self.position.1
            && y <= self.position.1 + popup_height
    }

    /// Check if a point is on the close button
    pub fn hit_test_close_button(&self, x: f32, y: f32) -> bool {
        if let Some(rect) = &self.close_button_rect {
            x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height
        } else {
            false
        }
    }

    /// Set close button hover state
    pub fn set_close_button_hovered(&mut self, hovered: bool) {
        if self.close_button_hovered != hovered {
            self.close_button_hovered = hovered;
            self.rebuild();
        }
    }

    /// Calculate the height of the popup based on content
    fn calculate_height(&self) -> f32 {
        let Some(note_data) = &self.note_data else {
            return MIN_POPUP_HEIGHT;
        };

        let char_height = 12.0;
        let line_spacing = 4.0;

        // Title bar
        let mut height = TITLE_BAR_HEIGHT;

        // Content area padding
        height += PADDING;

        // Content text (estimate lines based on character count)
        let content_width = POPUP_WIDTH - PADDING * 2.0;
        let chars_per_line = (content_width / 7.0) as usize; // Approximate char width
        let content_lines = if note_data.content.is_empty() {
            1
        } else {
            (note_data.content.len() / chars_per_line.max(1)).max(1)
        };
        height += content_lines as f32 * (char_height + line_spacing);

        // Author line
        if note_data.author.is_some() {
            height += char_height + line_spacing + PADDING;
        }

        // Timestamp line
        if note_data.created_at.is_some() {
            height += char_height + line_spacing;
        }

        // Bottom padding
        height += PADDING;

        height.clamp(MIN_POPUP_HEIGHT, MAX_POPUP_HEIGHT)
    }

    /// Rebuild the scene node
    fn rebuild(&mut self) {
        let mut new_node = SceneNode::new();

        if !self.visible || self.note_data.is_none() {
            new_node.set_visible(false);
            self.scene_node = Arc::new(new_node);
            return;
        }

        let popup_height = self.calculate_height();
        let x = self.position.0;
        let y = self.position.1;

        // Border/shadow (slightly larger rectangle behind)
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x - 1.0, y - 1.0, POPUP_WIDTH + 2.0, popup_height + 2.0),
            color: self.config.border_color,
        });

        // Main background
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, y, POPUP_WIDTH, popup_height),
            color: self.config.background_color,
        });

        // Title bar
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, y, POPUP_WIDTH, TITLE_BAR_HEIGHT),
            color: self.config.title_bar_color,
        });

        // Title bar bottom border
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, y + TITLE_BAR_HEIGHT - 1.0, POPUP_WIDTH, 1.0),
            color: self.config.border_color,
        });

        // Title text "Note"
        self.render_text(&mut new_node, "Note", x + PADDING, y + 4.0, self.config.text_color);

        // Close button
        let close_x = x + POPUP_WIDTH - CLOSE_BUTTON_SIZE - PADDING / 2.0;
        let close_y = y + (TITLE_BAR_HEIGHT - CLOSE_BUTTON_SIZE) / 2.0;
        self.close_button_rect = Some(Rect::new(close_x, close_y, CLOSE_BUTTON_SIZE, CLOSE_BUTTON_SIZE));

        let close_color = if self.close_button_hovered {
            self.config.close_button_hover_color
        } else {
            self.config.close_button_color
        };

        // Close button background (on hover)
        if self.close_button_hovered {
            new_node.add_primitive(Primitive::Rectangle {
                rect: Rect::new(close_x - 2.0, close_y - 2.0, CLOSE_BUTTON_SIZE + 4.0, CLOSE_BUTTON_SIZE + 4.0),
                color: Color::rgba(0.9, 0.2, 0.2, 0.3),
            });
        }

        // Close button X
        let center_x = close_x + CLOSE_BUTTON_SIZE / 2.0;
        let center_y = close_y + CLOSE_BUTTON_SIZE / 2.0;
        let half_size = CLOSE_BUTTON_SIZE / 4.0;

        // First diagonal (top-left to bottom-right, represented as rectangle)
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(center_x - half_size, center_y - 1.0, half_size * 2.0, 2.0),
            color: close_color,
        });
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(center_x - 1.0, center_y - half_size, 2.0, half_size * 2.0),
            color: close_color,
        });

        // Content area
        let content_y = y + TITLE_BAR_HEIGHT + PADDING;
        let mut current_y = content_y;

        if let Some(note_data) = &self.note_data {
            // Content text
            if !note_data.content.is_empty() {
                let lines = self.wrap_text(&note_data.content, POPUP_WIDTH - PADDING * 2.0);
                for line in lines {
                    self.render_text(&mut new_node, &line, x + PADDING, current_y, self.config.text_color);
                    current_y += 14.0;
                }
            } else {
                self.render_text(&mut new_node, "(No content)", x + PADDING, current_y, self.config.muted_text_color);
                current_y += 14.0;
            }

            current_y += PADDING;

            // Author
            if let Some(author) = &note_data.author {
                let author_text = format!("By: {}", author);
                self.render_text(&mut new_node, &author_text, x + PADDING, current_y, self.config.muted_text_color);
                current_y += 14.0;
            }

            // Timestamp
            if let Some(created_at) = &note_data.created_at {
                self.render_text(&mut new_node, created_at, x + PADDING, current_y, self.config.muted_text_color);
            }
        }

        self.scene_node = Arc::new(new_node);
    }

    /// Wrap text to fit within a given width
    fn wrap_text(&self, text: &str, max_width: f32) -> Vec<String> {
        let char_width = 7.0; // Approximate character width
        let chars_per_line = (max_width / char_width) as usize;

        if chars_per_line == 0 {
            return vec![text.to_string()];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= chars_per_line {
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

        // Limit to reasonable number of lines to stay within MAX_POPUP_HEIGHT
        let max_lines = ((MAX_POPUP_HEIGHT - TITLE_BAR_HEIGHT - PADDING * 3.0) / 14.0) as usize;
        if lines.len() > max_lines {
            lines.truncate(max_lines - 1);
            lines.push("...".to_string());
        }

        lines
    }

    /// Render text using simple primitives (similar to SearchBar)
    fn render_text(&self, node: &mut SceneNode, text: &str, x: f32, y: f32, color: Color) {
        let char_width = 6.0_f32;
        let char_height = 10.0_f32;
        let char_spacing = 1.0_f32;

        let mut current_x = x;

        for c in text.chars() {
            // Stop if we're going past the popup width
            if current_x > self.position.0 + POPUP_WIDTH - PADDING {
                break;
            }
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
    fn test_note_popup_creation() {
        let popup = NotePopup::new(1200.0, 800.0);

        assert!(!popup.is_visible());
        assert!(popup.note_data().is_none());
    }

    #[test]
    fn test_note_popup_with_custom_config() {
        let config = NotePopupConfig {
            background_color: Color::rgba(1.0, 0.0, 0.0, 1.0),
            ..Default::default()
        };
        let popup = NotePopup::with_config(1200.0, 800.0, config);

        assert!(!popup.is_visible());
    }

    #[test]
    fn test_note_popup_show() {
        let mut popup = NotePopup::new(1200.0, 800.0);

        let note_data = NoteData::new("Test note content".to_string());
        popup.show(note_data, 100.0, 100.0);

        assert!(popup.is_visible());
        assert!(popup.note_data().is_some());
        assert_eq!(popup.note_data().unwrap().content, "Test note content");
    }

    #[test]
    fn test_note_popup_hide() {
        let mut popup = NotePopup::new(1200.0, 800.0);

        let note_data = NoteData::new("Test".to_string());
        popup.show(note_data, 100.0, 100.0);
        assert!(popup.is_visible());

        popup.hide();
        assert!(!popup.is_visible());
        assert!(popup.note_data().is_none());
    }

    #[test]
    fn test_note_popup_position_clamping() {
        let mut popup = NotePopup::new(300.0, 200.0);

        // Try to show at position that would overflow viewport
        let note_data = NoteData::new("Test".to_string());
        popup.show(note_data, 200.0, 150.0);

        let (x, y) = popup.position();
        // Position should be clamped to keep popup within viewport
        assert!(x + POPUP_WIDTH <= popup.viewport_width);
        assert!(y + popup.calculate_height() <= popup.viewport_height + PADDING);
    }

    #[test]
    fn test_note_popup_contains_point() {
        let mut popup = NotePopup::new(1200.0, 800.0);

        // Not visible - should not contain any point
        assert!(!popup.contains_point(100.0, 100.0));

        let note_data = NoteData::new("Test".to_string());
        popup.show(note_data, 100.0, 100.0);

        // Point inside popup
        assert!(popup.contains_point(150.0, 120.0));

        // Point outside popup
        assert!(!popup.contains_point(0.0, 0.0));
        assert!(!popup.contains_point(500.0, 500.0));
    }

    #[test]
    fn test_note_popup_close_button_hit_test() {
        let mut popup = NotePopup::new(1200.0, 800.0);

        // Not visible - should not hit close button
        assert!(!popup.hit_test_close_button(100.0, 100.0));

        let note_data = NoteData::new("Test".to_string());
        popup.show(note_data, 100.0, 100.0);

        // Hit test in close button area (top-right of popup)
        let close_x = 100.0 + POPUP_WIDTH - CLOSE_BUTTON_SIZE - PADDING / 2.0;
        let close_y = 100.0 + (TITLE_BAR_HEIGHT - CLOSE_BUTTON_SIZE) / 2.0;
        assert!(popup.hit_test_close_button(close_x + CLOSE_BUTTON_SIZE / 2.0, close_y + CLOSE_BUTTON_SIZE / 2.0));

        // Hit test outside close button
        assert!(!popup.hit_test_close_button(100.0, 100.0));
    }

    #[test]
    fn test_note_popup_close_button_hover() {
        let mut popup = NotePopup::new(1200.0, 800.0);

        let note_data = NoteData::new("Test".to_string());
        popup.show(note_data, 100.0, 100.0);

        assert!(!popup.close_button_hovered);

        popup.set_close_button_hovered(true);
        assert!(popup.close_button_hovered);

        popup.set_close_button_hovered(false);
        assert!(!popup.close_button_hovered);
    }

    #[test]
    fn test_note_data_creation() {
        let note = NoteData::new("Content".to_string());
        assert_eq!(note.content, "Content");
        assert!(note.author.is_none());
        assert!(note.created_at.is_none());
    }

    #[test]
    fn test_note_data_with_metadata() {
        let note = NoteData::with_metadata(
            "Content".to_string(),
            Some("John".to_string()),
            Some("2024-01-15".to_string()),
        );
        assert_eq!(note.content, "Content");
        assert_eq!(note.author, Some("John".to_string()));
        assert_eq!(note.created_at, Some("2024-01-15".to_string()));
    }

    #[test]
    fn test_note_popup_viewport_resize() {
        let mut popup = NotePopup::new(1200.0, 800.0);

        popup.set_viewport_dimensions(1920.0, 1080.0);
        assert_eq!(popup.viewport_width, 1920.0);
        assert_eq!(popup.viewport_height, 1080.0);
    }

    #[test]
    fn test_note_popup_scene_node_visible_when_shown() {
        let mut popup = NotePopup::new(1200.0, 800.0);

        let note_data = NoteData::new("Test".to_string());
        popup.show(note_data, 100.0, 100.0);

        let primitives = popup.scene_node().primitives();
        assert!(!primitives.is_empty(), "Scene node should have primitives when visible");
    }

    #[test]
    fn test_note_popup_scene_node_hidden_when_not_visible() {
        let popup = NotePopup::new(1200.0, 800.0);

        let scene_node = popup.scene_node();
        assert!(!scene_node.is_visible() || scene_node.primitives().is_empty());
    }

    #[test]
    fn test_note_popup_node_id_unique() {
        let popup1 = NotePopup::new(1200.0, 800.0);
        let popup2 = NotePopup::new(1200.0, 800.0);

        assert_ne!(popup1.node_id(), popup2.node_id());
    }

    #[test]
    fn test_note_popup_calculate_height() {
        let mut popup = NotePopup::new(1200.0, 800.0);

        // Empty note - should use minimum height
        let note_data = NoteData::new(String::new());
        popup.show(note_data, 0.0, 0.0);
        assert!(popup.calculate_height() >= MIN_POPUP_HEIGHT);

        // Note with content
        let note_data = NoteData::new("Some content here".to_string());
        popup.show(note_data, 0.0, 0.0);
        assert!(popup.calculate_height() >= MIN_POPUP_HEIGHT);

        // Note with metadata
        let note_data = NoteData::with_metadata(
            "Content".to_string(),
            Some("Author".to_string()),
            Some("2024-01-01".to_string()),
        );
        popup.show(note_data, 0.0, 0.0);
        assert!(popup.calculate_height() >= MIN_POPUP_HEIGHT);
    }

    #[test]
    fn test_note_popup_text_wrapping() {
        let popup = NotePopup::new(1200.0, 800.0);

        let long_text = "This is a very long text that should be wrapped across multiple lines when displayed in the popup window";
        let lines = popup.wrap_text(long_text, POPUP_WIDTH - PADDING * 2.0);

        assert!(lines.len() > 1, "Long text should wrap to multiple lines");
    }

    #[test]
    fn test_note_popup_config_default() {
        let config = NotePopupConfig::default();

        // Check that colors are sensible (not all zeros)
        assert!(config.background_color.a > 0.0);
        assert!(config.text_color.a > 0.0);
    }
}
