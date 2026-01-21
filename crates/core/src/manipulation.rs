//! Annotation manipulation handles and operations
//!
//! Provides handles for selecting, moving, resizing, and rotating annotations.
//! Handles are rendered as small control points on the annotation's bounding box.

use crate::annotation::{Annotation, AnnotationGeometry, AnnotationId, PageCoordinate};
use crate::snapping::SnapTarget;

/// Type of manipulation handle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleType {
    /// Corner handles for resizing (preserves aspect ratio with shift)
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,

    /// Edge handles for resizing in one dimension
    Top,
    Bottom,
    Left,
    Right,

    /// Rotation handle (typically above the annotation)
    Rotate,

    /// Move handle (center of annotation, or entire bounding box)
    Move,
}

/// Manipulation handle with position and type
#[derive(Debug, Clone, Copy)]
pub struct ManipulationHandle {
    /// Type of handle
    pub handle_type: HandleType,

    /// Position in page coordinates
    pub position: PageCoordinate,

    /// Handle size in page coordinates (radius of hit area)
    pub size: f32,

    /// Associated annotation ID
    pub annotation_id: AnnotationId,
}

impl ManipulationHandle {
    /// Create a new manipulation handle
    pub fn new(
        handle_type: HandleType,
        position: PageCoordinate,
        size: f32,
        annotation_id: AnnotationId,
    ) -> Self {
        Self {
            handle_type,
            position,
            size,
            annotation_id,
        }
    }

    /// Check if a point hits this handle
    pub fn hit_test(&self, point: &PageCoordinate, tolerance: f32) -> bool {
        let hit_radius = self.size + tolerance;
        point.distance_to(&self.position) <= hit_radius
    }
}

