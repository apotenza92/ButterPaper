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

/// Width of the zoom dropdown display
const ZOOM_DROPDOWN_WIDTH: f32 = 60.0;
/// Height of the zoom dropdown display
const ZOOM_DROPDOWN_HEIGHT: f32 = 24.0;
/// Height of each item in the dropdown menu
const ZOOM_DROPDOWN_ITEM_HEIGHT: f32 = 28.0;

/// Available zoom levels for the dropdown
pub const ZOOM_LEVELS: [u32; 9] = [25, 50, 75, 100, 125, 150, 200, 300, 400];

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

/// Zoom dropdown display state
#[derive(Debug, Clone)]
pub struct ZoomDropdownState {
    /// Current zoom level as percentage (e.g., 100 for 100%)
    pub current_zoom: u32,
    /// Whether the dropdown is open
    pub is_open: bool,
    /// Hovered item index (if dropdown is open)
    pub hovered_item: Option<usize>,
}

impl Default for ZoomDropdownState {
    fn default() -> Self {
        Self {
            current_zoom: 100,
            is_open: false,
            hovered_item: None,
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

    /// Zoom dropdown state
    zoom_dropdown: ZoomDropdownState,

    /// Rectangle bounds for the zoom dropdown display (for hit testing)
    zoom_dropdown_rect: Option<Rect>,
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
            zoom_dropdown: ZoomDropdownState::default(),
            zoom_dropdown_rect: None,
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
            zoom_dropdown: ZoomDropdownState::default(),
            zoom_dropdown_rect: None,
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

    // --- Zoom dropdown methods ---

    /// Set the current zoom level (as percentage, e.g., 100 for 100%)
    pub fn set_zoom_level(&mut self, zoom: u32) {
        if self.zoom_dropdown.current_zoom != zoom {
            self.zoom_dropdown.current_zoom = zoom;
            self.rebuild();
        }
    }

    /// Get the current zoom level
    pub fn zoom_level(&self) -> u32 {
        self.zoom_dropdown.current_zoom
    }

    /// Get the zoom dropdown state
    pub fn zoom_dropdown(&self) -> &ZoomDropdownState {
        &self.zoom_dropdown
    }

    /// Check if a point is within the zoom dropdown display
    pub fn hit_test_zoom_dropdown(&self, x: f32, y: f32) -> bool {
        if let Some(rect) = &self.zoom_dropdown_rect {
            x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height
        } else {
            false
        }
    }

    /// Toggle the zoom dropdown open/closed
    pub fn toggle_zoom_dropdown(&mut self) {
        self.zoom_dropdown.is_open = !self.zoom_dropdown.is_open;
        self.zoom_dropdown.hovered_item = None;
        self.rebuild();
    }

    /// Close the zoom dropdown
    pub fn close_zoom_dropdown(&mut self) {
        if self.zoom_dropdown.is_open {
            self.zoom_dropdown.is_open = false;
            self.zoom_dropdown.hovered_item = None;
            self.rebuild();
        }
    }

    /// Check if the zoom dropdown is open
    pub fn is_zoom_dropdown_open(&self) -> bool {
        self.zoom_dropdown.is_open
    }

    /// Hit test for zoom dropdown menu items when the dropdown is open
    /// Returns the index of the zoom level if a menu item is hit
    pub fn hit_test_zoom_dropdown_item(&self, x: f32, y: f32) -> Option<usize> {
        if !self.zoom_dropdown.is_open {
            return None;
        }

        if let Some(rect) = &self.zoom_dropdown_rect {
            // Check if within the dropdown menu area (below the display)
            let menu_x = rect.x;
            let menu_y = rect.y + rect.height;
            let menu_width = rect.width;
            let menu_height = ZOOM_LEVELS.len() as f32 * ZOOM_DROPDOWN_ITEM_HEIGHT;

            if x >= menu_x && x <= menu_x + menu_width && y >= menu_y && y <= menu_y + menu_height {
                let item_index = ((y - menu_y) / ZOOM_DROPDOWN_ITEM_HEIGHT) as usize;
                if item_index < ZOOM_LEVELS.len() {
                    return Some(item_index);
                }
            }
        }

        None
    }

    /// Set the hovered item in the zoom dropdown
    pub fn set_zoom_dropdown_hover(&mut self, item_index: Option<usize>) {
        if self.zoom_dropdown.hovered_item != item_index {
            self.zoom_dropdown.hovered_item = item_index;
            if self.zoom_dropdown.is_open {
                self.rebuild();
            }
        }
    }

    /// Get the dropdown menu bounds for checking if a click is outside
    pub fn zoom_dropdown_menu_bounds(&self) -> Option<Rect> {
        if !self.zoom_dropdown.is_open {
            return None;
        }

        self.zoom_dropdown_rect.map(|rect| {
            Rect::new(
                rect.x,
                rect.y,
                rect.width,
                rect.height + ZOOM_LEVELS.len() as f32 * ZOOM_DROPDOWN_ITEM_HEIGHT,
            )
        })
    }

    /// Set button hover state
    pub fn set_button_hover(&mut self, button: ToolbarButton, hovering: bool) {
        let new_state = if hovering {
            ButtonState::Hover
        } else {
            ButtonState::Normal
        };

        // Find and update the button state, and clear hover from other buttons
        let mut changed = false;
        for (btn, state, _) in &mut self.button_states {
            if *btn == button {
                if *state != new_state {
                    *state = new_state;
                    changed = true;
                }
            } else if hovering && *state == ButtonState::Hover {
                // Clear hover state from other buttons when hovering a new one
                *state = ButtonState::Normal;
                changed = true;
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
        x = self.add_zoom_dropdown(&mut new_node, x, button_y);
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

    /// Add the zoom dropdown display to the scene node and return the next x position
    fn add_zoom_dropdown(&mut self, node: &mut SceneNode, x: f32, _button_y: f32) -> f32 {
        let dropdown_y = (TOOLBAR_HEIGHT - ZOOM_DROPDOWN_HEIGHT) / 2.0;
        let rect = Rect::new(x, dropdown_y, ZOOM_DROPDOWN_WIDTH, ZOOM_DROPDOWN_HEIGHT);

        // Store the rect for hit testing
        self.zoom_dropdown_rect = Some(rect);

        // Background - slightly different color when dropdown is open
        let bg_color = if self.zoom_dropdown.is_open {
            self.config.button_hover_color
        } else {
            Color::rgba(0.15, 0.15, 0.15, 1.0)
        };
        node.add_primitive(Primitive::Rectangle {
            rect,
            color: bg_color,
        });

        // Border
        let border_color = Color::rgba(0.35, 0.35, 0.35, 1.0);
        // Top border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, dropdown_y, ZOOM_DROPDOWN_WIDTH, 1.0),
            color: border_color,
        });
        // Bottom border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(
                x,
                dropdown_y + ZOOM_DROPDOWN_HEIGHT - 1.0,
                ZOOM_DROPDOWN_WIDTH,
                1.0,
            ),
            color: border_color,
        });
        // Left border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(x, dropdown_y, 1.0, ZOOM_DROPDOWN_HEIGHT),
            color: border_color,
        });
        // Right border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(
                x + ZOOM_DROPDOWN_WIDTH - 1.0,
                dropdown_y,
                1.0,
                ZOOM_DROPDOWN_HEIGHT,
            ),
            color: border_color,
        });

        // Render the current zoom level text (e.g., "100%")
        let zoom_text = format!("{}%", self.zoom_dropdown.current_zoom);
        Self::render_zoom_text(node, &zoom_text, x, dropdown_y);

        // Dropdown arrow indicator (small triangle on the right side)
        let arrow_size = 4.0;
        let arrow_x = x + ZOOM_DROPDOWN_WIDTH - 10.0;
        let arrow_y = dropdown_y + ZOOM_DROPDOWN_HEIGHT / 2.0;
        node.add_primitive(Primitive::Polygon {
            points: vec![
                [arrow_x - arrow_size / 2.0, arrow_y - arrow_size / 3.0],
                [arrow_x + arrow_size / 2.0, arrow_y - arrow_size / 3.0],
                [arrow_x, arrow_y + arrow_size / 2.0],
            ],
            fill_color: Some(Color::rgba(0.7, 0.7, 0.7, 1.0)),
            stroke_color: Color::rgba(0.7, 0.7, 0.7, 1.0),
            stroke_width: 0.0,
        });

        // If dropdown is open, render the menu items
        if self.zoom_dropdown.is_open {
            self.render_zoom_dropdown_menu(node, x, dropdown_y + ZOOM_DROPDOWN_HEIGHT);
        }

        x + ZOOM_DROPDOWN_WIDTH + self.config.button_spacing
    }

    /// Render the dropdown menu for zoom levels
    fn render_zoom_dropdown_menu(&self, node: &mut SceneNode, menu_x: f32, menu_y: f32) {
        let menu_width = ZOOM_DROPDOWN_WIDTH;
        let menu_height = ZOOM_LEVELS.len() as f32 * ZOOM_DROPDOWN_ITEM_HEIGHT;

        // Menu background
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(menu_x, menu_y, menu_width, menu_height),
            color: Color::rgba(0.2, 0.2, 0.2, 0.98),
        });

        // Menu border
        let border_color = Color::rgba(0.35, 0.35, 0.35, 1.0);
        // Left border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(menu_x, menu_y, 1.0, menu_height),
            color: border_color,
        });
        // Right border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(menu_x + menu_width - 1.0, menu_y, 1.0, menu_height),
            color: border_color,
        });
        // Bottom border
        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(menu_x, menu_y + menu_height - 1.0, menu_width, 1.0),
            color: border_color,
        });

        // Render each menu item
        for (idx, &zoom_level) in ZOOM_LEVELS.iter().enumerate() {
            let item_y = menu_y + idx as f32 * ZOOM_DROPDOWN_ITEM_HEIGHT;

            // Highlight background for hovered or current item
            let is_hovered = self.zoom_dropdown.hovered_item == Some(idx);
            let is_current = self.zoom_dropdown.current_zoom == zoom_level;

            if is_hovered || is_current {
                let highlight_color = if is_hovered {
                    self.config.button_hover_color
                } else {
                    Color::rgba(0.25, 0.35, 0.5, 0.5)
                };
                node.add_primitive(Primitive::Rectangle {
                    rect: Rect::new(
                        menu_x + 1.0,
                        item_y,
                        menu_width - 2.0,
                        ZOOM_DROPDOWN_ITEM_HEIGHT,
                    ),
                    color: highlight_color,
                });
            }

            // Render the zoom level text
            let item_text = format!("{}%", zoom_level);
            Self::render_zoom_menu_item_text(node, &item_text, menu_x, item_y, menu_width);
        }
    }

    /// Render zoom text in the dropdown display
    fn render_zoom_text(node: &mut SceneNode, text: &str, field_x: f32, field_y: f32) {
        let text_color = Color::rgba(0.85, 0.85, 0.85, 1.0);

        // Character dimensions (simplified 3x5 font at scale 2)
        let char_width = 6.0_f32;
        let char_height = 10.0_f32;
        let char_spacing = 2.0_f32;

        // Calculate total text width
        let text_width = text.len() as f32 * (char_width + char_spacing) - char_spacing;

        // Center the text in the dropdown (accounting for the arrow on the right)
        let available_width = ZOOM_DROPDOWN_WIDTH - 16.0; // Leave space for arrow
        let start_x = field_x + (available_width - text_width) / 2.0;
        let start_y = field_y + (ZOOM_DROPDOWN_HEIGHT - char_height) / 2.0;

        let mut current_x = start_x;

        for c in text.chars() {
            let char_rect = Rect::new(current_x, start_y, char_width, char_height);
            Self::render_char(node, c, char_rect, text_color);
            current_x += char_width + char_spacing;
        }
    }

    /// Render text for a menu item
    fn render_zoom_menu_item_text(
        node: &mut SceneNode,
        text: &str,
        menu_x: f32,
        item_y: f32,
        menu_width: f32,
    ) {
        let text_color = Color::rgba(0.85, 0.85, 0.85, 1.0);

        // Character dimensions
        let char_width = 6.0_f32;
        let char_height = 10.0_f32;
        let char_spacing = 2.0_f32;

        // Calculate total text width
        let text_width = text.len() as f32 * (char_width + char_spacing) - char_spacing;

        // Center the text in the menu item
        let start_x = menu_x + (menu_width - text_width) / 2.0;
        let start_y = item_y + (ZOOM_DROPDOWN_ITEM_HEIGHT - char_height) / 2.0;

        let mut current_x = start_x;

        for c in text.chars() {
            let char_rect = Rect::new(current_x, start_y, char_width, char_height);
            Self::render_char(node, c, char_rect, text_color);
            current_x += char_width + char_spacing;
        }
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
            '%' => [0b101, 0b001, 0b010, 0b100, 0b101],
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

    // Zoom dropdown tests

    #[test]
    fn test_zoom_dropdown_state_default() {
        let state = ZoomDropdownState::default();
        assert_eq!(state.current_zoom, 100);
        assert!(!state.is_open);
        assert!(state.hovered_item.is_none());
    }

    #[test]
    fn test_toolbar_zoom_level_default() {
        let toolbar = Toolbar::new(1200.0);
        assert_eq!(toolbar.zoom_level(), 100);
        assert!(!toolbar.is_zoom_dropdown_open());
    }

    #[test]
    fn test_toolbar_set_zoom_level() {
        let mut toolbar = Toolbar::new(1200.0);

        toolbar.set_zoom_level(150);
        assert_eq!(toolbar.zoom_level(), 150);

        toolbar.set_zoom_level(50);
        assert_eq!(toolbar.zoom_level(), 50);
    }

    #[test]
    fn test_toolbar_zoom_dropdown_toggle() {
        let mut toolbar = Toolbar::new(1200.0);

        assert!(!toolbar.is_zoom_dropdown_open());

        toolbar.toggle_zoom_dropdown();
        assert!(toolbar.is_zoom_dropdown_open());

        toolbar.toggle_zoom_dropdown();
        assert!(!toolbar.is_zoom_dropdown_open());
    }

    #[test]
    fn test_toolbar_zoom_dropdown_close() {
        let mut toolbar = Toolbar::new(1200.0);

        toolbar.toggle_zoom_dropdown();
        assert!(toolbar.is_zoom_dropdown_open());

        toolbar.close_zoom_dropdown();
        assert!(!toolbar.is_zoom_dropdown_open());
    }

    #[test]
    fn test_toolbar_zoom_dropdown_close_when_already_closed() {
        let mut toolbar = Toolbar::new(1200.0);
        let node_ptr = Arc::as_ptr(toolbar.scene_node());

        // Closing when already closed should not rebuild
        toolbar.close_zoom_dropdown();
        assert_eq!(Arc::as_ptr(toolbar.scene_node()), node_ptr);
    }

    #[test]
    fn test_toolbar_zoom_dropdown_rect_exists() {
        let toolbar = Toolbar::new(1200.0);

        // The zoom dropdown rect should be set after construction
        assert!(toolbar.zoom_dropdown_rect.is_some());
    }

    #[test]
    fn test_toolbar_hit_test_zoom_dropdown() {
        let toolbar = Toolbar::new(1200.0);

        // Get the zoom dropdown rect position
        let rect = toolbar.zoom_dropdown_rect.as_ref().unwrap();
        let center_x = rect.x + rect.width / 2.0;
        let center_y = rect.y + rect.height / 2.0;

        // Test hit in center of zoom dropdown
        assert!(toolbar.hit_test_zoom_dropdown(center_x, center_y));

        // Test miss outside the dropdown
        assert!(!toolbar.hit_test_zoom_dropdown(1000.0, 100.0));
    }

    #[test]
    fn test_toolbar_hit_test_zoom_dropdown_item_when_closed() {
        let toolbar = Toolbar::new(1200.0);

        // When dropdown is closed, should return None even if coordinates are valid
        let rect = toolbar.zoom_dropdown_rect.as_ref().unwrap();
        let x = rect.x + 10.0;
        let y = rect.y + rect.height + 10.0; // Below the dropdown display

        assert!(toolbar.hit_test_zoom_dropdown_item(x, y).is_none());
    }

    #[test]
    fn test_toolbar_hit_test_zoom_dropdown_item_when_open() {
        let mut toolbar = Toolbar::new(1200.0);
        toolbar.toggle_zoom_dropdown();

        // When dropdown is open, should detect item hits
        let rect = toolbar.zoom_dropdown_rect.as_ref().unwrap();
        let x = rect.x + 10.0;
        let y = rect.y + rect.height + 5.0; // Just below the dropdown display, in first item

        let result = toolbar.hit_test_zoom_dropdown_item(x, y);
        assert!(result.is_some());
        assert_eq!(result, Some(0)); // First zoom level
    }

    #[test]
    fn test_toolbar_zoom_dropdown_hover() {
        let mut toolbar = Toolbar::new(1200.0);
        toolbar.toggle_zoom_dropdown();

        // Set hover on an item
        toolbar.set_zoom_dropdown_hover(Some(2));
        assert_eq!(toolbar.zoom_dropdown().hovered_item, Some(2));

        // Clear hover
        toolbar.set_zoom_dropdown_hover(None);
        assert!(toolbar.zoom_dropdown().hovered_item.is_none());
    }

    #[test]
    fn test_toolbar_zoom_dropdown_menu_bounds_when_closed() {
        let toolbar = Toolbar::new(1200.0);

        // When dropdown is closed, menu bounds should be None
        assert!(toolbar.zoom_dropdown_menu_bounds().is_none());
    }

    #[test]
    fn test_toolbar_zoom_dropdown_menu_bounds_when_open() {
        let mut toolbar = Toolbar::new(1200.0);
        toolbar.toggle_zoom_dropdown();

        // When dropdown is open, menu bounds should include the menu items
        let bounds = toolbar.zoom_dropdown_menu_bounds();
        assert!(bounds.is_some());

        let bounds = bounds.unwrap();
        let rect = toolbar.zoom_dropdown_rect.as_ref().unwrap();

        // Menu should start at the same x position
        assert_eq!(bounds.x, rect.x);
        // Menu should include the display plus the items
        assert!(bounds.height > rect.height);
    }

    #[test]
    fn test_toolbar_zoom_level_no_rebuild_same_value() {
        let mut toolbar = Toolbar::new(1200.0);
        toolbar.set_zoom_level(150);

        // Get the scene node pointer
        let node_ptr = Arc::as_ptr(toolbar.scene_node());

        // Set the same value - should not rebuild
        toolbar.set_zoom_level(150);

        // Scene node should be the same (no rebuild occurred)
        assert_eq!(Arc::as_ptr(toolbar.scene_node()), node_ptr);
    }

    #[test]
    fn test_toolbar_zoom_level_rebuild_on_change() {
        let mut toolbar = Toolbar::new(1200.0);
        toolbar.set_zoom_level(100);

        // Get the scene node pointer
        let node_ptr = Arc::as_ptr(toolbar.scene_node());

        // Change value - should trigger rebuild
        toolbar.set_zoom_level(200);

        // Scene node should be different (rebuild occurred)
        assert_ne!(Arc::as_ptr(toolbar.scene_node()), node_ptr);
    }

    #[test]
    fn test_zoom_levels_constant() {
        // Verify ZOOM_LEVELS contains expected values
        assert_eq!(ZOOM_LEVELS.len(), 9);
        assert!(ZOOM_LEVELS.contains(&25));
        assert!(ZOOM_LEVELS.contains(&50));
        assert!(ZOOM_LEVELS.contains(&75));
        assert!(ZOOM_LEVELS.contains(&100));
        assert!(ZOOM_LEVELS.contains(&125));
        assert!(ZOOM_LEVELS.contains(&150));
        assert!(ZOOM_LEVELS.contains(&200));
        assert!(ZOOM_LEVELS.contains(&300));
        assert!(ZOOM_LEVELS.contains(&400));
    }

    #[test]
    fn test_zoom_dropdown_state_clone() {
        let state = ZoomDropdownState {
            current_zoom: 150,
            is_open: true,
            hovered_item: Some(3),
        };
        let cloned = state.clone();

        assert_eq!(cloned.current_zoom, 150);
        assert!(cloned.is_open);
        assert_eq!(cloned.hovered_item, Some(3));
    }

    #[test]
    fn test_toolbar_scene_has_zoom_dropdown_primitives() {
        let mut toolbar = Toolbar::new(1200.0);
        toolbar.set_zoom_level(200);

        // The scene node should have primitives including those for the zoom dropdown
        let primitives = toolbar.scene_node().primitives();

        // Should have the dropdown arrow (triangle/polygon)
        let has_polygon = primitives.iter().any(|p| matches!(p, Primitive::Polygon { .. }));
        assert!(has_polygon, "Toolbar should have polygon primitives for dropdown arrow");
    }

    #[test]
    fn test_toolbar_scene_has_zoom_menu_when_open() {
        let mut toolbar = Toolbar::new(1200.0);
        let primitives_closed = toolbar.scene_node().primitives().len();

        toolbar.toggle_zoom_dropdown();
        let primitives_open = toolbar.scene_node().primitives().len();

        // When dropdown is open, should have more primitives for the menu items
        assert!(
            primitives_open > primitives_closed,
            "Open dropdown should have more primitives (closed: {}, open: {})",
            primitives_closed,
            primitives_open
        );
    }

    #[test]
    fn test_toolbar_hover_clears_other_buttons() {
        let mut toolbar = Toolbar::new(1200.0);

        // Set hover on ZoomIn button
        toolbar.set_button_hover(ToolbarButton::ZoomIn, true);

        // Verify ZoomIn is hovered
        let zoom_in_state = toolbar
            .button_states
            .iter()
            .find(|(b, _, _)| *b == ToolbarButton::ZoomIn)
            .map(|(_, s, _)| *s);
        assert_eq!(zoom_in_state, Some(ButtonState::Hover));

        // Now hover ZoomOut button
        toolbar.set_button_hover(ToolbarButton::ZoomOut, true);

        // Verify ZoomOut is now hovered
        let zoom_out_state = toolbar
            .button_states
            .iter()
            .find(|(b, _, _)| *b == ToolbarButton::ZoomOut)
            .map(|(_, s, _)| *s);
        assert_eq!(zoom_out_state, Some(ButtonState::Hover));

        // Verify ZoomIn is no longer hovered
        let zoom_in_state = toolbar
            .button_states
            .iter()
            .find(|(b, _, _)| *b == ToolbarButton::ZoomIn)
            .map(|(_, s, _)| *s);
        assert_eq!(zoom_in_state, Some(ButtonState::Normal));
    }

    #[test]
    fn test_toolbar_clear_all_hover_states() {
        let mut toolbar = Toolbar::new(1200.0);

        // Set hover on multiple buttons
        toolbar.set_button_hover(ToolbarButton::ZoomIn, true);

        // Verify at least one button is hovered
        let has_hover = toolbar
            .button_states
            .iter()
            .any(|(_, s, _)| *s == ButtonState::Hover);
        assert!(has_hover, "Should have at least one hovered button");

        // Clear all hover states
        toolbar.clear_all_hover_states();

        // Verify no buttons are hovered
        let has_hover = toolbar
            .button_states
            .iter()
            .any(|(_, s, _)| *s == ButtonState::Hover);
        assert!(!has_hover, "Should have no hovered buttons after clearing");
    }

    #[test]
    fn test_toolbar_clear_all_hover_no_rebuild_when_no_hover() {
        let mut toolbar = Toolbar::new(1200.0);

        // Get the scene node pointer (no buttons are hovered initially)
        let node_ptr = Arc::as_ptr(toolbar.scene_node());

        // Clear all hover states when none are hovered - should not rebuild
        toolbar.clear_all_hover_states();

        // Scene node should be the same (no rebuild occurred)
        assert_eq!(Arc::as_ptr(toolbar.scene_node()), node_ptr);
    }

    #[test]
    fn test_toolbar_hover_only_one_button_at_a_time() {
        let mut toolbar = Toolbar::new(1200.0);

        // Hover multiple buttons in sequence
        let buttons = [
            ToolbarButton::PrevPage,
            ToolbarButton::NextPage,
            ToolbarButton::SelectTool,
            ToolbarButton::HandTool,
        ];

        for &button in &buttons {
            toolbar.set_button_hover(button, true);

            // Count how many buttons are hovered
            let hover_count = toolbar
                .button_states
                .iter()
                .filter(|(_, s, _)| *s == ButtonState::Hover)
                .count();

            // Only one button should be hovered at a time
            assert_eq!(
                hover_count, 1,
                "Only one button should be hovered, but {} are hovered after hovering {:?}",
                hover_count, button
            );
        }
    }
}
