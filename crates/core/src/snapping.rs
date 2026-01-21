//! Snapping system for precision measurements
//!
//! Provides intelligent snapping guides to help users align measurements
//! with existing geometry, grid points, and common angles.

use crate::annotation::{AnnotationCollection, AnnotationGeometry, PageCoordinate};
use crate::measurement::MeasurementCollection;

/// Snap target types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SnapType {
    /// Snap to a point (endpoint, center, etc.)
    Point,
    /// Snap to align horizontally with another point
    Horizontal,
    /// Snap to align vertically with another point
    Vertical,
    /// Snap to a grid intersection
    Grid,
    /// Snap to a common angle (0°, 45°, 90°, etc.)
    Angle,
}

/// A snap target candidate
#[derive(Debug, Clone)]
pub struct SnapTarget {
    /// Type of snap
    pub snap_type: SnapType,

    /// Target position to snap to
    pub target_position: PageCoordinate,

    /// Source position (the point we're snapping from, for guide rendering)
    pub source_position: PageCoordinate,

    /// Distance from original position to snap target
    pub distance: f32,

    /// Priority (lower is higher priority)
    pub priority: u32,
}

impl SnapTarget {
    /// Create a new snap target
    pub fn new(
        snap_type: SnapType,
        target_position: PageCoordinate,
        source_position: PageCoordinate,
        distance: f32,
    ) -> Self {
        let priority = match snap_type {
            SnapType::Point => 0, // Highest priority
            SnapType::Horizontal => 1,
            SnapType::Vertical => 1,
            SnapType::Angle => 2,
            SnapType::Grid => 3, // Lowest priority
        };

        Self {
            snap_type,
            target_position,
            source_position,
            distance,
            priority,
        }
    }
}

/// Configuration for snapping behavior
#[derive(Debug, Clone)]
pub struct SnapConfig {
    /// Enable/disable snapping system
    pub enabled: bool,

    /// Snap threshold in page coordinates (points)
    pub threshold: f32,

    /// Enable point snapping (endpoints, centers)
    pub snap_to_points: bool,

    /// Enable horizontal/vertical alignment
    pub snap_to_alignment: bool,

    /// Enable grid snapping
    pub snap_to_grid: bool,

    /// Grid spacing in page coordinates (if grid snapping enabled)
    pub grid_spacing: f32,

    /// Enable angle snapping
    pub snap_to_angles: bool,

    /// Angle snap increments in degrees (e.g., 15.0 for 15° increments)
    pub angle_increment: f32,
}

impl Default for SnapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 10.0, // 10 points (about 3.5mm at 72 DPI)
            snap_to_points: true,
            snap_to_alignment: true,
            snap_to_grid: false, // Disabled by default
            grid_spacing: 10.0,
            snap_to_angles: true,
            angle_increment: 15.0, // 15° increments
        }
    }
}

/// Snapping engine for calculating snap targets
#[derive(Debug)]
pub struct SnapEngine {
    config: SnapConfig,
}

impl SnapEngine {
    /// Create a new snap engine with default configuration
    pub fn new() -> Self {
        Self {
            config: SnapConfig::default(),
        }
    }

    /// Create a snap engine with custom configuration
    pub fn with_config(config: SnapConfig) -> Self {
        Self { config }
    }

