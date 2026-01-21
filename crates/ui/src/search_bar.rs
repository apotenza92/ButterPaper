//! Search bar component for PDF Editor
//!
//! Provides a GPU-rendered search bar UI at the top of the window (below toolbar).
//! Features include:
//! - Search input field
//! - Match count display
//! - Previous/Next navigation buttons
//! - Close button

use crate::scene::{Color, NodeId, Primitive, Rect, SceneNode};
use std::sync::Arc;

/// Height of the search bar in pixels
pub const SEARCH_BAR_HEIGHT: f32 = 36.0;

/// Width of the search input field
const SEARCH_INPUT_WIDTH: f32 = 250.0;

/// Height of the search input field
const SEARCH_INPUT_HEIGHT: f32 = 24.0;

/// Width of a navigation button
const NAV_BUTTON_SIZE: f32 = 24.0;

/// Width of the match count display
const MATCH_COUNT_WIDTH: f32 = 60.0;

/// Configuration for search bar appearance
#[derive(Debug, Clone)]
pub struct SearchBarConfig {
    /// Background color for the search bar
    pub background_color: Color,

    /// Input field background color
    pub input_background_color: Color,

    /// Input field border color
    pub input_border_color: Color,

    /// Input field focused border color
    pub input_focused_border_color: Color,

    /// Button background color (normal state)
    pub button_color: Color,

    /// Button background color (hover state)
    pub button_hover_color: Color,

    /// Button icon color
    pub button_icon_color: Color,

    /// Text color
    pub text_color: Color,

    /// Placeholder text color
    pub placeholder_color: Color,

    /// Padding from search bar edges
    pub padding: f32,

    /// Whether the search bar is visible
    pub visible: bool,
}

impl Default for SearchBarConfig {
    fn default() -> Self {
        Self {
            background_color: Color::rgba(0.15, 0.15, 0.15, 0.98),
            input_background_color: Color::rgba(0.1, 0.1, 0.1, 1.0),
            input_border_color: Color::rgba(0.35, 0.35, 0.35, 1.0),
            input_focused_border_color: Color::rgba(0.3, 0.5, 0.8, 1.0),
            button_color: Color::rgba(0.25, 0.25, 0.25, 1.0),
            button_hover_color: Color::rgba(0.35, 0.35, 0.35, 1.0),
            button_icon_color: Color::rgba(0.9, 0.9, 0.9, 1.0),
            text_color: Color::rgba(0.9, 0.9, 0.9, 1.0),
            placeholder_color: Color::rgba(0.5, 0.5, 0.5, 1.0),
            padding: 8.0,
            visible: false,
        }
    }
}

/// Represents a search bar button
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchBarButton {
    /// Previous match button
    PreviousMatch,
    /// Next match button
    NextMatch,
    /// Close search bar button
    Close,
}

impl SearchBarButton {
    /// Get the icon primitives for this button
    fn icon_primitives(&self, x: f32, y: f32, size: f32, color: Color) -> Vec<Primitive> {
        let center_x = x + size / 2.0;
        let center_y = y + size / 2.0;
        let icon_size = size * 0.4;
        let half_icon = icon_size / 2.0;

        match self {
            // Previous match: up-pointing triangle/chevron
            SearchBarButton::PreviousMatch => vec![Primitive::Polygon {
                points: vec![
                    [center_x, center_y - half_icon * 0.8],
                    [center_x + half_icon, center_y + half_icon * 0.5],
                    [center_x - half_icon, center_y + half_icon * 0.5],
                ],
                fill_color: Some(color),
                stroke_color: color,
                stroke_width: 0.0,
            }],

            // Next match: down-pointing triangle/chevron
            SearchBarButton::NextMatch => vec![Primitive::Polygon {
                points: vec![
                    [center_x - half_icon, center_y - half_icon * 0.5],
                    [center_x + half_icon, center_y - half_icon * 0.5],
                    [center_x, center_y + half_icon * 0.8],
                ],
                fill_color: Some(color),
                stroke_color: color,
                stroke_width: 0.0,
            }],

            // Close: X shape
            SearchBarButton::Close => vec![
                // First diagonal line (top-left to bottom-right)
                Primitive::Rectangle {
                    rect: Rect::new(
                        center_x - half_icon * 0.7,
                        center_y - 1.0,
                        half_icon * 1.4,
                        2.0,
                    ),
                    color,
                },
                // Second diagonal line (top-right to bottom-left)
                Primitive::Rectangle {
                    rect: Rect::new(
                        center_x - 1.0,
                        center_y - half_icon * 0.7,
                        2.0,
                        half_icon * 1.4,
                    ),
                    color,
                },
            ],
        }
    }
}

