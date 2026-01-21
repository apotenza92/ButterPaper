//! Input handling for smooth pan and zoom
//!
//! This module provides input event processing for viewport navigation:
//! - Mouse drag for panning (smooth scrolling)
//! - Mouse wheel for zooming (with zoom centering on cursor position)
//! - Keyboard shortcuts for navigation
//! - Vector-based hit testing for annotation selection
//!
//! The input handler maintains velocity-based smooth interpolation for
//! natural-feeling pan and zoom animations.
//!
//! Zoom levels are quantized to discrete steps (25, 50, 75, 100, 125, 150, 200, 300, 400)
//! to optimize tile cache efficiency. Smooth interpolation provides seamless transitions
//! between discrete levels.

use pdf_editor_core::annotation::{AnnotationCollection, AnnotationId};
use pdf_editor_core::manipulation::{
    generate_handles, ManipulationHandle, ManipulationState,
};
use pdf_editor_core::PageCoordinate;
use pdf_editor_scheduler::Viewport;
use std::time::Duration;

/// Discrete zoom levels for tile rendering optimization
/// These are the only zoom levels at which tiles will be rendered and cached
const ZOOM_LEVELS: [u32; 9] = [25, 50, 75, 100, 125, 150, 200, 300, 400];

/// Snap a zoom level to the nearest discrete level
fn snap_to_discrete_zoom(zoom: u32) -> u32 {
    let clamped = zoom.clamp(ZOOM_LEVELS[0], ZOOM_LEVELS[ZOOM_LEVELS.len() - 1]);

    // Find the closest discrete level
    ZOOM_LEVELS
        .iter()
        .min_by_key(|&&level| (level as i32 - clamped as i32).abs())
        .copied()
        .unwrap_or(100)
}

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

    /// Currently selected annotation (if any)
    selected_annotation: Option<AnnotationId>,

    /// Hit test tolerance in page coordinates (points)
    hit_test_tolerance: f32,

    /// Active manipulation state (if dragging a handle)
    manipulation_state: Option<ManipulationState>,

    /// Handle size in page coordinates (points)
    handle_size: f32,
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
            selected_annotation: None,
            hit_test_tolerance: 5.0, // 5 points in page coordinates
            manipulation_state: None,
            handle_size: 6.0, // 6 points in page coordinates
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
    /// Snaps to discrete zoom levels for tile cache efficiency
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

        // Snap to nearest discrete zoom level
        let discrete_zoom = snap_to_discrete_zoom(new_zoom);

        // Only update if we're moving to a different discrete level
        if discrete_zoom != self.zoom_state.target_zoom {
            // Store zoom center for focal point zooming
            self.zoom_state.zoom_center = Some(self.mouse_position);
            self.zoom_state.target_zoom = discrete_zoom;
        }
    }

    /// Handle discrete zoom level change (e.g., keyboard shortcuts)
    ///
    /// zoom_level: target zoom percentage (25-400)
    /// Snaps to nearest discrete zoom level
    pub fn set_zoom_level(&mut self, zoom_level: u32) {
        let discrete_zoom = snap_to_discrete_zoom(zoom_level);
        self.zoom_state.target_zoom = discrete_zoom;
        // Use center of viewport as zoom center
        self.zoom_state.zoom_center = Some((self.viewport_width / 2.0, self.viewport_height / 2.0));
    }

    /// Zoom in by one step (keyboard shortcut)
    pub fn zoom_in(&mut self) {
        let current = self.viewport.zoom_level;

        // Find the next higher discrete level
        let next_level = ZOOM_LEVELS
            .iter()
            .find(|&&level| level > current)
            .copied()
            .unwrap_or(ZOOM_LEVELS[ZOOM_LEVELS.len() - 1]);

        self.set_zoom_level(next_level);
    }

    /// Zoom out by one step (keyboard shortcut)
    pub fn zoom_out(&mut self) {
        let current = self.viewport.zoom_level;

        // Find the next lower discrete level
        let prev_level = ZOOM_LEVELS
            .iter()
            .rev()
            .find(|&&level| level < current)
            .copied()
            .unwrap_or(ZOOM_LEVELS[0]);

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

    /// Convert screen coordinates to page coordinates
    ///
    /// # Arguments
    /// * `screen_x` - X coordinate in screen space (pixels)
    /// * `screen_y` - Y coordinate in screen space (pixels)
    ///
    /// # Returns
    /// Page coordinate accounting for zoom and pan
    pub fn screen_to_page(&self, screen_x: f32, screen_y: f32) -> PageCoordinate {
        let zoom_scale = self.viewport.zoom_level as f32 / 100.0;
        PageCoordinate::new(
            (screen_x + self.viewport.x) / zoom_scale,
            (screen_y + self.viewport.y) / zoom_scale,
        )
    }

    /// Get the currently selected annotation ID
    pub fn selected_annotation(&self) -> Option<AnnotationId> {
        self.selected_annotation
    }

    /// Set the selected annotation
    pub fn set_selected_annotation(&mut self, id: Option<AnnotationId>) {
        self.selected_annotation = id;
    }

    /// Get the hit test tolerance in page coordinates
    pub fn hit_test_tolerance(&self) -> f32 {
        self.hit_test_tolerance
    }

    /// Set the hit test tolerance in page coordinates
    pub fn set_hit_test_tolerance(&mut self, tolerance: f32) {
        self.hit_test_tolerance = tolerance;
    }

    /// Perform hit test on annotations at current mouse position
    ///
    /// Returns the topmost annotation at the mouse position, or None if no hit.
    /// This should be called with an AnnotationCollection to test against.
    pub fn hit_test_at_mouse(
        &self,
        annotations: &AnnotationCollection,
    ) -> Option<AnnotationId> {
        let page_coord = self.screen_to_page(self.mouse_position.0, self.mouse_position.1);
        let hits = annotations.hit_test(
            self.viewport.page_index,
            &page_coord,
            self.hit_test_tolerance,
        );

        // Return the topmost annotation (first in the list, which is sorted by layer descending)
        hits.first().map(|annotation| annotation.id())
    }

    /// Handle mouse click for annotation selection
    ///
    /// Call this on mouse down when not panning (e.g., when Ctrl/Cmd is not pressed).
    /// Returns true if an annotation was selected, false otherwise.
    pub fn handle_annotation_selection(
        &mut self,
        annotations: &AnnotationCollection,
    ) -> bool {
        if let Some(hit_id) = self.hit_test_at_mouse(annotations) {
            self.selected_annotation = Some(hit_id);
            true
        } else {
            self.selected_annotation = None;
            false
        }
    }

    /// Get manipulation handles for the currently selected annotation
    ///
    /// Returns None if no annotation is selected.
    pub fn get_selection_handles(
        &self,
        annotations: &AnnotationCollection,
    ) -> Option<Vec<ManipulationHandle>> {
        if let Some(annotation_id) = self.selected_annotation {
            if let Some(annotation) = annotations.get(annotation_id) {
                return Some(generate_handles(annotation, self.handle_size));
            }
        }
        None
    }

    /// Start handle manipulation (call on mouse down over a handle)
    ///
    /// Returns true if manipulation started, false otherwise.
    pub fn start_handle_manipulation(
        &mut self,
        annotations: &AnnotationCollection,
    ) -> bool {
        // Only allow handle manipulation if an annotation is selected
        if let Some(annotation_id) = self.selected_annotation {
            if let Some(annotation) = annotations.get(annotation_id) {
                let handles = generate_handles(annotation, self.handle_size);
                let page_coord =
                    self.screen_to_page(self.mouse_position.0, self.mouse_position.1);

                // Check if mouse is over any handle
                for handle in handles {
                    if handle.hit_test(&page_coord, self.hit_test_tolerance) {
                        // Start manipulation
                        self.manipulation_state = Some(ManipulationState::new(
                            annotation_id,
                            handle.handle_type,
                            annotation.geometry().clone(),
                            page_coord,
                        ));
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Update active handle manipulation (call on mouse move while dragging)
    pub fn update_handle_manipulation(&mut self) {
        let page_coord = self.screen_to_page(self.mouse_position.0, self.mouse_position.1);
        if let Some(ref mut state) = self.manipulation_state {
            state.update_position(page_coord);
        }
    }

    /// End handle manipulation (call on mouse up)
    ///
    /// Returns the new geometry if manipulation was active, None otherwise.
    pub fn end_handle_manipulation(
        &mut self,
    ) -> Option<(AnnotationId, pdf_editor_core::AnnotationGeometry)> {
        if let Some(state) = self.manipulation_state.take() {
            let new_geometry = state.calculate_new_geometry();
            return Some((state.annotation_id, new_geometry));
        }
        None
    }

    /// Check if currently manipulating a handle
    pub fn is_manipulating(&self) -> bool {
        self.manipulation_state.is_some()
    }

    /// Get the current manipulation state (for preview rendering)
    pub fn manipulation_state(&self) -> Option<&ManipulationState> {
        self.manipulation_state.as_ref()
    }

    /// Set the handle size in page coordinates
    pub fn set_handle_size(&mut self, size: f32) {
        self.handle_size = size;
    }

    /// Get the handle size in page coordinates
    pub fn handle_size(&self) -> f32 {
        self.handle_size
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

        // Zoom in from 100% with larger delta to reach next discrete level
        handler.on_mouse_wheel(2.0); // 100 * 1.1^2 = 121, snaps to 125%

        // Target zoom should snap to 125% (next discrete level)
        assert_eq!(handler.zoom_state.target_zoom, 125);
    }

    #[test]
    fn test_zoom_clamping() {
        let mut handler = InputHandler::new(1024.0, 768.0);

        // Try to zoom beyond max - should snap to 400%
        handler.set_zoom_level(500);
        assert_eq!(handler.zoom_state.target_zoom, 400);

        // Try to zoom below min - should snap to 25%
        handler.set_zoom_level(10);
        assert_eq!(handler.zoom_state.target_zoom, 25);

        // Arbitrary value should snap to nearest discrete level
        handler.set_zoom_level(135);
        assert_eq!(handler.zoom_state.target_zoom, 125); // Closer to 125 than 150

        handler.set_zoom_level(140);
        assert_eq!(handler.zoom_state.target_zoom, 150); // Closer to 150 than 125
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

    #[test]
    fn test_snap_to_discrete_zoom() {
        // Test snapping to discrete levels
        assert_eq!(snap_to_discrete_zoom(25), 25);
        assert_eq!(snap_to_discrete_zoom(50), 50);
        assert_eq!(snap_to_discrete_zoom(100), 100);
        assert_eq!(snap_to_discrete_zoom(400), 400);

        // Test snapping intermediate values
        // Midpoints: 25-50: 37.5, 50-75: 62.5, 75-100: 87.5, 100-125: 112.5, 125-150: 137.5, 150-200: 175, 200-300: 250
        assert_eq!(snap_to_discrete_zoom(37), 25); // Below midpoint 37.5
        assert_eq!(snap_to_discrete_zoom(38), 50); // Above midpoint 37.5
        assert_eq!(snap_to_discrete_zoom(62), 50); // Below midpoint 62.5
        assert_eq!(snap_to_discrete_zoom(63), 75); // Above midpoint 62.5
        assert_eq!(snap_to_discrete_zoom(87), 75); // Below midpoint 87.5
        assert_eq!(snap_to_discrete_zoom(88), 100); // Above midpoint 87.5
        assert_eq!(snap_to_discrete_zoom(112), 100); // Below midpoint 112.5
        assert_eq!(snap_to_discrete_zoom(113), 125); // Above midpoint 112.5
        assert_eq!(snap_to_discrete_zoom(137), 125); // Below midpoint 137.5
        assert_eq!(snap_to_discrete_zoom(138), 150); // Above midpoint 137.5
        assert_eq!(snap_to_discrete_zoom(175), 150); // At midpoint, snaps to first (150)
        assert_eq!(snap_to_discrete_zoom(250), 200); // At midpoint, snaps to first (200)

        // Test out-of-range values
        assert_eq!(snap_to_discrete_zoom(10), 25);
        assert_eq!(snap_to_discrete_zoom(500), 400);
    }

    #[test]
    fn test_mouse_wheel_snaps_to_discrete_levels() {
        let mut handler = InputHandler::new(1024.0, 768.0);
        handler.on_mouse_move(512.0, 384.0);

        // Starting at 100%, single wheel notch gives 110% which snaps back to 100%
        // Need larger delta to reach next level
        handler.on_mouse_wheel(2.0); // 100 * 1.1^2 = 121, snaps to 125%
        assert_eq!(handler.zoom_state.target_zoom, 125);

        // Simulate reaching 125%
        handler.viewport.zoom_level = 125;
        handler.zoom_state.target_zoom = 125;

        // Wheel up again: 125 * 1.1^2 = 151.25, snaps to 150%
        handler.on_mouse_wheel(2.0);
        assert_eq!(handler.zoom_state.target_zoom, 150);

        // Simulate reaching 150%
        handler.viewport.zoom_level = 150;
        handler.zoom_state.target_zoom = 150;

        // Wheel down: 150 * 0.9^2 = 121.5, snaps to 125%
        handler.on_mouse_wheel(-2.0);
        assert_eq!(handler.zoom_state.target_zoom, 125);
    }

    #[test]
    fn test_wheel_zoom_skips_unchanged_discrete_level() {
        let mut handler = InputHandler::new(1024.0, 768.0);
        handler.on_mouse_move(512.0, 384.0);

        // Set to 100%
        handler.viewport.zoom_level = 100;
        handler.zoom_state.target_zoom = 100;

        // Small wheel movement that would normally give 110% but snaps to 125%
        handler.on_mouse_wheel(0.5);
        let first_target = handler.zoom_state.target_zoom;

        // If wheel scroll results in the same discrete level, target shouldn't change
        handler.viewport.zoom_level = first_target;
        handler.zoom_state.target_zoom = first_target;

        let prev_target = handler.zoom_state.target_zoom;
        handler.on_mouse_wheel(0.001); // Tiny scroll that doesn't reach next level
        assert_eq!(handler.zoom_state.target_zoom, prev_target);
    }

    #[test]
    fn test_all_discrete_zoom_levels_accessible() {
        let mut handler = InputHandler::new(1024.0, 768.0);

        // Test zoom_in progression through all levels
        let expected_levels = [25, 50, 75, 100, 125, 150, 200, 300, 400];

        for i in 0..expected_levels.len() - 1 {
            handler.viewport.zoom_level = expected_levels[i];
            handler.zoom_in();
            assert_eq!(
                handler.zoom_state.target_zoom,
                expected_levels[i + 1],
                "zoom_in from {}% should target {}%",
                expected_levels[i],
                expected_levels[i + 1]
            );
        }

        // Test zoom_out progression through all levels
        for i in (1..expected_levels.len()).rev() {
            handler.viewport.zoom_level = expected_levels[i];
            handler.zoom_out();
            assert_eq!(
                handler.zoom_state.target_zoom,
                expected_levels[i - 1],
                "zoom_out from {}% should target {}%",
                expected_levels[i],
                expected_levels[i - 1]
            );
        }
    }

    #[test]
    fn test_zoom_in_at_max_stays_at_max() {
        let mut handler = InputHandler::new(1024.0, 768.0);
        handler.viewport.zoom_level = 400;

        handler.zoom_in();
        assert_eq!(handler.zoom_state.target_zoom, 400);
    }

    #[test]
    fn test_zoom_out_at_min_stays_at_min() {
        let mut handler = InputHandler::new(1024.0, 768.0);
        handler.viewport.zoom_level = 25;

        handler.zoom_out();
        assert_eq!(handler.zoom_state.target_zoom, 25);
    }

    #[test]
    fn test_screen_to_page_at_100_zoom() {
        let handler = InputHandler::new(1024.0, 768.0);
        // At 100% zoom with no pan, screen coords should equal page coords
        let page_coord = handler.screen_to_page(100.0, 200.0);
        assert!((page_coord.x - 100.0).abs() < 0.001);
        assert!((page_coord.y - 200.0).abs() < 0.001);
    }

    #[test]
    fn test_screen_to_page_at_200_zoom() {
        let mut handler = InputHandler::new(1024.0, 768.0);
        handler.viewport.zoom_level = 200; // 2x zoom

        // At 2x zoom, screen pixels are half the size in page coordinates
        let page_coord = handler.screen_to_page(100.0, 200.0);
        assert!((page_coord.x - 50.0).abs() < 0.001);
        assert!((page_coord.y - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_screen_to_page_with_pan() {
        let mut handler = InputHandler::new(1024.0, 768.0);
        handler.viewport.x = 50.0; // Pan offset
        handler.viewport.y = 100.0;

        // With pan, page coordinates should account for viewport offset
        let page_coord = handler.screen_to_page(0.0, 0.0);
        assert!((page_coord.x - 50.0).abs() < 0.001);
        assert!((page_coord.y - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_screen_to_page_with_zoom_and_pan() {
        let mut handler = InputHandler::new(1024.0, 768.0);
        handler.viewport.zoom_level = 200; // 2x zoom
        handler.viewport.x = 100.0; // Pan offset
        handler.viewport.y = 200.0;

        // Screen point (0, 0) with 2x zoom and pan
        let page_coord = handler.screen_to_page(0.0, 0.0);
        // (0 + 100) / 2.0 = 50.0
        // (0 + 200) / 2.0 = 100.0
        assert!((page_coord.x - 50.0).abs() < 0.001);
        assert!((page_coord.y - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_hit_test_tolerance() {
        let mut handler = InputHandler::new(1024.0, 768.0);
        assert_eq!(handler.hit_test_tolerance(), 5.0);

        handler.set_hit_test_tolerance(10.0);
        assert_eq!(handler.hit_test_tolerance(), 10.0);
    }

    #[test]
    fn test_selected_annotation() {
        use pdf_editor_core::annotation::AnnotationId;

        let mut handler = InputHandler::new(1024.0, 768.0);
        assert_eq!(handler.selected_annotation(), None);

        let id = AnnotationId::new_v4();
        handler.set_selected_annotation(Some(id));
        assert_eq!(handler.selected_annotation(), Some(id));

        handler.set_selected_annotation(None);
        assert_eq!(handler.selected_annotation(), None);
    }

    #[test]
    fn test_hit_test_at_mouse() {
        use pdf_editor_core::annotation::{
            Annotation, AnnotationCollection, AnnotationGeometry, AnnotationStyle,
        };

        let mut handler = InputHandler::new(1024.0, 768.0);
        let mut collection = AnnotationCollection::new();

        // Create a line annotation at (100, 100) to (200, 200) on page 0
        let geometry = AnnotationGeometry::Line {
            start: PageCoordinate::new(100.0, 100.0),
            end: PageCoordinate::new(200.0, 200.0),
        };
        let annotation = Annotation::new(0, geometry, AnnotationStyle::new());
        let annotation_id = annotation.id();
        collection.add(annotation);

        // Position mouse at the midpoint of the line (150, 150)
        handler.on_mouse_move(150.0, 150.0);

        // Should hit the annotation
        let hit = handler.hit_test_at_mouse(&collection);
        assert_eq!(hit, Some(annotation_id));

        // Move mouse away from the line
        handler.on_mouse_move(500.0, 500.0);

        // Should not hit anything
        let hit = handler.hit_test_at_mouse(&collection);
        assert_eq!(hit, None);
    }

    #[test]
    fn test_handle_annotation_selection() {
        use pdf_editor_core::annotation::{
            Annotation, AnnotationCollection, AnnotationGeometry, AnnotationStyle,
        };

        let mut handler = InputHandler::new(1024.0, 768.0);
        let mut collection = AnnotationCollection::new();

        // Create a rectangle annotation
        let geometry = AnnotationGeometry::Rectangle {
            top_left: PageCoordinate::new(50.0, 50.0),
            bottom_right: PageCoordinate::new(150.0, 150.0),
        };
        let annotation = Annotation::new(0, geometry, AnnotationStyle::new());
        let annotation_id = annotation.id();
        collection.add(annotation);

        // Position mouse inside the rectangle
        handler.on_mouse_move(100.0, 100.0);

        // Should select the annotation
        let selected = handler.handle_annotation_selection(&collection);
        assert!(selected);
        assert_eq!(handler.selected_annotation(), Some(annotation_id));

        // Move mouse outside the rectangle
        handler.on_mouse_move(200.0, 200.0);

        // Should deselect
        let selected = handler.handle_annotation_selection(&collection);
        assert!(!selected);
        assert_eq!(handler.selected_annotation(), None);
    }

    #[test]
    fn test_hit_test_with_multiple_layers() {
        use pdf_editor_core::annotation::{
            Annotation, AnnotationCollection, AnnotationGeometry, AnnotationStyle,
        };

        let mut handler = InputHandler::new(1024.0, 768.0);
        let mut collection = AnnotationCollection::new();

        // Create two overlapping annotations at the same location
        let geometry1 = AnnotationGeometry::Circle {
            center: PageCoordinate::new(100.0, 100.0),
            radius: 50.0,
        };
        let mut annotation1 = Annotation::new(0, geometry1, AnnotationStyle::new());
        annotation1.set_layer(1);

        let geometry2 = AnnotationGeometry::Circle {
            center: PageCoordinate::new(100.0, 100.0),
            radius: 50.0,
        };
        let mut annotation2 = Annotation::new(0, geometry2, AnnotationStyle::new());
        annotation2.set_layer(2); // Higher layer (on top)
        let id2 = annotation2.id();

        collection.add(annotation1);
        collection.add(annotation2);

        // Position mouse at the center
        handler.on_mouse_move(100.0, 100.0);

        // Should hit the topmost annotation (layer 2)
        let hit = handler.hit_test_at_mouse(&collection);
        assert_eq!(hit, Some(id2));
    }

    #[test]
    fn test_get_selection_handles() {
        use pdf_editor_core::annotation::{
            Annotation, AnnotationCollection, AnnotationGeometry, AnnotationStyle,
        };

        let mut handler = InputHandler::new(1024.0, 768.0);
        let mut collection = AnnotationCollection::new();

        // Create a rectangle annotation
        let geometry = AnnotationGeometry::Rectangle {
            top_left: PageCoordinate::new(50.0, 50.0),
            bottom_right: PageCoordinate::new(150.0, 150.0),
        };
        let annotation = Annotation::new(0, geometry, AnnotationStyle::new());
        let annotation_id = annotation.id();
        collection.add(annotation);

        // No handles when nothing is selected
        assert!(handler.get_selection_handles(&collection).is_none());

        // Select the annotation
        handler.set_selected_annotation(Some(annotation_id));

        // Should get handles for rectangle (9 handles: 4 corners + 4 edges + 1 rotation)
        let handles = handler.get_selection_handles(&collection);
        assert!(handles.is_some());
        assert_eq!(handles.unwrap().len(), 9);
    }

    #[test]
    fn test_start_handle_manipulation() {
        use pdf_editor_core::annotation::{
            Annotation, AnnotationCollection, AnnotationGeometry, AnnotationStyle,
        };

        let mut handler = InputHandler::new(1024.0, 768.0);
        let mut collection = AnnotationCollection::new();

        // Create a line annotation
        let geometry = AnnotationGeometry::Line {
            start: PageCoordinate::new(0.0, 0.0),
            end: PageCoordinate::new(100.0, 100.0),
        };
        let annotation = Annotation::new(0, geometry, AnnotationStyle::new());
        let annotation_id = annotation.id();
        collection.add(annotation);

        // Select the annotation
        handler.set_selected_annotation(Some(annotation_id));

        // Position mouse at the end point handle (100, 100)
        handler.on_mouse_move(100.0, 100.0);

        // Start manipulation
        let started = handler.start_handle_manipulation(&collection);
        assert!(started);
        assert!(handler.is_manipulating());
    }

    #[test]
    fn test_handle_manipulation_flow() {
        use pdf_editor_core::annotation::{
            Annotation, AnnotationCollection, AnnotationGeometry, AnnotationStyle,
        };

        let mut handler = InputHandler::new(1024.0, 768.0);
        let mut collection = AnnotationCollection::new();

        // Create a rectangle annotation
        let geometry = AnnotationGeometry::Rectangle {
            top_left: PageCoordinate::new(50.0, 50.0),
            bottom_right: PageCoordinate::new(150.0, 150.0),
        };
        let annotation = Annotation::new(0, geometry, AnnotationStyle::new());
        let annotation_id = annotation.id();
        collection.add(annotation);

        // Select the annotation
        handler.set_selected_annotation(Some(annotation_id));

        // Position mouse at bottom-right handle (150, 150)
        handler.on_mouse_move(150.0, 150.0);

        // Start manipulation
        assert!(handler.start_handle_manipulation(&collection));

        // Drag to (200, 200)
        handler.on_mouse_move(200.0, 200.0);
        handler.update_handle_manipulation();

        // End manipulation
        let result = handler.end_handle_manipulation();
        assert!(result.is_some());

        let (_, new_geometry) = result.unwrap();
        if let AnnotationGeometry::Rectangle {
            top_left: _,
            bottom_right,
        } = new_geometry
        {
            // Bottom-right should be moved to (200, 200)
            assert!((bottom_right.x - 200.0).abs() < 0.001);
            assert!((bottom_right.y - 200.0).abs() < 0.001);
        } else {
            panic!("Expected Rectangle geometry");
        }
    }

    #[test]
    fn test_handle_size_configuration() {
        let mut handler = InputHandler::new(1024.0, 768.0);
        assert_eq!(handler.handle_size(), 6.0);

        handler.set_handle_size(8.0);
        assert_eq!(handler.handle_size(), 8.0);
    }
}