    /// Get current configuration
    pub fn config(&self) -> &SnapConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: SnapConfig) {
        self.config = config;
    }

    /// Calculate snap targets for a point being manipulated
    ///
    /// Returns the best snap target if one is within threshold, or None
    pub fn calculate_snap(
        &self,
        position: &PageCoordinate,
        page_index: u16,
        annotations: &AnnotationCollection,
        measurements: &MeasurementCollection,
        exclude_annotation_id: Option<crate::annotation::AnnotationId>,
        reference_point: Option<&PageCoordinate>,
    ) -> Option<SnapTarget> {
        if !self.config.enabled {
            return None;
        }

        let mut candidates = Vec::new();

        // Collect snap candidates from annotations and measurements
        if self.config.snap_to_points || self.config.snap_to_alignment {
            self.collect_geometry_snaps(
                position,
                page_index,
                annotations,
                exclude_annotation_id,
                &mut candidates,
            );
            self.collect_measurement_snaps(position, page_index, measurements, &mut candidates);
        }

        // Add grid snaps
        if self.config.snap_to_grid {
            self.collect_grid_snaps(position, &mut candidates);
        }

        // Add angle snaps (if we have a reference point for angle calculation)
        if self.config.snap_to_angles {
            if let Some(ref_point) = reference_point {
                self.collect_angle_snaps(position, ref_point, &mut candidates);
            }
        }

        // Filter by threshold and select best candidate
        self.select_best_snap(candidates)
    }

    /// Collect snap candidates from annotation geometry
    fn collect_geometry_snaps(
        &self,
        position: &PageCoordinate,
        page_index: u16,
        annotations: &AnnotationCollection,
        exclude_annotation_id: Option<crate::annotation::AnnotationId>,
        candidates: &mut Vec<SnapTarget>,
    ) {
        let page_annotations = annotations.get_page_annotations(page_index);

        for annotation in page_annotations {
            // Skip the annotation being manipulated
            if exclude_annotation_id.is_some() && exclude_annotation_id.unwrap() == annotation.id()
            {
                continue;
            }

            // Extract snap points from geometry
            let snap_points = self.extract_snap_points(annotation.geometry());

            for snap_point in snap_points {
                let distance = position.distance_to(&snap_point);

                // Point snap
                if self.config.snap_to_points && distance <= self.config.threshold {
                    candidates.push(SnapTarget::new(
                        SnapType::Point,
                        snap_point,
                        *position,
                        distance,
                    ));
                }

                // Alignment snaps
                if self.config.snap_to_alignment {
                    // Horizontal alignment
                    let h_distance = (position.y - snap_point.y).abs();
                    if h_distance <= self.config.threshold {
                        candidates.push(SnapTarget::new(
                            SnapType::Horizontal,
                            PageCoordinate::new(position.x, snap_point.y),
                            *position,
                            h_distance,
                        ));
                    }

                    // Vertical alignment
                    let v_distance = (position.x - snap_point.x).abs();
                    if v_distance <= self.config.threshold {
                        candidates.push(SnapTarget::new(
                            SnapType::Vertical,
                            PageCoordinate::new(snap_point.x, position.y),
                            *position,
                            v_distance,
                        ));
                    }
                }
            }
        }
    }

    /// Collect snap candidates from measurements
    fn collect_measurement_snaps(
        &self,
        position: &PageCoordinate,
        page_index: u16,
        measurements: &MeasurementCollection,
        candidates: &mut Vec<SnapTarget>,
    ) {
        let page_measurements = measurements.get_for_page(page_index);

        for measurement in page_measurements {
            let snap_points = self.extract_snap_points(measurement.geometry());

            for snap_point in snap_points {
                let distance = position.distance_to(&snap_point);

                if self.config.snap_to_points && distance <= self.config.threshold {
                    candidates.push(SnapTarget::new(
                        SnapType::Point,
                        snap_point,
                        *position,
                        distance,
                    ));
                }

                // Alignment snaps
                if self.config.snap_to_alignment {
                    let h_distance = (position.y - snap_point.y).abs();
                    if h_distance <= self.config.threshold {
                        candidates.push(SnapTarget::new(
                            SnapType::Horizontal,
                            PageCoordinate::new(position.x, snap_point.y),
                            *position,
                            h_distance,
                        ));
                    }

                    let v_distance = (position.x - snap_point.x).abs();
                    if v_distance <= self.config.threshold {
                        candidates.push(SnapTarget::new(
                            SnapType::Vertical,
                            PageCoordinate::new(snap_point.x, position.y),
                            *position,
                            v_distance,
                        ));
                    }
                }
            }
        }
    }

    /// Extract snap points from geometry
    fn extract_snap_points(&self, geometry: &AnnotationGeometry) -> Vec<PageCoordinate> {
        let mut points = Vec::new();

        match geometry {
            AnnotationGeometry::Line { start, end } | AnnotationGeometry::Arrow { start, end } => {
                points.push(*start);
                points.push(*end);
                // Add midpoint
                points.push(PageCoordinate::new(
                    (start.x + end.x) / 2.0,
                    (start.y + end.y) / 2.0,
                ));
            }

            AnnotationGeometry::Rectangle {
                top_left,
                bottom_right,
            } => {
                points.push(*top_left);
                points.push(*bottom_right);
                points.push(PageCoordinate::new(bottom_right.x, top_left.y));
                points.push(PageCoordinate::new(top_left.x, bottom_right.y));
                // Add center
                points.push(PageCoordinate::new(
                    (top_left.x + bottom_right.x) / 2.0,
                    (top_left.y + bottom_right.y) / 2.0,
                ));
            }

            AnnotationGeometry::Circle { center, .. }
            | AnnotationGeometry::Ellipse { center, .. } => {
                points.push(*center);
            }

            AnnotationGeometry::Polyline {
                points: poly_points,
            }
            | AnnotationGeometry::Polygon {
                points: poly_points,
            }
            | AnnotationGeometry::Freehand {
                points: poly_points,
            } => {
                points.extend(poly_points.iter().copied());
            }

            AnnotationGeometry::Text { position, .. } => {
                points.push(*position);
            }

            AnnotationGeometry::Note { position, .. } => {
                points.push(*position);
            }
        }

        points
    }

    /// Collect grid snap candidates
    fn collect_grid_snaps(&self, position: &PageCoordinate, candidates: &mut Vec<SnapTarget>) {
        let spacing = self.config.grid_spacing;

        // Find nearest grid intersection
        let grid_x = (position.x / spacing).round() * spacing;
        let grid_y = (position.y / spacing).round() * spacing;
        let grid_point = PageCoordinate::new(grid_x, grid_y);

        let distance = position.distance_to(&grid_point);

        if distance <= self.config.threshold {
            candidates.push(SnapTarget::new(
                SnapType::Grid,
                grid_point,
                *position,
                distance,
            ));
        }
    }

    /// Collect angle snap candidates
    fn collect_angle_snaps(
        &self,
        position: &PageCoordinate,
        reference_point: &PageCoordinate,
        candidates: &mut Vec<SnapTarget>,
    ) {
        let dx = position.x - reference_point.x;
        let dy = position.y - reference_point.y;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance < 1.0 {
            return; // Too close to reference point
        }

        // Current angle in degrees
        let current_angle = dy.atan2(dx).to_degrees();

        // Find nearest angle increment
        let increment = self.config.angle_increment;
        let snapped_angle = (current_angle / increment).round() * increment;

        // Convert back to radians
        let snapped_rad = snapped_angle.to_radians();

        // Calculate snapped position
        let snapped_position = PageCoordinate::new(
            reference_point.x + distance * snapped_rad.cos(),
            reference_point.y + distance * snapped_rad.sin(),
        );

        let snap_distance = position.distance_to(&snapped_position);

        if snap_distance <= self.config.threshold {
            candidates.push(SnapTarget::new(
                SnapType::Angle,
                snapped_position,
                *position,
                snap_distance,
            ));
        }
    }

    /// Select the best snap target from candidates
    fn select_best_snap(&self, mut candidates: Vec<SnapTarget>) -> Option<SnapTarget> {
        if candidates.is_empty() {
            return None;
        }

        // Sort by priority first, then by distance
        candidates.sort_by(|a, b| {
            a.priority.cmp(&b.priority).then(
                a.distance
                    .partial_cmp(&b.distance)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });

        // Return the best candidate
        candidates.into_iter().next()
    }
}