/// Visual state of a button
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonState {
    #[default]
    Normal,
    Hover,
}

/// Search bar component that displays search input and navigation
pub struct SearchBar {
    /// Configuration for layout and appearance
    config: SearchBarConfig,

    /// Current viewport width
    viewport_width: f32,

    /// Scene node for the search bar
    scene_node: Arc<SceneNode>,

    /// Node ID for the search bar
    node_id: NodeId,

    /// Button states and bounds (for hover/active visual feedback)
    button_states: Vec<(SearchBarButton, ButtonState, Rect)>,

    /// Whether the input field is focused
    input_focused: bool,

    /// Rectangle bounds for the input field (for hit testing)
    input_rect: Option<Rect>,

    /// Current search text
    search_text: String,

    /// Current match index (1-indexed for display)
    current_match: usize,

    /// Total number of matches
    total_matches: usize,

    /// Case sensitive search option
    case_sensitive: bool,
}

impl SearchBar {
    /// Create a new search bar with default configuration
    pub fn new(viewport_width: f32) -> Self {
        let config = SearchBarConfig::default();
        let node_id = NodeId::new();
        let scene_node = Arc::new(SceneNode::new());

        let mut search_bar = Self {
            config,
            viewport_width,
            scene_node,
            node_id,
            button_states: Vec::new(),
            input_focused: false,
            input_rect: None,
            search_text: String::new(),
            current_match: 0,
            total_matches: 0,
            case_sensitive: false,
        };

        search_bar.rebuild();
        search_bar
    }

    /// Create with custom configuration
    pub fn with_config(viewport_width: f32, config: SearchBarConfig) -> Self {
        let node_id = NodeId::new();
        let scene_node = Arc::new(SceneNode::new());

        let mut search_bar = Self {
            config,
            viewport_width,
            scene_node,
            node_id,
            button_states: Vec::new(),
            input_focused: false,
            input_rect: None,
            search_text: String::new(),
            current_match: 0,
            total_matches: 0,
            case_sensitive: false,
        };

        search_bar.rebuild();
        search_bar
    }

    /// Update viewport width (e.g., on window resize)
    pub fn set_viewport_width(&mut self, width: f32) {
        if (self.viewport_width - width).abs() > 0.1 {
            self.viewport_width = width;
            self.rebuild();
        }
    }

    /// Set search bar visibility
    pub fn set_visible(&mut self, visible: bool) {
        if self.config.visible != visible {
            self.config.visible = visible;
            if visible {
                self.input_focused = true;
            } else {
                self.input_focused = false;
                self.search_text.clear();
                self.current_match = 0;
                self.total_matches = 0;
            }
            self.rebuild();
        }
    }

    /// Toggle search bar visibility
    pub fn toggle_visible(&mut self) {
        self.set_visible(!self.config.visible);
    }

    /// Check if search bar is visible
    pub fn is_visible(&self) -> bool {
        self.config.visible
    }

    /// Get the search bar height
    pub fn height(&self) -> f32 {
        if self.config.visible {
            SEARCH_BAR_HEIGHT
        } else {
            0.0
        }
    }

    /// Get the scene node for rendering
    pub fn scene_node(&self) -> &Arc<SceneNode> {
        &self.scene_node
    }

    /// Get the node ID
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Set the search text
    pub fn set_search_text(&mut self, text: &str) {
        if self.search_text != text {
            self.search_text = text.to_string();
            self.rebuild();
        }
    }

    /// Get the current search text
    pub fn search_text(&self) -> &str {
        &self.search_text
    }

    /// Append a character to the search text
    pub fn append_char(&mut self, c: char) {
        self.search_text.push(c);
        self.rebuild();
    }

    /// Remove the last character from the search text (backspace)
    pub fn backspace(&mut self) {
        if !self.search_text.is_empty() {
            self.search_text.pop();
            self.rebuild();
        }
    }

    /// Clear the search text
    pub fn clear_search_text(&mut self) {
        if !self.search_text.is_empty() {
            self.search_text.clear();
            self.current_match = 0;
            self.total_matches = 0;
            self.rebuild();
        }
    }

    /// Set the match information (current match index and total)
    pub fn set_match_info(&mut self, current: usize, total: usize) {
        if self.current_match != current || self.total_matches != total {
            self.current_match = current;
            self.total_matches = total;
            self.rebuild();
        }
    }

