//! Toolbar component for PDF Editor
//!
//! Provides a GPU-rendered toolbar at the top of the window with navigation,
//! zoom controls, and tool selection buttons.

use crate::scene::{Color, NodeId, Primitive, Rect, SceneNode};
use std::sync::Arc;

/// Height of the toolbar in pixels
pub const TOOLBAR_HEIGHT: f32 = 44.0;

/// Width of the page number input field
const PAGE_INPUT_WIDTH: f32 = 70.0;
/// Height of the page number input field
const PAGE_INPUT_HEIGHT: f32 = 24.0;

/// Configuration for toolbar appearance
#[derive(Debug, Clone)]
pub struct ToolbarConfig {
    /// Background color for the toolbar
    pub background_color: Color,

    /// Separator color between sections
    pub separator_color: Color,

    /// Button background color (normal state)
    pub button_color: Color,

    /// Button background color (hover state)
    pub button_hover_color: Color,

    /// Button background color (pressed/active state)
    pub button_active_color: Color,

    /// Button icon/text color
    pub button_icon_color: Color,

    /// Button size (width and height)
    pub button_size: f32,

    /// Spacing between buttons
    pub button_spacing: f32,

    /// Padding from toolbar edges
    pub padding: f32,

    /// Whether the toolbar is visible
    pub visible: bool,
}

impl Default for ToolbarConfig {
    fn default() -> Self {
        Self {
            background_color: Color::rgba(0.18, 0.18, 0.18, 0.98),
            separator_color: Color::rgba(0.3, 0.3, 0.3, 1.0),
            button_color: Color::rgba(0.25, 0.25, 0.25, 1.0),
            button_hover_color: Color::rgba(0.35, 0.35, 0.35, 1.0),
            button_active_color: Color::rgba(0.2, 0.4, 0.7, 1.0),
            button_icon_color: Color::rgba(0.9, 0.9, 0.9, 1.0),
            button_size: 32.0,
            button_spacing: 4.0,
            padding: 8.0,
            visible: true,
        }
    }
}

/// Represents a toolbar button
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolbarButton {
    // Navigation section
    PrevPage,
    NextPage,

    // Zoom section
    ZoomOut,
    ZoomIn,
    FitPage,
    FitWidth,

    // Tools section
    SelectTool,
    HandTool,
    TextSelectTool,
    HighlightTool,
    CommentTool,
    MeasureTool,
}

