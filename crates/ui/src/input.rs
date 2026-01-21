//! Input handling for smooth pan and zoom
//!
//! This module provides input event processing for viewport navigation:
//! - Mouse drag for panning (smooth scrolling)
//! - Mouse wheel for zooming (with zoom centering on cursor position)
//! - Keyboard shortcuts for navigation
//!
//! The input handler maintains velocity-based smooth interpolation for
//! natural-feeling pan and zoom animations.

use pdf_editor_scheduler::Viewport;
use std::time::Duration;

/// Input state for pan and zoom operations
#[derive(Debug)]
pub struct InputHandler {
    /// Current viewport state
    viewport: Viewport,

    /// Pan state
    pan_state: PanState,

    /// Zoom state
    zoom_state: ZoomState,

    /// Mouse position (screen coordinates)
    mouse_position: (f32, f32),

    /// Whether mouse button is currently pressed
    mouse_pressed: bool,

    /// Viewport dimensions (screen size in pixels)
    viewport_width: f32,
    viewport_height: f32,
}

/// Pan state with velocity interpolation
#[derive(Debug, Clone)]
struct PanState {
    /// Current pan velocity (pixels per second)
    velocity_x: f32,
    velocity_y: f32,

    /// Last mouse position when dragging
    last_drag_pos: Option<(f32, f32)>,

    /// Drag start position
    drag_start: Option<(f32, f32)>,

    /// Momentum decay factor (0.0 - 1.0)
    momentum_decay: f32,
}

impl Default for PanState {
    fn default() -> Self {
        Self {
            velocity_x: 0.0,
            velocity_y: 0.0,
            last_drag_pos: None,
            drag_start: None,
            momentum_decay: 0.92, // Decay to 8% per frame at 60fps
        }
    }
}

/// Zoom state with smooth interpolation
#[derive(Debug, Clone)]
struct ZoomState {
    /// Target zoom level (percentage)
    target_zoom: u32,

    /// Zoom center point (screen coordinates)
    zoom_center: Option<(f32, f32)>,

    /// Zoom interpolation speed (0.0 - 1.0)
    interpolation_speed: f32,
}

impl Default for ZoomState {
    fn default() -> Self {
        Self {
            target_zoom: 100,
            zoom_center: None,
            interpolation_speed: 0.15, // 15% of remaining distance per frame
        }
    }
}

impl InputHandler {
    /// Create a new input handler
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        let viewport = Viewport::new(0, 0.0, 0.0, viewport_width, viewport_height, 100);