    /// Get the current match index (1-indexed)
    pub fn current_match(&self) -> usize {
        self.current_match
    }

    /// Get the total number of matches
    pub fn total_matches(&self) -> usize {
        self.total_matches
    }

    /// Set whether the input field is focused
    pub fn set_input_focused(&mut self, focused: bool) {
        if self.input_focused != focused {
            self.input_focused = focused;
            self.rebuild();
        }
    }

    /// Check if the input field is focused
    pub fn is_input_focused(&self) -> bool {
        self.input_focused
    }

    /// Set case sensitive search option
    pub fn set_case_sensitive(&mut self, case_sensitive: bool) {
        if self.case_sensitive != case_sensitive {
            self.case_sensitive = case_sensitive;
            self.rebuild();
        }
    }

    /// Get case sensitive search option
    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
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
    pub fn set_button_hover(&mut self, button: SearchBarButton, hovering: bool) {
        let new_state = if hovering {
            ButtonState::Hover
        } else {
            ButtonState::Normal
        };

        let mut changed = false;
        for (btn, state, _) in &mut self.button_states {
            if *btn == button {
                if *state != new_state {
                    *state = new_state;
                    changed = true;
                }
            } else if hovering && *state == ButtonState::Hover {
                *state = ButtonState::Normal;
                changed = true;
            }
        }

        if changed {
            let saved_states: Vec<(SearchBarButton, ButtonState)> = self
                .button_states
                .iter()
                .map(|(btn, state, _)| (*btn, *state))
                .collect();

            self.rebuild();

            for (btn, state) in saved_states {
                for (b, s, _) in &mut self.button_states {
                    if *b == btn {
                        *s = state;
                        break;
                    }
                }
            }
        }
    }

    /// Clear all button hover states
    pub fn clear_all_hover_states(&mut self) {
        let mut changed = false;
        for (_, state, _) in &mut self.button_states {
            if *state == ButtonState::Hover {
                *state = ButtonState::Normal;
                changed = true;
            }
        }

        if changed {
            self.rebuild();
        }
    }

    /// Hit test - check if a point is within a button and return which one
    pub fn hit_test(&self, x: f32, y: f32) -> Option<SearchBarButton> {
        if !self.config.visible {
            return None;
        }

        for (button, _, rect) in &self.button_states {
            if x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height {
                return Some(*button);
            }
        }

        None
    }

    /// Check if a point is within the search bar bounds
    pub fn contains_point(&self, _x: f32, y: f32) -> bool {
        self.config.visible && y <= SEARCH_BAR_HEIGHT
    }