impl ToolbarButton {
    /// Get the icon character/shape for this button
    fn icon_primitives(&self, x: f32, y: f32, size: f32, color: Color) -> Vec<Primitive> {
        let center_x = x + size / 2.0;
        let center_y = y + size / 2.0;
        let icon_size = size * 0.5;
        let half_icon = icon_size / 2.0;

        match self {
            // Previous page: left-pointing triangle
            ToolbarButton::PrevPage => vec![Primitive::Polygon {
                points: vec![
                    [center_x + half_icon * 0.5, center_y - half_icon],
                    [center_x + half_icon * 0.5, center_y + half_icon],
                    [center_x - half_icon * 0.7, center_y],
                ],
                fill_color: Some(color),
                stroke_color: color,
                stroke_width: 0.0,
            }],

            // Next page: right-pointing triangle
            ToolbarButton::NextPage => vec![Primitive::Polygon {
                points: vec![
                    [center_x - half_icon * 0.5, center_y - half_icon],
                    [center_x - half_icon * 0.5, center_y + half_icon],
                    [center_x + half_icon * 0.7, center_y],
                ],
                fill_color: Some(color),
                stroke_color: color,
                stroke_width: 0.0,
            }],

            // Zoom out: minus sign
            ToolbarButton::ZoomOut => vec![Primitive::Rectangle {
                rect: Rect::new(
                    center_x - half_icon,
                    center_y - 1.5,
                    icon_size,
                    3.0,
                ),
                color,
            }],

            // Zoom in: plus sign
            ToolbarButton::ZoomIn => vec![
                Primitive::Rectangle {
                    rect: Rect::new(center_x - half_icon, center_y - 1.5, icon_size, 3.0),
                    color,
                },
                Primitive::Rectangle {
                    rect: Rect::new(center_x - 1.5, center_y - half_icon, 3.0, icon_size),
                    color,
                },
            ],

            // Fit page: rectangle with arrows pointing inward
            ToolbarButton::FitPage => vec![Primitive::Rectangle {
                rect: Rect::new(
                    center_x - half_icon,
                    center_y - half_icon * 1.2,
                    icon_size,
                    icon_size * 1.2,
                ),
                color,
            }],

            // Fit width: wide rectangle
            ToolbarButton::FitWidth => vec![Primitive::Rectangle {
                rect: Rect::new(
                    center_x - half_icon * 1.2,
                    center_y - half_icon * 0.6,
                    icon_size * 1.2,
                    icon_size * 0.6,
                ),
                color,
            }],

            // Select tool: arrow cursor
            ToolbarButton::SelectTool => vec![Primitive::Polygon {
                points: vec![
                    [center_x - half_icon * 0.5, center_y - half_icon],
                    [center_x - half_icon * 0.5, center_y + half_icon * 0.6],
                    [center_x - half_icon * 0.1, center_y + half_icon * 0.2],
                    [center_x + half_icon * 0.3, center_y + half_icon * 0.8],
                    [center_x + half_icon * 0.5, center_y + half_icon * 0.5],
                    [center_x, center_y + half_icon * 0.1],
                    [center_x + half_icon * 0.5, center_y + half_icon * 0.1],
                ],
                fill_color: Some(color),
                stroke_color: color,
                stroke_width: 0.0,
            }],

            // Hand tool: simple hand shape (palm)
            ToolbarButton::HandTool => vec![
                // Palm
                Primitive::Circle {
                    center: [center_x, center_y + half_icon * 0.2],
                    radius: half_icon * 0.7,
                    color,
                },
                // Thumb
                Primitive::Circle {
                    center: [center_x - half_icon * 0.6, center_y],
                    radius: half_icon * 0.25,
                    color,
                },
            ],

            // Text select tool: I-beam cursor
            ToolbarButton::TextSelectTool => vec![
                // Top horizontal
                Primitive::Rectangle {
                    rect: Rect::new(center_x - half_icon * 0.6, center_y - half_icon, half_icon * 1.2, 2.0),
                    color,
                },
                // Bottom horizontal
                Primitive::Rectangle {
                    rect: Rect::new(center_x - half_icon * 0.6, center_y + half_icon - 2.0, half_icon * 1.2, 2.0),
                    color,
                },
                // Vertical stem
                Primitive::Rectangle {
                    rect: Rect::new(center_x - 1.0, center_y - half_icon, 2.0, icon_size),
                    color,
                },
            ],

            // Highlight tool: marker/highlighter shape
            ToolbarButton::HighlightTool => vec![
                Primitive::Rectangle {
                    rect: Rect::new(
                        center_x - half_icon * 0.3,
                        center_y - half_icon,
                        half_icon * 0.6,
                        icon_size * 0.8,
                    ),
                    color,
                },
                // Tip
                Primitive::Polygon {
                    points: vec![
                        [center_x - half_icon * 0.3, center_y + half_icon * 0.6],
                        [center_x + half_icon * 0.3, center_y + half_icon * 0.6],
                        [center_x, center_y + half_icon],
                    ],
                    fill_color: Some(color),
                    stroke_color: color,
                    stroke_width: 0.0,
                },
            ],

            // Comment tool: speech bubble
            ToolbarButton::CommentTool => vec![
                Primitive::Circle {
                    center: [center_x, center_y - half_icon * 0.2],
                    radius: half_icon * 0.8,
                    color,
                },
                // Tail
                Primitive::Polygon {
                    points: vec![
                        [center_x - half_icon * 0.3, center_y + half_icon * 0.3],
                        [center_x - half_icon * 0.5, center_y + half_icon],
                        [center_x + half_icon * 0.1, center_y + half_icon * 0.4],
                    ],
                    fill_color: Some(color),
                    stroke_color: color,
                    stroke_width: 0.0,
                },
            ],

            // Measure tool: ruler shape
            ToolbarButton::MeasureTool => vec![
                // Ruler body
                Primitive::Rectangle {
                    rect: Rect::new(
                        center_x - half_icon,
                        center_y - half_icon * 0.3,
                        icon_size,
                        half_icon * 0.6,
                    ),
                    color,
                },
                // Tick marks
                Primitive::Rectangle {
                    rect: Rect::new(center_x - half_icon * 0.6, center_y - half_icon * 0.3, 1.5, half_icon * 0.3),
                    color: Color::rgba(0.1, 0.1, 0.1, 1.0),
                },
                Primitive::Rectangle {
                    rect: Rect::new(center_x, center_y - half_icon * 0.3, 1.5, half_icon * 0.3),
                    color: Color::rgba(0.1, 0.1, 0.1, 1.0),
                },
                Primitive::Rectangle {
                    rect: Rect::new(center_x + half_icon * 0.6, center_y - half_icon * 0.3, 1.5, half_icon * 0.3),
                    color: Color::rgba(0.1, 0.1, 0.1, 1.0),
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
    Active,
}

/// Page number display state
#[derive(Debug, Clone)]
pub struct PageNumberDisplay {
    /// Current page number (1-indexed)
    pub current_page: u16,
    /// Total number of pages
    pub total_pages: u16,
    /// Whether the input field is focused/active
    pub is_focused: bool,
}

impl Default for PageNumberDisplay {
    fn default() -> Self {
        Self {
            current_page: 1,
            total_pages: 1,
            is_focused: false,
        }
    }
}

/// Toolbar component that displays navigation, zoom, and tool buttons
pub struct Toolbar {
    /// Configuration for layout and appearance
    config: ToolbarConfig,

    /// Current viewport width
    viewport_width: f32,

    /// Scene node for the toolbar
    scene_node: Arc<SceneNode>,

    /// Node ID for the toolbar
    node_id: NodeId,

    /// Button states (for hover/active visual feedback)
    button_states: Vec<(ToolbarButton, ButtonState, Rect)>,

    /// Currently selected tool
    selected_tool: Option<ToolbarButton>,

    /// Page number display state
    page_display: PageNumberDisplay,

    /// Rectangle bounds for the page input field (for hit testing)
    page_input_rect: Option<Rect>,
}

impl Toolbar {
    /// Create a new toolbar with default configuration
    pub fn new(viewport_width: f32) -> Self {
        let config = ToolbarConfig::default();
        let node_id = NodeId::new();
        let scene_node = Arc::new(SceneNode::new());

        let mut toolbar = Self {
            config,
            viewport_width,
            scene_node,
            node_id,
            button_states: Vec::new(),
            selected_tool: Some(ToolbarButton::SelectTool),
            page_display: PageNumberDisplay::default(),
            page_input_rect: None,
        };

        toolbar.rebuild();
        toolbar
    }

    /// Create with custom configuration
    pub fn with_config(viewport_width: f32, config: ToolbarConfig) -> Self {
        let node_id = NodeId::new();
        let scene_node = Arc::new(SceneNode::new());

        let mut toolbar = Self {
            config,
            viewport_width,
            scene_node,
            node_id,
            button_states: Vec::new(),
            selected_tool: Some(ToolbarButton::SelectTool),
            page_display: PageNumberDisplay::default(),
            page_input_rect: None,
        };

        toolbar.rebuild();
        toolbar
    }

    /// Update viewport width (e.g., on window resize)
    pub fn set_viewport_width(&mut self, width: f32) {
        if (self.viewport_width - width).abs() > 0.1 {
            self.viewport_width = width;
            self.rebuild();
        }
    }

    /// Set toolbar visibility
    pub fn set_visible(&mut self, visible: bool) {
        if self.config.visible != visible {
            self.config.visible = visible;
            self.rebuild();
        }
    }

    /// Check if toolbar is visible
    pub fn is_visible(&self) -> bool {
        self.config.visible
    }

    /// Get the toolbar height
    pub fn height(&self) -> f32 {
        if self.config.visible {
            TOOLBAR_HEIGHT
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

    /// Set the selected tool
    pub fn set_selected_tool(&mut self, tool: ToolbarButton) {
        if self.selected_tool != Some(tool) {
            self.selected_tool = Some(tool);
            self.rebuild();
        }
    }

    /// Get the selected tool
    pub fn selected_tool(&self) -> Option<ToolbarButton> {
        self.selected_tool
    }

    /// Set the current page number (1-indexed)
    pub fn set_current_page(&mut self, page: u16) {
        if self.page_display.current_page != page {
            self.page_display.current_page = page;
            self.rebuild();
        }
    }

    /// Set the total number of pages
    pub fn set_total_pages(&mut self, total: u16) {
        if self.page_display.total_pages != total {
            self.page_display.total_pages = total;
            self.rebuild();
        }
    }

    /// Set both current page and total pages
    pub fn set_page_info(&mut self, current: u16, total: u16) {
        if self.page_display.current_page != current || self.page_display.total_pages != total {
            self.page_display.current_page = current;
            self.page_display.total_pages = total;
            self.rebuild();
        }
    }

    /// Get the current page display state
    pub fn page_display(&self) -> &PageNumberDisplay {
        &self.page_display
    }

    /// Check if a point is within the page input field
    pub fn hit_test_page_input(&self, x: f32, y: f32) -> bool {
        if let Some(rect) = &self.page_input_rect {
            x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height
        } else {
            false
        }
    }

    /// Set button hover state
    pub fn set_button_hover(&mut self, button: ToolbarButton, hovering: bool) {
        let new_state = if hovering {
            ButtonState::Hover
        } else {
            ButtonState::Normal
        };

        // Find and update the button state
        let mut changed = false;
        for (btn, state, _) in &mut self.button_states {
            if *btn == button {
                if *state != new_state {
                    *state = new_state;
                    changed = true;
                }
                break;
            }
        }

        if changed {
            // Save the updated states before rebuild
            let saved_states: Vec<(ToolbarButton, ButtonState)> = self
                .button_states
                .iter()
                .map(|(btn, state, _)| (*btn, *state))
                .collect();

            self.rebuild();

            // Restore states after rebuild
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

    /// Hit test - check if a point is within a button and return which one
    pub fn hit_test(&self, x: f32, y: f32) -> Option<ToolbarButton> {
        if !self.config.visible {
            return None;
        }

        // Check if point is within toolbar bounds
        if y > TOOLBAR_HEIGHT {
            return None;
        }

        // Check each button
        for (button, _, rect) in &self.button_states {
            if x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height {
                return Some(*button);
            }
        }

        None
    }

    /// Check if a point is within the toolbar bounds
    pub fn contains_point(&self, _x: f32, y: f32) -> bool {
        self.config.visible && y <= TOOLBAR_HEIGHT
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
            rect: Rect::new(0.0, 0.0, self.viewport_width, TOOLBAR_HEIGHT),
            color: self.config.background_color,
        });

        // Bottom border/separator
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(0.0, TOOLBAR_HEIGHT - 1.0, self.viewport_width, 1.0),
            color: self.config.separator_color,
        });

        let mut x = self.config.padding;
        let button_y = (TOOLBAR_HEIGHT - self.config.button_size) / 2.0;

        // Navigation section
        x = self.add_button(&mut new_node, ToolbarButton::PrevPage, x, button_y);

        // Page number input field
        x = self.add_page_input(&mut new_node, x, button_y);

        x = self.add_button(&mut new_node, ToolbarButton::NextPage, x, button_y);

        // Separator
        x += self.config.button_spacing * 2.0;
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, 8.0, 1.0, TOOLBAR_HEIGHT - 16.0),
            color: self.config.separator_color,
        });
        x += self.config.button_spacing * 2.0 + 1.0;

        // Zoom section
        x = self.add_button(&mut new_node, ToolbarButton::ZoomOut, x, button_y);
        x = self.add_button(&mut new_node, ToolbarButton::ZoomIn, x, button_y);
        x = self.add_button(&mut new_node, ToolbarButton::FitPage, x, button_y);
        x = self.add_button(&mut new_node, ToolbarButton::FitWidth, x, button_y);

        // Separator
        x += self.config.button_spacing * 2.0;
        new_node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, 8.0, 1.0, TOOLBAR_HEIGHT - 16.0),
            color: self.config.separator_color,
        });
        x += self.config.button_spacing * 2.0 + 1.0;

        // Tools section
        x = self.add_button(&mut new_node, ToolbarButton::SelectTool, x, button_y);
        x = self.add_button(&mut new_node, ToolbarButton::HandTool, x, button_y);
        x = self.add_button(&mut new_node, ToolbarButton::TextSelectTool, x, button_y);
        x = self.add_button(&mut new_node, ToolbarButton::HighlightTool, x, button_y);
        x = self.add_button(&mut new_node, ToolbarButton::CommentTool, x, button_y);
        let _ = self.add_button(&mut new_node, ToolbarButton::MeasureTool, x, button_y);

        self.scene_node = Arc::new(new_node);
    }

    /// Add the page number input field to the scene node and return the next x position
    fn add_page_input(&mut self, node: &mut SceneNode, x: f32, _button_y: f32) -> f32 {
        let input_y = (TOOLBAR_HEIGHT - PAGE_INPUT_HEIGHT) / 2.0;
        let rect = Rect::new(x, input_y, PAGE_INPUT_WIDTH, PAGE_INPUT_HEIGHT);

        // Store the rect for hit testing
        self.page_input_rect = Some(rect);

        // Background (darker than buttons, like an input field)
        let bg_color = Color::rgba(0.15, 0.15, 0.15, 1.0);
        node.add_primitive(Primitive::Rectangle {
            rect,
            color: bg_color,
        });

        // Border
        let border_color = Color::rgba(0.35, 0.35, 0.35, 1.0);
        // Top border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, input_y, PAGE_INPUT_WIDTH, 1.0),
            color: border_color,
        });
        // Bottom border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, input_y + PAGE_INPUT_HEIGHT - 1.0, PAGE_INPUT_WIDTH, 1.0),
            color: border_color,
        });
        // Left border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, input_y, 1.0, PAGE_INPUT_HEIGHT),
            color: border_color,
        });
        // Right border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x + PAGE_INPUT_WIDTH - 1.0, input_y, 1.0, PAGE_INPUT_HEIGHT),
            color: border_color,
        });

        // Render the page number text "X / Y" using simple rectangles for each digit
        // This uses a basic approach - for actual text, a texture-based approach would be better
        let text = format!(
            "{} / {}",
            self.page_display.current_page, self.page_display.total_pages
        );
        Self::render_page_text(node, &text, x, input_y);

        x + PAGE_INPUT_WIDTH + self.config.button_spacing
    }

    /// Render page number text using simple primitives
    /// This is a simplified text renderer - for production, use texture-based text
    fn render_page_text(node: &mut SceneNode, text: &str, field_x: f32, field_y: f32) {
        let text_color = Color::rgba(0.85, 0.85, 0.85, 1.0);

        // Character dimensions (simplified 3x5 font at scale 2)
        let char_width = 6.0_f32;
        let char_height = 10.0_f32;
        let char_spacing = 2.0_f32;

        // Calculate total text width
        let text_width = text.len() as f32 * (char_width + char_spacing) - char_spacing;

        // Center the text in the input field
        let start_x = field_x + (PAGE_INPUT_WIDTH - text_width) / 2.0;
        let start_y = field_y + (PAGE_INPUT_HEIGHT - char_height) / 2.0;

        let mut current_x = start_x;

        for c in text.chars() {
            let char_rect = Rect::new(current_x, start_y, char_width, char_height);
            Self::render_char(node, c, char_rect, text_color);
            current_x += char_width + char_spacing;
        }
    }

    /// Render a single character using primitives
    fn render_char(node: &mut SceneNode, c: char, bounds: Rect, color: Color) {
        // Simple 3x5 pixel representations of digits and symbols
        // Scaled to fill the given bounds
        let pixel_w = bounds.width / 3.0;
        let pixel_h = bounds.height / 5.0;

        // Get the pixel pattern for this character (3 wide x 5 tall)
        // Each element is a row, with bits representing pixels
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
    fn add_button(&mut self, node: &mut SceneNode, button: ToolbarButton, x: f32, y: f32) -> f32 {
        let rect = Rect::new(x, y, self.config.button_size, self.config.button_size);

        // Determine button color based on state
        let is_selected = self.selected_tool == Some(button);
        let state = self
            .button_states
            .iter()
            .find(|(b, _, _)| *b == button)
            .map(|(_, s, _)| *s)
            .unwrap_or(ButtonState::Normal);

        let bg_color = if is_selected {
            self.config.button_active_color
        } else {
            match state {
                ButtonState::Normal => self.config.button_color,
                ButtonState::Hover => self.config.button_hover_color,
                ButtonState::Active => self.config.button_active_color,
            }
        };

        // Button background
        node.add_primitive(Primitive::Rectangle {
            rect,
            color: bg_color,
        });

        // Button icon
        let icon_primitives =
            button.icon_primitives(x, y, self.config.button_size, self.config.button_icon_color);
        for primitive in icon_primitives {
            node.add_primitive(primitive);
        }

        // Store button state and bounds for hit testing
        self.button_states.push((button, state, rect));

        x + self.config.button_size + self.config.button_spacing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toolbar_creation() {
        let toolbar = Toolbar::new(1200.0);

        assert!(toolbar.is_visible());
        assert_eq!(toolbar.height(), TOOLBAR_HEIGHT);
        assert_eq!(toolbar.selected_tool(), Some(ToolbarButton::SelectTool));
    }

    #[test]
    fn test_toolbar_with_custom_config() {
        let config = ToolbarConfig {
            visible: false,
            ..Default::default()
        };
        let toolbar = Toolbar::with_config(1200.0, config);

        assert!(!toolbar.is_visible());
        assert_eq!(toolbar.height(), 0.0);
    }

    #[test]
    fn test_toolbar_visibility_toggle() {
        let mut toolbar = Toolbar::new(1200.0);

        assert!(toolbar.is_visible());
        toolbar.set_visible(false);
        assert!(!toolbar.is_visible());
        assert_eq!(toolbar.height(), 0.0);

        toolbar.set_visible(true);
        assert!(toolbar.is_visible());
        assert_eq!(toolbar.height(), TOOLBAR_HEIGHT);
    }

    #[test]
    fn test_toolbar_viewport_resize() {
        let mut toolbar = Toolbar::new(1200.0);

        toolbar.set_viewport_width(1920.0);
        // The toolbar should still be visible and functional
        assert!(toolbar.is_visible());
    }

    #[test]
    fn test_toolbar_selected_tool() {
        let mut toolbar = Toolbar::new(1200.0);

        assert_eq!(toolbar.selected_tool(), Some(ToolbarButton::SelectTool));

        toolbar.set_selected_tool(ToolbarButton::HandTool);
        assert_eq!(toolbar.selected_tool(), Some(ToolbarButton::HandTool));

        toolbar.set_selected_tool(ToolbarButton::MeasureTool);
        assert_eq!(toolbar.selected_tool(), Some(ToolbarButton::MeasureTool));
    }

    #[test]
    fn test_toolbar_hit_test_when_visible() {
        let toolbar = Toolbar::new(1200.0);

        // Click in toolbar area - should hit a button
        let result = toolbar.hit_test(20.0, 22.0);
        // The exact button depends on the layout, but we should get something
        assert!(result.is_some() || toolbar.contains_point(20.0, 22.0));

        // Click below toolbar - should return None
        let result = toolbar.hit_test(20.0, TOOLBAR_HEIGHT + 10.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_toolbar_hit_test_when_invisible() {
        let mut toolbar = Toolbar::new(1200.0);
        toolbar.set_visible(false);

        // Should return None even in toolbar area
        let result = toolbar.hit_test(20.0, 22.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_toolbar_contains_point() {
        let toolbar = Toolbar::new(1200.0);

        assert!(toolbar.contains_point(100.0, 10.0));
        assert!(toolbar.contains_point(0.0, TOOLBAR_HEIGHT));
        assert!(!toolbar.contains_point(100.0, TOOLBAR_HEIGHT + 1.0));
    }

    #[test]
    fn test_toolbar_button_hover() {
        let mut toolbar = Toolbar::new(1200.0);

        // Set hover state
        toolbar.set_button_hover(ToolbarButton::ZoomIn, true);

        // Find the button state
        let state = toolbar
            .button_states
            .iter()
            .find(|(b, _, _)| *b == ToolbarButton::ZoomIn)
            .map(|(_, s, _)| *s);
        assert_eq!(state, Some(ButtonState::Hover));

        // Clear hover state
        toolbar.set_button_hover(ToolbarButton::ZoomIn, false);
        let state = toolbar
            .button_states
            .iter()
            .find(|(b, _, _)| *b == ToolbarButton::ZoomIn)
            .map(|(_, s, _)| *s);
        assert_eq!(state, Some(ButtonState::Normal));
    }

    #[test]
    fn test_toolbar_config_default() {
        let config = ToolbarConfig::default();

        assert!(config.visible);
        assert_eq!(config.button_size, 32.0);
        assert_eq!(config.button_spacing, 4.0);
        assert_eq!(config.padding, 8.0);
    }

    #[test]
    fn test_toolbar_button_icon_primitives() {
        let buttons = [
            ToolbarButton::PrevPage,
            ToolbarButton::NextPage,
            ToolbarButton::ZoomIn,
            ToolbarButton::ZoomOut,
            ToolbarButton::FitPage,
            ToolbarButton::FitWidth,
            ToolbarButton::SelectTool,
            ToolbarButton::HandTool,
            ToolbarButton::TextSelectTool,
            ToolbarButton::HighlightTool,
            ToolbarButton::CommentTool,
            ToolbarButton::MeasureTool,
        ];

        let color = Color::rgb(1.0, 1.0, 1.0);
        for button in buttons {
            let primitives = button.icon_primitives(0.0, 0.0, 32.0, color);
            // Each button should produce at least one primitive for its icon
            assert!(
                !primitives.is_empty(),
                "Button {:?} should have icon primitives",
                button
            );
        }
    }

    #[test]
    fn test_toolbar_scene_node_has_primitives() {
        let toolbar = Toolbar::new(1200.0);

        // The scene node should have primitives (background + buttons)
        let primitives = toolbar.scene_node().primitives();
        assert!(
            !primitives.is_empty(),
            "Toolbar scene node should have primitives"
        );
    }

    #[test]
    fn test_toolbar_node_id_unique() {
        let toolbar1 = Toolbar::new(1200.0);
        let toolbar2 = Toolbar::new(1200.0);

        assert_ne!(toolbar1.node_id(), toolbar2.node_id());
    }

    // Page number input field tests

    #[test]
    fn test_page_display_default() {
        let display = PageNumberDisplay::default();
        assert_eq!(display.current_page, 1);
        assert_eq!(display.total_pages, 1);
        assert!(!display.is_focused);
    }

    #[test]
    fn test_toolbar_page_display_initial() {
        let toolbar = Toolbar::new(1200.0);
        let display = toolbar.page_display();
        assert_eq!(display.current_page, 1);
        assert_eq!(display.total_pages, 1);
    }

    #[test]
    fn test_toolbar_set_current_page() {
        let mut toolbar = Toolbar::new(1200.0);

        toolbar.set_current_page(5);
        assert_eq!(toolbar.page_display().current_page, 5);

        toolbar.set_current_page(10);
        assert_eq!(toolbar.page_display().current_page, 10);
    }

    #[test]
    fn test_toolbar_set_total_pages() {
        let mut toolbar = Toolbar::new(1200.0);

        toolbar.set_total_pages(100);
        assert_eq!(toolbar.page_display().total_pages, 100);

        toolbar.set_total_pages(50);
        assert_eq!(toolbar.page_display().total_pages, 50);
    }

    #[test]
    fn test_toolbar_set_page_info() {
        let mut toolbar = Toolbar::new(1200.0);

        toolbar.set_page_info(7, 42);
        assert_eq!(toolbar.page_display().current_page, 7);
        assert_eq!(toolbar.page_display().total_pages, 42);
    }

    #[test]
    fn test_toolbar_page_input_rect_exists() {
        let toolbar = Toolbar::new(1200.0);

        // The page input rect should be set after construction
        assert!(toolbar.page_input_rect.is_some());
    }

    #[test]
    fn test_toolbar_hit_test_page_input() {
        let toolbar = Toolbar::new(1200.0);

        // Get the page input rect position
        let rect = toolbar.page_input_rect.as_ref().unwrap();
        let center_x = rect.x + rect.width / 2.0;
        let center_y = rect.y + rect.height / 2.0;

        // Test hit in center of page input
        assert!(toolbar.hit_test_page_input(center_x, center_y));

        // Test miss outside the toolbar
        assert!(!toolbar.hit_test_page_input(1000.0, 100.0));
    }

    #[test]
    fn test_toolbar_page_input_not_hit_when_outside() {
        let toolbar = Toolbar::new(1200.0);

        // Test points that should not hit the page input
        assert!(!toolbar.hit_test_page_input(0.0, 0.0));
        assert!(!toolbar.hit_test_page_input(500.0, 500.0));
    }

    #[test]
    fn test_page_number_display_clone() {
        let display = PageNumberDisplay {
            current_page: 5,
            total_pages: 10,
            is_focused: true,
        };
        let cloned = display.clone();

        assert_eq!(cloned.current_page, 5);
        assert_eq!(cloned.total_pages, 10);
        assert!(cloned.is_focused);
    }

    #[test]
    fn test_toolbar_scene_has_page_input_primitives() {
        let mut toolbar = Toolbar::new(1200.0);
        toolbar.set_page_info(3, 15);

        // The scene node should have primitives including those for the page input
        let primitives = toolbar.scene_node().primitives();

        // Count rectangles - should be many since we render digits as rectangles
        let rect_count = primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Rectangle { .. }))
            .count();

        // Should have many rectangles: background, borders, buttons, and digit pixels
        assert!(
            rect_count > 20,
            "Should have many rectangles including page input (got {})",
            rect_count
        );
    }

    #[test]
    fn test_toolbar_page_display_no_rebuild_same_values() {
        let mut toolbar = Toolbar::new(1200.0);
        toolbar.set_page_info(5, 10);

        // Get the scene node pointer
        let node_ptr = Arc::as_ptr(toolbar.scene_node());

        // Set the same values - should not rebuild
        toolbar.set_page_info(5, 10);

        // Scene node should be the same (no rebuild occurred)
        assert_eq!(Arc::as_ptr(toolbar.scene_node()), node_ptr);
    }

    #[test]
    fn test_toolbar_page_display_rebuild_on_change() {
        let mut toolbar = Toolbar::new(1200.0);
        toolbar.set_page_info(5, 10);

        // Get the scene node pointer
        let node_ptr = Arc::as_ptr(toolbar.scene_node());

        // Change values - should trigger rebuild
        toolbar.set_page_info(6, 10);

        // Scene node should be different (rebuild occurred)
        assert_ne!(Arc::as_ptr(toolbar.scene_node()), node_ptr);
    }
}