        Self {
            viewport,
            pan_state: PanState::default(),
            zoom_state: ZoomState::default(),
            mouse_position: (0.0, 0.0),
            mouse_pressed: false,
            viewport_width,
            viewport_height,
        }
    }

    /// Update the viewport dimensions (call on window resize)
    pub fn set_viewport_dimensions(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
        self.viewport.width = width;
        self.viewport.height = height;
    }

    /// Get the current viewport state
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    /// Get the current mouse position
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    /// Handle mouse motion event
    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);

        if self.mouse_pressed {
            if let Some((last_x, last_y)) = self.pan_state.last_drag_pos {
                // Calculate delta from last position
                let delta_x = x - last_x;
                let delta_y = y - last_y;

                // Pan viewport (negative delta because moving mouse right pans content left)
                self.viewport.x -= delta_x;
                self.viewport.y -= delta_y;

                // Update velocity for momentum (simplified - just delta)
                // In a real implementation, this would calculate velocity over time
                self.pan_state.velocity_x = -delta_x * 60.0; // Convert to per-second
                self.pan_state.velocity_y = -delta_y * 60.0;
            }

            self.pan_state.last_drag_pos = Some((x, y));
        }
    }

    /// Handle mouse button press
    pub fn on_mouse_down(&mut self, x: f32, y: f32) {
        self.mouse_pressed = true;
        self.mouse_position = (x, y);
        self.pan_state.drag_start = Some((x, y));
        self.pan_state.last_drag_pos = Some((x, y));

        // Stop any momentum
        self.pan_state.velocity_x = 0.0;
        self.pan_state.velocity_y = 0.0;
    }

    /// Handle mouse button release
    pub fn on_mouse_up(&mut self) {
        self.mouse_pressed = false;
        self.pan_state.last_drag_pos = None;
        self.pan_state.drag_start = None;

        // Keep momentum velocity for smooth deceleration
        // (velocity is already set from last drag movement)
    }

    /// Handle mouse wheel event (zoom)
    ///
    /// delta: scroll amount (positive = zoom in, negative = zoom out)
    pub fn on_mouse_wheel(&mut self, delta: f32) {
        // Calculate zoom factor (10% per wheel notch)
        let zoom_factor = if delta > 0.0 {
            1.1_f32.powf(delta)
        } else {
            0.9_f32.powf(-delta)
        };

        // Calculate new target zoom level
        let current_zoom = self.viewport.zoom_level as f32;
        let new_zoom = (current_zoom * zoom_factor).clamp(25.0, 400.0) as u32;

        // Store zoom center for focal point zooming
        self.zoom_state.zoom_center = Some(self.mouse_position);
        self.zoom_state.target_zoom = new_zoom;
    }

    /// Handle discrete zoom level change (e.g., keyboard shortcuts)
    ///
    /// zoom_level: target zoom percentage (25-400)
    pub fn set_zoom_level(&mut self, zoom_level: u32) {
        let clamped_zoom = zoom_level.clamp(25, 400);
        self.zoom_state.target_zoom = clamped_zoom;
        // Use center of viewport as zoom center
        self.zoom_state.zoom_center = Some((self.viewport_width / 2.0, self.viewport_height / 2.0));
    }

    /// Zoom in by one step (keyboard shortcut)
    pub fn zoom_in(&mut self) {
        let current = self.viewport.zoom_level;
        let next_level = match current {
            0..=24 => 25,
            25..=49 => 50,
            50..=74 => 75,
            75..=99 => 100,
            100..=124 => 125,
            125..=149 => 150,
            150..=199 => 200,
            200..=299 => 300,
            _ => 400,
        };
        self.set_zoom_level(next_level);
    }

    /// Zoom out by one step (keyboard shortcut)
    pub fn zoom_out(&mut self) {
        let current = self.viewport.zoom_level;
        let prev_level = match current {
            0..=25 => 25,
            26..=50 => 25,
            51..=75 => 50,
            76..=100 => 75,
            101..=125 => 100,
            126..=150 => 125,
            151..=200 => 150,
            201..=300 => 200,
            _ => 300,
        };
        self.set_zoom_level(prev_level);
    }

    /// Reset zoom to 100%
    pub fn zoom_reset(&mut self) {
        self.set_zoom_level(100);
    }

    /// Update animation state (call every frame)
    ///
    /// delta_time: time since last frame
    /// Returns true if viewport changed
    pub fn update(&mut self, delta_time: Duration) -> bool {
        let mut changed = false;

        // Update momentum-based panning
        if !self.mouse_pressed {
            let delta_seconds = delta_time.as_secs_f32();

            // Apply velocity decay
            self.pan_state.velocity_x *= self.pan_state.momentum_decay;
            self.pan_state.velocity_y *= self.pan_state.momentum_decay;

            // Apply velocity to viewport position
            let vel_threshold = 0.5; // Stop when velocity is very small
            if self.pan_state.velocity_x.abs() > vel_threshold
                || self.pan_state.velocity_y.abs() > vel_threshold
            {
                self.viewport.x += self.pan_state.velocity_x * delta_seconds;
                self.viewport.y += self.pan_state.velocity_y * delta_seconds;
                changed = true;
            } else {
                // Stop momentum completely
                self.pan_state.velocity_x = 0.0;
                self.pan_state.velocity_y = 0.0;
            }
        } else {
            // Mouse is pressed, viewport is being updated in on_mouse_move
            changed = true;
        }

        // Update smooth zooming
        if self.viewport.zoom_level != self.zoom_state.target_zoom {
            let current_zoom = self.viewport.zoom_level as f32;
            let target_zoom = self.zoom_state.target_zoom as f32;

            // Interpolate zoom level
            let new_zoom = current_zoom
                + (target_zoom - current_zoom) * self.zoom_state.interpolation_speed;

            // If very close to target, snap to it
            if (new_zoom - target_zoom).abs() < 0.5 {
                self.viewport.zoom_level = self.zoom_state.target_zoom;
            } else {
                // Adjust viewport position to zoom toward center point
                if let Some((center_x, center_y)) = self.zoom_state.zoom_center {
                    let old_zoom = self.viewport.zoom_level as f32 / 100.0;
                    let new_zoom_normalized = new_zoom / 100.0;

                    // Calculate point in page coordinates under cursor
                    let page_x = self.viewport.x + center_x / old_zoom;
                    let page_y = self.viewport.y + center_y / old_zoom;

                    // Update viewport to keep that point under cursor at new zoom
                    self.viewport.x = page_x - center_x / new_zoom_normalized;
                    self.viewport.y = page_y - center_y / new_zoom_normalized;
                }

                self.viewport.zoom_level = new_zoom.round() as u32;
            }

            changed = true;
        }

        // Clamp viewport position (prevent negative coordinates)
        // In a full implementation, this would respect page boundaries
        if self.viewport.x < 0.0 {
            self.viewport.x = 0.0;
            self.pan_state.velocity_x = 0.0;
        }
        if self.viewport.y < 0.0 {
            self.viewport.y = 0.0;
            self.pan_state.velocity_y = 0.0;
        }

        changed
    }

    /// Navigate to a specific page
    pub fn go_to_page(&mut self, page_index: u16) {
        self.viewport.page_index = page_index;
        // Reset viewport position when changing pages
        self.viewport.x = 0.0;
        self.viewport.y = 0.0;
        self.pan_state.velocity_x = 0.0;
        self.pan_state.velocity_y = 0.0;
    }

    /// Navigate to next page
    pub fn next_page(&mut self) {
        self.go_to_page(self.viewport.page_index.saturating_add(1));
    }

    /// Navigate to previous page
    pub fn prev_page(&mut self) {
        self.go_to_page(self.viewport.page_index.saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_handler_creation() {
        let handler = InputHandler::new(1024.0, 768.0);
        assert_eq!(handler.viewport().zoom_level, 100);
        assert_eq!(handler.viewport().page_index, 0);
    }

    #[test]
    fn test_pan_with_mouse() {
        let mut handler = InputHandler::new(1024.0, 768.0);

        // Start drag
        handler.on_mouse_down(500.0, 400.0);

        // Move mouse (drag right and down)
        handler.on_mouse_move(600.0, 500.0);

        // Viewport should pan left and up (opposite of mouse movement)
        assert!(handler.viewport().x < 0.0);
        assert!(handler.viewport().y < 0.0);
    }

    #[test]
    fn test_zoom_with_wheel() {
        let mut handler = InputHandler::new(1024.0, 768.0);

        // Position mouse at center
        handler.on_mouse_move(512.0, 384.0);

        // Zoom in
        handler.on_mouse_wheel(1.0);

        // Target zoom should be > 100
        assert!(handler.zoom_state.target_zoom > 100);
    }

    #[test]
    fn test_zoom_clamping() {
        let mut handler = InputHandler::new(1024.0, 768.0);

        // Try to zoom beyond max
        handler.set_zoom_level(500);
        assert_eq!(handler.zoom_state.target_zoom, 400);

        // Try to zoom below min
        handler.set_zoom_level(10);
        assert_eq!(handler.zoom_state.target_zoom, 25);
    }

    #[test]
    fn test_zoom_discrete_steps() {
        let mut handler = InputHandler::new(1024.0, 768.0);

        // Zoom in from 100%
        handler.zoom_in();
        assert_eq!(handler.zoom_state.target_zoom, 125);

        // Zoom in again
        handler.zoom_state.target_zoom = 125; // Simulate reaching target
        handler.viewport.zoom_level = 125;
        handler.zoom_in();
        assert_eq!(handler.zoom_state.target_zoom, 150);

        // Zoom out
        handler.zoom_state.target_zoom = 150;
        handler.viewport.zoom_level = 150;
        handler.zoom_out();
        assert_eq!(handler.zoom_state.target_zoom, 125);
    }

    #[test]
    fn test_zoom_reset() {
        let mut handler = InputHandler::new(1024.0, 768.0);

        handler.set_zoom_level(200);
        assert_eq!(handler.zoom_state.target_zoom, 200);

        handler.zoom_reset();
        assert_eq!(handler.zoom_state.target_zoom, 100);
    }

    #[test]
    fn test_momentum_decay() {
        let mut handler = InputHandler::new(1024.0, 768.0);

        // Set initial velocity
        handler.pan_state.velocity_x = 100.0;
        handler.pan_state.velocity_y = 100.0;

        // Update for 1 frame (16ms at 60fps)
        let delta = Duration::from_millis(16);
        handler.update(delta);

        // Velocity should have decayed
        assert!(handler.pan_state.velocity_x < 100.0);
        assert!(handler.pan_state.velocity_y < 100.0);
    }

    #[test]
    fn test_page_navigation() {
        let mut handler = InputHandler::new(1024.0, 768.0);

        // Start on page 0
        assert_eq!(handler.viewport().page_index, 0);

        // Go to next page
        handler.next_page();
        assert_eq!(handler.viewport().page_index, 1);

        // Go to previous page
        handler.prev_page();
        assert_eq!(handler.viewport().page_index, 0);

        // Previous on page 0 should stay at 0
        handler.prev_page();
        assert_eq!(handler.viewport().page_index, 0);
    }

    #[test]
    fn test_viewport_position_reset_on_page_change() {
        let mut handler = InputHandler::new(1024.0, 768.0);

        // Pan viewport
        handler.on_mouse_down(500.0, 400.0);
        handler.on_mouse_move(400.0, 300.0);

        // Viewport should be panned
        assert!(handler.viewport().x != 0.0 || handler.viewport().y != 0.0);

        // Change page
        handler.go_to_page(1);

        // Viewport should be reset
        assert_eq!(handler.viewport().x, 0.0);
        assert_eq!(handler.viewport().y, 0.0);
        assert_eq!(handler.viewport().page_index, 1);
    }

    #[test]
    fn test_smooth_zoom_interpolation() {
        let mut handler = InputHandler::new(1024.0, 768.0);

        // Set target zoom to 200%
        handler.set_zoom_level(200);

        // Current zoom is 100, target is 200
        assert_eq!(handler.viewport().zoom_level, 100);
        assert_eq!(handler.zoom_state.target_zoom, 200);

        // Update once - zoom should interpolate toward target
        let delta = Duration::from_millis(16);
        let changed = handler.update(delta);

        assert!(changed);
        assert!(handler.viewport().zoom_level > 100);
        assert!(handler.viewport().zoom_level < 200);
    }

    #[test]
    fn test_viewport_dimensions_update() {
        let mut handler = InputHandler::new(1024.0, 768.0);

        assert_eq!(handler.viewport().width, 1024.0);
        assert_eq!(handler.viewport().height, 768.0);

        handler.set_viewport_dimensions(1920.0, 1080.0);

        assert_eq!(handler.viewport().width, 1920.0);
        assert_eq!(handler.viewport().height, 1080.0);
    }
}