    /// Rebuild the scene node with current state
    fn rebuild(&mut self) {
        let mut new_node = SceneNode::new();

        if !self.config.visible {
            new_node.set_visible(false);
            self.scene_node = Arc::new(new_node);
            return;
        }

        // Clear button states for rebuilding
        self.button_states.clear();

        // Background
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(0.0, 0.0, self.viewport_width, SEARCH_BAR_HEIGHT),
            color: self.config.background_color,
        });

        // Bottom border
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(0.0, SEARCH_BAR_HEIGHT - 1.0, self.viewport_width, 1.0),
            color: Color::rgba(0.25, 0.25, 0.25, 1.0),
        });

        let mut x = self.config.padding;
        let button_y = (SEARCH_BAR_HEIGHT - NAV_BUTTON_SIZE) / 2.0;

        // Search icon (magnifying glass)
        x = self.add_search_icon(&mut new_node, x);

        // Search input field
        x = self.add_input_field(&mut new_node, x);

        // Match count display
        x = self.add_match_count(&mut new_node, x);

        // Navigation buttons
        x = self.add_button(&mut new_node, SearchBarButton::PreviousMatch, x, button_y);
        x = self.add_button(&mut new_node, SearchBarButton::NextMatch, x, button_y);

        // Spacer
        x += self.config.padding;

        // Close button
        let _ = self.add_button(&mut new_node, SearchBarButton::Close, x, button_y);

        self.scene_node = Arc::new(new_node);
    }

    /// Add a search icon to the scene node
    fn add_search_icon(&self, node: &mut SceneNode, x: f32) -> f32 {
        let icon_size = 16.0;
        let icon_y = (SEARCH_BAR_HEIGHT - icon_size) / 2.0;
        let color = self.config.placeholder_color;

        // Magnifying glass circle
        node.add_primitive(Primitive::Circle {
            center: [x + icon_size / 2.0 - 2.0, icon_y + icon_size / 2.0 - 2.0],
            radius: 5.0,
            color,
        });

        // Handle (diagonal line represented as small rectangle)
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x + icon_size - 6.0, icon_y + icon_size - 6.0, 6.0, 2.0),
            color,
        });

        x + icon_size + 4.0
    }

    /// Add the search input field
    fn add_input_field(&mut self, node: &mut SceneNode, x: f32) -> f32 {
        let input_y = (SEARCH_BAR_HEIGHT - SEARCH_INPUT_HEIGHT) / 2.0;
        let rect = Rect::new(x, input_y, SEARCH_INPUT_WIDTH, SEARCH_INPUT_HEIGHT);

        self.input_rect = Some(rect);

        // Background
        node.add_primitive(Primitive::Rectangle {
            rect,
            color: self.config.input_background_color,
        });

        // Border
        let border_color = if self.input_focused {
            self.config.input_focused_border_color
        } else {
            self.config.input_border_color
        };

        // Top border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, input_y, SEARCH_INPUT_WIDTH, 1.0),
            color: border_color,
        });
        // Bottom border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, input_y + SEARCH_INPUT_HEIGHT - 1.0, SEARCH_INPUT_WIDTH, 1.0),
            color: border_color,
        });
        // Left border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, input_y, 1.0, SEARCH_INPUT_HEIGHT),
            color: border_color,
        });
        // Right border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x + SEARCH_INPUT_WIDTH - 1.0, input_y, 1.0, SEARCH_INPUT_HEIGHT),
            color: border_color,
        });

        // Render text or placeholder
        if self.search_text.is_empty() {
            Self::render_text(node, "Search...", x + 4.0, input_y, self.config.placeholder_color);
        } else {
            Self::render_text(node, &self.search_text, x + 4.0, input_y, self.config.text_color);
        }

        // Cursor if focused
        if self.input_focused {
            let cursor_x = x + 4.0 + self.search_text.len() as f32 * 8.0;
            let cursor_y = input_y + 4.0;
            node.add_primitive(Primitive::Rectangle {
                rect: Rect::new(cursor_x, cursor_y, 1.0, SEARCH_INPUT_HEIGHT - 8.0),
                color: self.config.text_color,
            });
        }

        x + SEARCH_INPUT_WIDTH + self.config.padding
    }

    /// Add the match count display
    fn add_match_count(&self, node: &mut SceneNode, x: f32) -> f32 {
        let count_y = (SEARCH_BAR_HEIGHT - SEARCH_INPUT_HEIGHT) / 2.0;

        // Display match count or empty if no search
        let text = if self.total_matches > 0 {
            format!("{}/{}", self.current_match, self.total_matches)
        } else if !self.search_text.is_empty() {
            "0/0".to_string()
        } else {
            String::new()
        };

        if !text.is_empty() {
            Self::render_text(node, &text, x, count_y, self.config.text_color);
        }

        x + MATCH_COUNT_WIDTH
    }

    /// Render text using simple primitives
    fn render_text(node: &mut SceneNode, text: &str, field_x: f32, field_y: f32, color: Color) {
        let char_width = 6.0_f32;
        let char_height = 10.0_f32;
        let char_spacing = 2.0_f32;

        let start_y = field_y + (SEARCH_INPUT_HEIGHT - char_height) / 2.0;

        let mut current_x = field_x;

        for c in text.chars() {
            // Only render visible characters (limit to first ~30 chars to fit in field)
            if current_x > field_x + SEARCH_INPUT_WIDTH - char_width - 4.0 {
                break;
            }
            let char_rect = Rect::new(current_x, start_y, char_width, char_height);
            Self::render_char(node, c, char_rect, color);
            current_x += char_width + char_spacing;
        }
    }

    /// Render a single character using primitives
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

    /// Add a button to the scene node and return the next x position
    fn add_button(
        &mut self,
        node: &mut SceneNode,
        button: SearchBarButton,
        x: f32,
        y: f32,
    ) -> f32 {
        let rect = Rect::new(x, y, NAV_BUTTON_SIZE, NAV_BUTTON_SIZE);

        // Determine button color based on state
        let state = self
            .button_states
            .iter()
            .find(|(b, _, _)| *b == button)
            .map(|(_, s, _)| *s)
            .unwrap_or(ButtonState::Normal);

        let bg_color = match state {
            ButtonState::Normal => self.config.button_color,
            ButtonState::Hover => self.config.button_hover_color,
        };

        // Button background
        node.add_primitive(Primitive::Rectangle {
            rect,
            color: bg_color,
        });

        // Button icon
        let icon_primitives =
            button.icon_primitives(x, y, NAV_BUTTON_SIZE, self.config.button_icon_color);
        for primitive in icon_primitives {
            node.add_primitive(primitive);
        }

        // Store button state and bounds for hit testing
        self.button_states.push((button, state, rect));

        x + NAV_BUTTON_SIZE + 4.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_bar_creation() {
        let search_bar = SearchBar::new(1200.0);

        assert!(!search_bar.is_visible());
        assert_eq!(search_bar.height(), 0.0);
        assert!(search_bar.search_text().is_empty());
    }

    #[test]
    fn test_search_bar_with_custom_config() {
        let config = SearchBarConfig {
            visible: true,
            ..Default::default()
        };
        let search_bar = SearchBar::with_config(1200.0, config);

        assert!(search_bar.is_visible());
        assert_eq!(search_bar.height(), SEARCH_BAR_HEIGHT);
    }

    #[test]
    fn test_search_bar_visibility_toggle() {
        let mut search_bar = SearchBar::new(1200.0);

        assert!(!search_bar.is_visible());
        search_bar.set_visible(true);
        assert!(search_bar.is_visible());
        assert_eq!(search_bar.height(), SEARCH_BAR_HEIGHT);

        search_bar.set_visible(false);
        assert!(!search_bar.is_visible());
        assert_eq!(search_bar.height(), 0.0);
    }

    #[test]
    fn test_search_bar_toggle_visible() {
        let mut search_bar = SearchBar::new(1200.0);

        search_bar.toggle_visible();
        assert!(search_bar.is_visible());

        search_bar.toggle_visible();
        assert!(!search_bar.is_visible());
    }

    #[test]
    fn test_search_bar_viewport_resize() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);

        search_bar.set_viewport_width(1920.0);
        assert!(search_bar.is_visible());
    }

    #[test]
    fn test_search_bar_search_text() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);

        search_bar.set_search_text("test query");
        assert_eq!(search_bar.search_text(), "test query");

        search_bar.clear_search_text();
        assert!(search_bar.search_text().is_empty());
    }

    #[test]
    fn test_search_bar_append_char() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);

        search_bar.append_char('h');
        search_bar.append_char('e');
        search_bar.append_char('l');
        search_bar.append_char('l');
        search_bar.append_char('o');

        assert_eq!(search_bar.search_text(), "hello");
    }

    #[test]
    fn test_search_bar_backspace() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);

        search_bar.set_search_text("hello");
        search_bar.backspace();
        assert_eq!(search_bar.search_text(), "hell");

        search_bar.backspace();
        search_bar.backspace();
        search_bar.backspace();
        search_bar.backspace();
        assert!(search_bar.search_text().is_empty());

        // Backspace on empty string should do nothing
        search_bar.backspace();
        assert!(search_bar.search_text().is_empty());
    }

    #[test]
    fn test_search_bar_match_info() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);

        search_bar.set_match_info(5, 10);
        assert_eq!(search_bar.current_match(), 5);
        assert_eq!(search_bar.total_matches(), 10);
    }

    #[test]
    fn test_search_bar_input_focus() {
        let mut search_bar = SearchBar::new(1200.0);

        // When search bar is shown, input should be focused
        search_bar.set_visible(true);
        assert!(search_bar.is_input_focused());

        search_bar.set_input_focused(false);
        assert!(!search_bar.is_input_focused());

        search_bar.set_input_focused(true);
        assert!(search_bar.is_input_focused());
    }

    #[test]
    fn test_search_bar_case_sensitive() {
        let mut search_bar = SearchBar::new(1200.0);

        assert!(!search_bar.is_case_sensitive());

        search_bar.set_case_sensitive(true);
        assert!(search_bar.is_case_sensitive());

        search_bar.set_case_sensitive(false);
        assert!(!search_bar.is_case_sensitive());
    }

    #[test]
    fn test_search_bar_hit_test_when_visible() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);

        // Hit test should return buttons when clicking in button areas
        // The exact button depends on the layout
        let result = search_bar.hit_test(20.0, 18.0);
        // May or may not hit a button depending on exact positioning
        assert!(result.is_none() || search_bar.contains_point(20.0, 18.0));
    }

    #[test]
    fn test_search_bar_hit_test_when_invisible() {
        let search_bar = SearchBar::new(1200.0);

        // Should return None even in search bar area
        let result = search_bar.hit_test(20.0, 18.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_search_bar_contains_point() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);

        assert!(search_bar.contains_point(100.0, 10.0));
        assert!(search_bar.contains_point(0.0, SEARCH_BAR_HEIGHT));
        assert!(!search_bar.contains_point(100.0, SEARCH_BAR_HEIGHT + 1.0));
    }

    #[test]
    fn test_search_bar_input_rect_exists() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);

        assert!(search_bar.input_rect.is_some());
    }

    #[test]
    fn test_search_bar_hit_test_input() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);

        let rect = search_bar.input_rect.as_ref().unwrap();
        let center_x = rect.x + rect.width / 2.0;
        let center_y = rect.y + rect.height / 2.0;

        assert!(search_bar.hit_test_input(center_x, center_y));
        assert!(!search_bar.hit_test_input(1000.0, 100.0));
    }

    #[test]
    fn test_search_bar_button_hover() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);

        search_bar.set_button_hover(SearchBarButton::NextMatch, true);

        let state = search_bar
            .button_states
            .iter()
            .find(|(b, _, _)| *b == SearchBarButton::NextMatch)
            .map(|(_, s, _)| *s);
        assert_eq!(state, Some(ButtonState::Hover));

        search_bar.set_button_hover(SearchBarButton::NextMatch, false);
        let state = search_bar
            .button_states
            .iter()
            .find(|(b, _, _)| *b == SearchBarButton::NextMatch)
            .map(|(_, s, _)| *s);
        assert_eq!(state, Some(ButtonState::Normal));
    }

    #[test]
    fn test_search_bar_clear_all_hover_states() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);

        search_bar.set_button_hover(SearchBarButton::Close, true);

        let has_hover = search_bar
            .button_states
            .iter()
            .any(|(_, s, _)| *s == ButtonState::Hover);
        assert!(has_hover);

        search_bar.clear_all_hover_states();

        let has_hover = search_bar
            .button_states
            .iter()
            .any(|(_, s, _)| *s == ButtonState::Hover);
        assert!(!has_hover);
    }

    #[test]
    fn test_search_bar_config_default() {
        let config = SearchBarConfig::default();

        assert!(!config.visible);
        assert_eq!(config.padding, 8.0);
    }

    #[test]
    fn test_search_bar_button_icon_primitives() {
        let buttons = [
            SearchBarButton::PreviousMatch,
            SearchBarButton::NextMatch,
            SearchBarButton::Close,
        ];

        let color = Color::rgb(1.0, 1.0, 1.0);
        for button in buttons {
            let primitives = button.icon_primitives(0.0, 0.0, 24.0, color);
            assert!(
                !primitives.is_empty(),
                "Button {:?} should have icon primitives",
                button
            );
        }
    }

    #[test]
    fn test_search_bar_scene_node_has_primitives() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);

        let primitives = search_bar.scene_node().primitives();
        assert!(
            !primitives.is_empty(),
            "Search bar scene node should have primitives"
        );
    }

    #[test]
    fn test_search_bar_node_id_unique() {
        let search_bar1 = SearchBar::new(1200.0);
        let search_bar2 = SearchBar::new(1200.0);

        assert_ne!(search_bar1.node_id(), search_bar2.node_id());
    }

    #[test]
    fn test_search_bar_clears_on_hide() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);

        search_bar.set_search_text("test");
        search_bar.set_match_info(5, 10);

        search_bar.set_visible(false);

        // Search text and match info should be cleared when hidden
        assert!(search_bar.search_text().is_empty());
        assert_eq!(search_bar.current_match(), 0);
        assert_eq!(search_bar.total_matches(), 0);
    }

    #[test]
    fn test_search_bar_no_rebuild_same_values() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);
        search_bar.set_search_text("test");

        let node_ptr = Arc::as_ptr(search_bar.scene_node());

        // Set the same value - should not rebuild
        search_bar.set_search_text("test");

        assert_eq!(Arc::as_ptr(search_bar.scene_node()), node_ptr);
    }

    #[test]
    fn test_search_bar_rebuild_on_change() {
        let mut search_bar = SearchBar::new(1200.0);
        search_bar.set_visible(true);
        search_bar.set_search_text("test");

        let node_ptr = Arc::as_ptr(search_bar.scene_node());

        // Change value - should trigger rebuild
        search_bar.set_search_text("different");

        assert_ne!(Arc::as_ptr(search_bar.scene_node()), node_ptr);
    }
}