impl Default for SnapEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::{Annotation, AnnotationStyle};

    #[test]
    fn test_extract_snap_points_line() {
        let engine = SnapEngine::new();
        let geometry = AnnotationGeometry::Line {
            start: PageCoordinate::new(0.0, 0.0),
            end: PageCoordinate::new(100.0, 100.0),
        };

        let points = engine.extract_snap_points(&geometry);
        assert_eq!(points.len(), 3); // start, end, midpoint
    }

    #[test]
    fn test_extract_snap_points_rectangle() {
        let engine = SnapEngine::new();
        let geometry = AnnotationGeometry::Rectangle {
            top_left: PageCoordinate::new(0.0, 0.0),
            bottom_right: PageCoordinate::new(100.0, 100.0),
        };

        let points = engine.extract_snap_points(&geometry);
        assert_eq!(points.len(), 5); // 4 corners + center
    }

    #[test]
    fn test_grid_snap() {
        let config = SnapConfig {
            snap_to_grid: true,
            grid_spacing: 10.0,
            threshold: 5.0,
            ..Default::default()
        };

        let engine = SnapEngine::with_config(config);
        let annotations = AnnotationCollection::new();
        let measurements = MeasurementCollection::new();

        // Point close to grid intersection (10, 10)
        let position = PageCoordinate::new(12.0, 11.0);

        let snap = engine.calculate_snap(&position, 0, &annotations, &measurements, None, None);

        assert!(snap.is_some());
        let snap = snap.unwrap();
        assert_eq!(snap.snap_type, SnapType::Grid);
        assert!((snap.target_position.x - 10.0).abs() < 0.001);
        assert!((snap.target_position.y - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_point_snap() {
        let config = SnapConfig::default();
        let engine = SnapEngine::with_config(config);

        let mut annotations = AnnotationCollection::new();
        let geometry = AnnotationGeometry::Line {
            start: PageCoordinate::new(0.0, 0.0),
            end: PageCoordinate::new(100.0, 100.0),
        };
        let annotation = Annotation::new(0, geometry, AnnotationStyle::new());
        annotations.add(annotation);

        let measurements = MeasurementCollection::new();

        // Point close to line endpoint
        let position = PageCoordinate::new(102.0, 101.0);

        let snap = engine.calculate_snap(&position, 0, &annotations, &measurements, None, None);

        assert!(snap.is_some());
        let snap = snap.unwrap();
        assert_eq!(snap.snap_type, SnapType::Point);
        assert!((snap.target_position.x - 100.0).abs() < 0.001);
        assert!((snap.target_position.y - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_angle_snap() {
        let config = SnapConfig {
            snap_to_angles: true,
            angle_increment: 45.0,
            threshold: 10.0,
            ..Default::default()
        };

        let engine = SnapEngine::with_config(config);
        let annotations = AnnotationCollection::new();
        let measurements = MeasurementCollection::new();

        let reference = PageCoordinate::new(0.0, 0.0);

        // Point at approximately 47° (should snap to 45°)
        let position = PageCoordinate::new(100.0, 105.0);

        let snap = engine.calculate_snap(
            &position,
            0,
            &annotations,
            &measurements,
            None,
            Some(&reference),
        );

        assert!(snap.is_some());
        let snap = snap.unwrap();
        assert_eq!(snap.snap_type, SnapType::Angle);
    }

    #[test]
    fn test_snap_disabled() {
        let config = SnapConfig {
            enabled: false,
            ..Default::default()
        };

        let engine = SnapEngine::with_config(config);
        let annotations = AnnotationCollection::new();
        let measurements = MeasurementCollection::new();

        let position = PageCoordinate::new(12.0, 11.0);

        let snap = engine.calculate_snap(&position, 0, &annotations, &measurements, None, None);

        assert!(snap.is_none());
    }
}