/// Generate manipulation handles for an annotation
///
/// Returns a vector of handles based on the annotation's geometry.
/// Different geometry types have different handle configurations.
pub fn generate_handles(annotation: &Annotation, handle_size: f32) -> Vec<ManipulationHandle> {
    let mut handles = Vec::new();
    let annotation_id = annotation.id();

    match annotation.geometry() {
        AnnotationGeometry::Line { start, end } => {
            // Line: handles at both endpoints for repositioning
            handles.push(ManipulationHandle::new(
                HandleType::TopLeft,
                *start,
                handle_size,
                annotation_id,
            ));
            handles.push(ManipulationHandle::new(
                HandleType::BottomRight,
                *end,
                handle_size,
                annotation_id,
            ));
        }

        AnnotationGeometry::Arrow { start, end } => {
            // Arrow: handles at both endpoints
            handles.push(ManipulationHandle::new(
                HandleType::TopLeft,
                *start,
                handle_size,
                annotation_id,
            ));
            handles.push(ManipulationHandle::new(
                HandleType::BottomRight,
                *end,
                handle_size,
                annotation_id,
            ));
        }

        AnnotationGeometry::Rectangle {
            top_left,
            bottom_right,
        } => {
            // Rectangle: 8 handles (4 corners + 4 edges) + rotation
            let top_right = PageCoordinate::new(bottom_right.x, top_left.y);
            let bottom_left = PageCoordinate::new(top_left.x, bottom_right.y);
            let center_x = (top_left.x + bottom_right.x) / 2.0;
            let center_y = (top_left.y + bottom_right.y) / 2.0;

            // Corner handles
            handles.push(ManipulationHandle::new(
                HandleType::TopLeft,
                *top_left,
                handle_size,
                annotation_id,
            ));
            handles.push(ManipulationHandle::new(
                HandleType::TopRight,
                top_right,
                handle_size,
                annotation_id,
            ));
            handles.push(ManipulationHandle::new(
                HandleType::BottomLeft,
                bottom_left,
                handle_size,
                annotation_id,
            ));
            handles.push(ManipulationHandle::new(
                HandleType::BottomRight,
                *bottom_right,
                handle_size,
                annotation_id,
            ));

            // Edge handles
            handles.push(ManipulationHandle::new(
                HandleType::Top,
                PageCoordinate::new(center_x, top_left.y),
                handle_size,
                annotation_id,
            ));
            handles.push(ManipulationHandle::new(
                HandleType::Bottom,
                PageCoordinate::new(center_x, bottom_right.y),
                handle_size,
                annotation_id,
            ));
            handles.push(ManipulationHandle::new(
                HandleType::Left,
                PageCoordinate::new(top_left.x, center_y),
                handle_size,
                annotation_id,
            ));
            handles.push(ManipulationHandle::new(
                HandleType::Right,
                PageCoordinate::new(bottom_right.x, center_y),
                handle_size,
                annotation_id,
            ));

            // Rotation handle above the shape
            let rotation_offset = 30.0; // Distance above the shape
            handles.push(ManipulationHandle::new(
                HandleType::Rotate,
                PageCoordinate::new(center_x, top_left.y - rotation_offset),
                handle_size,
                annotation_id,
            ));
        }

        AnnotationGeometry::Circle { center, radius } => {
            // Circle: 4 cardinal direction handles for radius adjustment
            handles.push(ManipulationHandle::new(
                HandleType::Top,
                PageCoordinate::new(center.x, center.y - radius),
                handle_size,
                annotation_id,
            ));
            handles.push(ManipulationHandle::new(
                HandleType::Bottom,
                PageCoordinate::new(center.x, center.y + radius),
                handle_size,
                annotation_id,
            ));
            handles.push(ManipulationHandle::new(
                HandleType::Left,
                PageCoordinate::new(center.x - radius, center.y),
                handle_size,
                annotation_id,
            ));
            handles.push(ManipulationHandle::new(
                HandleType::Right,
                PageCoordinate::new(center.x + radius, center.y),
                handle_size,
                annotation_id,
            ));
        }

        AnnotationGeometry::Ellipse {
            center,
            radius_x,
            radius_y,
        } => {
            // Ellipse: 4 handles on the axes
            handles.push(ManipulationHandle::new(
                HandleType::Top,
                PageCoordinate::new(center.x, center.y - radius_y),
                handle_size,
                annotation_id,
            ));
            handles.push(ManipulationHandle::new(
                HandleType::Bottom,
                PageCoordinate::new(center.x, center.y + radius_y),
                handle_size,
                annotation_id,
            ));
            handles.push(ManipulationHandle::new(
                HandleType::Left,
                PageCoordinate::new(center.x - radius_x, center.y),
                handle_size,
                annotation_id,
            ));
            handles.push(ManipulationHandle::new(
                HandleType::Right,
                PageCoordinate::new(center.x + radius_x, center.y),
                handle_size,
                annotation_id,
            ));
        }

        AnnotationGeometry::Polyline { points }
        | AnnotationGeometry::Polygon { points }
        | AnnotationGeometry::Freehand { points } => {
            // Polyline/Polygon/Freehand: handle at each point for vertex editing
            for (i, point) in points.iter().enumerate() {
                // Use different handle types to distinguish points
                let handle_type = match i {
                    0 => HandleType::TopLeft,
                    _ if i == points.len() - 1 => HandleType::BottomRight,
                    _ => HandleType::Move,
                };

                handles.push(ManipulationHandle::new(
                    handle_type,
                    *point,
                    handle_size,
                    annotation_id,
                ));
            }
        }

        AnnotationGeometry::Text { position, .. } => {
            // Text: single move handle at position
            handles.push(ManipulationHandle::new(
                HandleType::Move,
                *position,
                handle_size,
                annotation_id,
            ));
        }

        AnnotationGeometry::Note { position, .. } => {
            // Note: single move handle at position
            handles.push(ManipulationHandle::new(
                HandleType::Move,
                *position,
                handle_size,
                annotation_id,
            ));
        }
    }

    handles
}

/// Active manipulation state
#[derive(Debug, Clone)]
pub struct ManipulationState {
    /// ID of annotation being manipulated
    pub annotation_id: AnnotationId,

    /// Type of handle being dragged
    pub handle_type: HandleType,

    /// Original geometry before manipulation started
    pub original_geometry: AnnotationGeometry,

    /// Drag start position in page coordinates
    pub drag_start: PageCoordinate,

    /// Current drag position in page coordinates (before snapping)
    pub current_position: PageCoordinate,

    /// Active snap target (if any)
    pub snap_target: Option<SnapTarget>,
}

impl ManipulationState {
    /// Create a new manipulation state
    pub fn new(
        annotation_id: AnnotationId,
        handle_type: HandleType,
        original_geometry: AnnotationGeometry,
        drag_start: PageCoordinate,
    ) -> Self {
        Self {
            annotation_id,
            handle_type,
            original_geometry,
            drag_start,
            current_position: drag_start,
            snap_target: None,
        }
    }

    /// Update the current drag position
    pub fn update_position(&mut self, position: PageCoordinate) {
        self.current_position = position;
    }

    /// Set the active snap target
    pub fn set_snap_target(&mut self, snap_target: Option<SnapTarget>) {
        self.snap_target = snap_target;
    }

    /// Get the effective position (snapped if snap is active)
    pub fn effective_position(&self) -> PageCoordinate {
        if let Some(ref snap) = self.snap_target {
            snap.target_position
        } else {
            self.current_position
        }
    }

    /// Calculate the new geometry based on current manipulation
    pub fn calculate_new_geometry(&self) -> AnnotationGeometry {
        // Use effective position (with snapping applied)
        let effective_pos = self.effective_position();
        let delta_x = effective_pos.x - self.drag_start.x;
        let delta_y = effective_pos.y - self.drag_start.y;

        match &self.original_geometry {
            AnnotationGeometry::Line { start, end } => {
                match self.handle_type {
                    HandleType::TopLeft => {
                        // Moving start point
                        AnnotationGeometry::Line {
                            start: PageCoordinate::new(start.x + delta_x, start.y + delta_y),
                            end: *end,
                        }
                    }
                    HandleType::BottomRight => {
                        // Moving end point
                        AnnotationGeometry::Line {
                            start: *start,
                            end: PageCoordinate::new(end.x + delta_x, end.y + delta_y),
                        }
                    }
                    _ => self.original_geometry.clone(),
                }
            }

            AnnotationGeometry::Arrow { start, end } => match self.handle_type {
                HandleType::TopLeft => AnnotationGeometry::Arrow {
                    start: PageCoordinate::new(start.x + delta_x, start.y + delta_y),
                    end: *end,
                },
                HandleType::BottomRight => AnnotationGeometry::Arrow {
                    start: *start,
                    end: PageCoordinate::new(end.x + delta_x, end.y + delta_y),
                },
                _ => self.original_geometry.clone(),
            },

            AnnotationGeometry::Rectangle {
                top_left,
                bottom_right,
            } => match self.handle_type {
                HandleType::TopLeft => AnnotationGeometry::Rectangle {
                    top_left: PageCoordinate::new(top_left.x + delta_x, top_left.y + delta_y),
                    bottom_right: *bottom_right,
                },
                HandleType::BottomRight => AnnotationGeometry::Rectangle {
                    top_left: *top_left,
                    bottom_right: PageCoordinate::new(
                        bottom_right.x + delta_x,
                        bottom_right.y + delta_y,
                    ),
                },
                HandleType::TopRight => AnnotationGeometry::Rectangle {
                    top_left: PageCoordinate::new(top_left.x, top_left.y + delta_y),
                    bottom_right: PageCoordinate::new(bottom_right.x + delta_x, bottom_right.y),
                },
                HandleType::BottomLeft => AnnotationGeometry::Rectangle {
                    top_left: PageCoordinate::new(top_left.x + delta_x, top_left.y),
                    bottom_right: PageCoordinate::new(bottom_right.x, bottom_right.y + delta_y),
                },
                HandleType::Top => AnnotationGeometry::Rectangle {
                    top_left: PageCoordinate::new(top_left.x, top_left.y + delta_y),
                    bottom_right: *bottom_right,
                },
                HandleType::Bottom => AnnotationGeometry::Rectangle {
                    top_left: *top_left,
                    bottom_right: PageCoordinate::new(bottom_right.x, bottom_right.y + delta_y),
                },
                HandleType::Left => AnnotationGeometry::Rectangle {
                    top_left: PageCoordinate::new(top_left.x + delta_x, top_left.y),
                    bottom_right: *bottom_right,
                },
                HandleType::Right => AnnotationGeometry::Rectangle {
                    top_left: *top_left,
                    bottom_right: PageCoordinate::new(bottom_right.x + delta_x, bottom_right.y),
                },
                _ => self.original_geometry.clone(),
            },

            AnnotationGeometry::Circle { center, radius: _ } => {
                match self.handle_type {
                    HandleType::Top | HandleType::Bottom | HandleType::Left | HandleType::Right => {
                        // Calculate new radius based on distance from center to effective position
                        let effective_pos = self.effective_position();
                        let new_radius = center.distance_to(&effective_pos);
                        AnnotationGeometry::Circle {
                            center: *center,
                            radius: new_radius,
                        }
                    }
                    _ => self.original_geometry.clone(),
                }
            }

            AnnotationGeometry::Ellipse {
                center,
                radius_x,
                radius_y,
            } => {
                let effective_pos = self.effective_position();
                match self.handle_type {
                    HandleType::Left | HandleType::Right => {
                        let new_radius_x = (effective_pos.x - center.x).abs();
                        AnnotationGeometry::Ellipse {
                            center: *center,
                            radius_x: new_radius_x,
                            radius_y: *radius_y,
                        }
                    }
                    HandleType::Top | HandleType::Bottom => {
                        let new_radius_y = (effective_pos.y - center.y).abs();
                        AnnotationGeometry::Ellipse {
                            center: *center,
                            radius_x: *radius_x,
                            radius_y: new_radius_y,
                        }
                    }
                    _ => self.original_geometry.clone(),
                }
            }

            _ => {
                // For other geometry types, just return original for now
                // Full implementation would handle polyline/polygon vertex editing
                self.original_geometry.clone()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::{Annotation, AnnotationStyle};

    #[test]
    fn test_handle_hit_test() {
        let handle = ManipulationHandle::new(
            HandleType::TopLeft,
            PageCoordinate::new(100.0, 100.0),
            5.0,
            AnnotationId::new_v4(),
        );

        // Point within handle
        assert!(handle.hit_test(&PageCoordinate::new(102.0, 102.0), 2.0));

        // Point outside handle
        assert!(!handle.hit_test(&PageCoordinate::new(120.0, 120.0), 2.0));
    }

    #[test]
    fn test_generate_handles_for_line() {
        let geometry = AnnotationGeometry::Line {
            start: PageCoordinate::new(0.0, 0.0),
            end: PageCoordinate::new(100.0, 100.0),
        };
        let annotation = Annotation::new(0, geometry, AnnotationStyle::new());

        let handles = generate_handles(&annotation, 5.0);
        assert_eq!(handles.len(), 2); // Start and end handles
    }

    #[test]
    fn test_generate_handles_for_rectangle() {
        let geometry = AnnotationGeometry::Rectangle {
            top_left: PageCoordinate::new(0.0, 0.0),
            bottom_right: PageCoordinate::new(100.0, 100.0),
        };
        let annotation = Annotation::new(0, geometry, AnnotationStyle::new());

        let handles = generate_handles(&annotation, 5.0);
        assert_eq!(handles.len(), 9); // 4 corners + 4 edges + 1 rotation
    }

    #[test]
    fn test_generate_handles_for_circle() {
        let geometry = AnnotationGeometry::Circle {
            center: PageCoordinate::new(50.0, 50.0),
            radius: 25.0,
        };
        let annotation = Annotation::new(0, geometry, AnnotationStyle::new());

        let handles = generate_handles(&annotation, 5.0);
        assert_eq!(handles.len(), 4); // 4 cardinal direction handles
    }

    #[test]
    fn test_manipulation_state_line() {
        let geometry = AnnotationGeometry::Line {
            start: PageCoordinate::new(0.0, 0.0),
            end: PageCoordinate::new(100.0, 100.0),
        };

        let mut state = ManipulationState::new(
            AnnotationId::new_v4(),
            HandleType::BottomRight,
            geometry,
            PageCoordinate::new(100.0, 100.0),
        );

        // Move end point to (150, 150)
        state.update_position(PageCoordinate::new(150.0, 150.0));

        let new_geometry = state.calculate_new_geometry();
        if let AnnotationGeometry::Line { start: _, end } = new_geometry {
            assert!((end.x - 150.0).abs() < 0.001);
            assert!((end.y - 150.0).abs() < 0.001);
        } else {
            panic!("Expected Line geometry");
        }
    }

    #[test]
    fn test_manipulation_state_rectangle_corner() {
        let geometry = AnnotationGeometry::Rectangle {
            top_left: PageCoordinate::new(0.0, 0.0),
            bottom_right: PageCoordinate::new(100.0, 100.0),
        };

        let mut state = ManipulationState::new(
            AnnotationId::new_v4(),
            HandleType::BottomRight,
            geometry,
            PageCoordinate::new(100.0, 100.0),
        );

        // Resize to (150, 150)
        state.update_position(PageCoordinate::new(150.0, 150.0));

        let new_geometry = state.calculate_new_geometry();
        if let AnnotationGeometry::Rectangle {
            top_left: _,
            bottom_right,
        } = new_geometry
        {
            assert!((bottom_right.x - 150.0).abs() < 0.001);
            assert!((bottom_right.y - 150.0).abs() < 0.001);
        } else {
            panic!("Expected Rectangle geometry");
        }
    }

    #[test]
    fn test_manipulation_state_circle_radius() {
        let geometry = AnnotationGeometry::Circle {
            center: PageCoordinate::new(100.0, 100.0),
            radius: 25.0,
        };

        let mut state = ManipulationState::new(
            AnnotationId::new_v4(),
            HandleType::Right,
            geometry,
            PageCoordinate::new(125.0, 100.0),
        );

        // Drag to increase radius
        state.update_position(PageCoordinate::new(150.0, 100.0));

        let new_geometry = state.calculate_new_geometry();
        if let AnnotationGeometry::Circle { center: _, radius } = new_geometry {
            assert!((radius - 50.0).abs() < 0.001);
        } else {
            panic!("Expected Circle geometry");
        }
    }
}
