//! Measurement geometry storage and scale system
//!
//! Provides CAD-style measurements with scale-aware calculations.
//! All geometry is stored in page coordinates (PDF coordinate system).

use crate::annotation::{AnnotationGeometry, PageCoordinate};
use std::collections::HashMap;
use std::sync::Arc;

/// Unique identifier for measurements
pub type MeasurementId = uuid::Uuid;

/// Unique identifier for scale systems
pub type ScaleSystemId = uuid::Uuid;

/// Type of scale calibration used
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ScaleType {
    /// Manual scale ratio (e.g., 1:100)
    Manual {
        /// Points per real-world unit
        ratio: f32,
    },
    /// Two-point calibration (user specifies known distance)
    TwoPoint {
        /// First calibration point (page coordinates)
        p1: PageCoordinate,
        /// Second calibration point (page coordinates)
        p2: PageCoordinate,
        /// Known distance between points in real-world units
        distance: f32,
    },
    /// OCR-detected scale (suggestion only)
    OCRDetected {
        /// Confidence score (0.0-1.0)
        confidence: f32,
        /// Detected ratio
        ratio: f32,
    },
}

impl ScaleType {
    /// Calculate the conversion ratio from page coordinates to real-world units
    pub fn ratio(&self) -> f32 {
        match self {
            ScaleType::Manual { ratio } => *ratio,
            ScaleType::TwoPoint { p1, p2, distance } => {
                let page_distance = p1.distance_to(p2);
                if page_distance > 0.0 {
                    page_distance / distance
                } else {
                    1.0 // Fallback to 1:1 if points are identical
                }
            }
            ScaleType::OCRDetected { ratio, .. } => *ratio,
        }
    }

    /// Check if this scale is derived from OCR (less reliable)
    pub fn is_ocr_derived(&self) -> bool {
        matches!(self, ScaleType::OCRDetected { .. })
    }
}

/// Scale system for converting page coordinates to real-world measurements
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScaleSystem {
    /// Unique identifier
    id: ScaleSystemId,
    /// Page this scale applies to (0-based)
    page_index: u16,
    /// Type of scale calibration
    scale_type: ScaleType,
    /// Unit of measurement (e.g., "m", "ft", "mm")
    unit: String,
    /// Optional label for this scale system
    label: Option<String>,
}

impl ScaleSystem {
    /// Create a new scale system
    pub fn new(page_index: u16, scale_type: ScaleType, unit: impl Into<String>) -> Self {
        Self {
            id: MeasurementId::new_v4(),
            page_index,
            scale_type,
            unit: unit.into(),
            label: None,
        }
    }

    /// Create a manual scale system with a ratio
    pub fn manual(page_index: u16, ratio: f32, unit: impl Into<String>) -> Self {
        Self::new(page_index, ScaleType::Manual { ratio }, unit)
    }

    /// Create a two-point calibration scale system
    pub fn two_point(
        page_index: u16,
        p1: PageCoordinate,
        p2: PageCoordinate,
        distance: f32,
        unit: impl Into<String>,
    ) -> Self {
        Self::new(page_index, ScaleType::TwoPoint { p1, p2, distance }, unit)
    }

    /// Create an OCR-detected scale system
    pub fn ocr_detected(
        page_index: u16,
        ratio: f32,
        confidence: f32,
        unit: impl Into<String>,
    ) -> Self {
        Self::new(
            page_index,
            ScaleType::OCRDetected { confidence, ratio },
            unit,
        )
    }

    /// Get the scale system ID
    pub fn id(&self) -> ScaleSystemId {
        self.id
    }

    /// Get the page index
    pub fn page_index(&self) -> u16 {
        self.page_index
    }

    /// Get the measurement unit
    pub fn unit(&self) -> &str {
        &self.unit
    }

    /// Get the scale ratio (page coordinates per real-world unit)
    pub fn ratio(&self) -> f32 {
        self.scale_type.ratio()
    }

    /// Set a label for this scale system
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Get the label
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Convert page coordinate distance to real-world distance
    pub fn to_real_world(&self, page_distance: f32) -> f32 {
        page_distance / self.ratio()
    }

    /// Convert real-world distance to page coordinate distance
    pub fn to_page_coords(&self, real_world_distance: f32) -> f32 {
        real_world_distance * self.ratio()
    }

    /// Check if this scale is reliable (not OCR-derived or has high confidence)
    pub fn is_reliable(&self) -> bool {
        match &self.scale_type {
            ScaleType::Manual { .. } | ScaleType::TwoPoint { .. } => true,
            ScaleType::OCRDetected { confidence, .. } => *confidence > 0.8,
        }
    }
}

/// Type of measurement being performed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeasurementType {
    /// Linear distance between two points or along a path
    Distance,
    /// Area of a closed shape
    Area,
    /// Radius or diameter of a circle
    Radius,
    /// Angle between two lines or vectors
    Angle,
}

/// Metadata for a measurement
#[derive(Debug, Clone, Default)]
pub struct MeasurementMetadata {
    /// User-provided label
    pub label: Option<String>,
    /// User-provided tags
    pub tags: Vec<String>,
    /// Custom key-value data
    pub custom: HashMap<String, String>,
    /// Notes about this measurement
    pub notes: Option<String>,
}

/// A measurement with geometry and scale information
#[derive(Debug, Clone)]
pub struct Measurement {
    /// Unique identifier
    id: MeasurementId,
    /// Page this measurement is on (0-based)
    page_index: u16,
    /// Geometry defining the measurement (stored in page coordinates)
    geometry: Arc<AnnotationGeometry>,
    /// Type of measurement
    measurement_type: MeasurementType,
    /// Reference to the scale system used
    scale_system_id: ScaleSystemId,
    /// Cached computed value in real-world units (None if needs recomputation)
    value: Option<f32>,
    /// Formatted label with value and unit
    formatted_label: Option<String>,
    /// Additional metadata
    metadata: MeasurementMetadata,
    /// Visibility flag
    visible: bool,
    /// Z-order layer for rendering
    layer: u32,
}

impl Measurement {
    /// Create a new measurement
    pub fn new(
        page_index: u16,
        geometry: AnnotationGeometry,
        measurement_type: MeasurementType,
        scale_system_id: ScaleSystemId,
    ) -> Self {
        Self {
            id: MeasurementId::new_v4(),
            page_index,
            geometry: Arc::new(geometry),
            measurement_type,
            scale_system_id,
            value: None,
            formatted_label: None,
            metadata: MeasurementMetadata::default(),
            visible: true,
            layer: 0,
        }
    }

    /// Get the measurement ID
    pub fn id(&self) -> MeasurementId {
        self.id
    }

    /// Get the page index
    pub fn page_index(&self) -> u16 {
        self.page_index
    }

    /// Get the geometry
    pub fn geometry(&self) -> &AnnotationGeometry {
        &self.geometry
    }

    /// Get the measurement type
    pub fn measurement_type(&self) -> MeasurementType {
        self.measurement_type
    }

    /// Get the scale system ID
    pub fn scale_system_id(&self) -> ScaleSystemId {
        self.scale_system_id
    }

    /// Get the cached value
    pub fn value(&self) -> Option<f32> {
        self.value
    }

    /// Get the formatted label
    pub fn formatted_label(&self) -> Option<&str> {
        self.formatted_label.as_deref()
    }

    /// Get the metadata
    pub fn metadata(&self) -> &MeasurementMetadata {
        &self.metadata
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get the layer
    pub fn layer(&self) -> u32 {
        self.layer
    }

    /// Set visibility
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Set layer
    pub fn set_layer(&mut self, layer: u32) {
        self.layer = layer;
    }

    /// Update geometry (invalidates cached value)
    pub fn with_geometry(mut self, geometry: AnnotationGeometry) -> Self {
        self.geometry = Arc::new(geometry);
        self.value = None;
        self.formatted_label = None;
        self
    }

    /// Update metadata
    pub fn with_metadata(mut self, metadata: MeasurementMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Compute and cache the measurement value
    pub fn compute_value(&mut self, scale_system: &ScaleSystem) {
        let page_value = match self.measurement_type {
            MeasurementType::Distance => self.compute_distance(),
            MeasurementType::Area => self.compute_area(),
            MeasurementType::Radius => self.compute_radius(),
            MeasurementType::Angle => self.compute_angle(),
        };

        // Apply scale conversion
        let real_world_value = match self.measurement_type {
            // Angles: no scale conversion needed (degrees)
            MeasurementType::Angle => page_value,
            // Area: scale squared (square units)
            MeasurementType::Area => {
                let ratio = scale_system.ratio();
                page_value / (ratio * ratio)
            }
            // Distance and Radius: linear scale
            MeasurementType::Distance | MeasurementType::Radius => {
                scale_system.to_real_world(page_value)
            }
        };

        self.value = Some(real_world_value);

        // Format label
        let unit_suffix = if self.measurement_type == MeasurementType::Angle {
            "°".to_string()
        } else if self.measurement_type == MeasurementType::Area {
            format!("{}²", scale_system.unit())
        } else {
            scale_system.unit().to_string()
        };

        self.formatted_label = Some(format!("{:.2}{}", real_world_value, unit_suffix));
    }

    /// Compute distance in page coordinates
    fn compute_distance(&self) -> f32 {
        match self.geometry.as_ref() {
            AnnotationGeometry::Line { start, end } => start.distance_to(end),
            AnnotationGeometry::Arrow { start, end } => start.distance_to(end),
            AnnotationGeometry::Polyline { points } => {
                points.windows(2).map(|w| w[0].distance_to(&w[1])).sum()
            }
            AnnotationGeometry::Circle { radius, .. } => 2.0 * std::f32::consts::PI * radius, // Circumference
            _ => 0.0,
        }
    }

    /// Compute area in page coordinates squared
    fn compute_area(&self) -> f32 {
        match self.geometry.as_ref() {
            AnnotationGeometry::Rectangle {
                top_left,
                bottom_right,
            } => {
                let width = (bottom_right.x - top_left.x).abs();
                let height = (top_left.y - bottom_right.y).abs();
                width * height
            }
            AnnotationGeometry::Circle { radius, .. } => std::f32::consts::PI * radius * radius,
            AnnotationGeometry::Ellipse {
                radius_x, radius_y, ..
            } => std::f32::consts::PI * radius_x * radius_y,
            AnnotationGeometry::Polygon { points } => {
                // Shoelace formula
                let n = points.len();
                if n < 3 {
                    return 0.0;
                }
                let mut area = 0.0;
                for i in 0..n {
                    let j = (i + 1) % n;
                    area += points[i].x * points[j].y;
                    area -= points[j].x * points[i].y;
                }
                (area / 2.0).abs()
            }
            _ => 0.0,
        }
    }

    /// Compute radius in page coordinates
    fn compute_radius(&self) -> f32 {
        match self.geometry.as_ref() {
            AnnotationGeometry::Circle { radius, .. } => *radius,
            AnnotationGeometry::Ellipse {
                radius_x, radius_y, ..
            } => {
                // Average radius for ellipses
                (radius_x + radius_y) / 2.0
            }
            _ => 0.0,
        }
    }

    /// Compute angle in degrees
    fn compute_angle(&self) -> f32 {
        match self.geometry.as_ref() {
            AnnotationGeometry::Polyline { points } if points.len() >= 3 => {
                // Angle between first two segments
                let v1_x = points[1].x - points[0].x;
                let v1_y = points[1].y - points[0].y;
                let v2_x = points[2].x - points[1].x;
                let v2_y = points[2].y - points[1].y;

                let dot = v1_x * v2_x + v1_y * v2_y;
                let mag1 = (v1_x * v1_x + v1_y * v1_y).sqrt();
                let mag2 = (v2_x * v2_x + v2_y * v2_y).sqrt();

                if mag1 > 0.0 && mag2 > 0.0 {
                    let cos_angle = dot / (mag1 * mag2);
                    cos_angle.acos().to_degrees()
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    /// Compute the optimal label position in page coordinates
    /// Returns the position where the measurement label should be placed
    pub fn label_position(&self) -> PageCoordinate {
        match self.geometry.as_ref() {
            // Line and Arrow: midpoint
            AnnotationGeometry::Line { start, end } | AnnotationGeometry::Arrow { start, end } => {
                PageCoordinate::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0)
            }

            // Polyline: midpoint of total path
            AnnotationGeometry::Polyline { points } => {
                if points.is_empty() {
                    return PageCoordinate::new(0.0, 0.0);
                }
                if points.len() == 1 {
                    return points[0];
                }

                // Find midpoint along the path length
                let total_length: f32 = points.windows(2).map(|w| w[0].distance_to(&w[1])).sum();
                let half_length = total_length / 2.0;

                let mut accumulated = 0.0;
                for window in points.windows(2) {
                    let segment_length = window[0].distance_to(&window[1]);
                    if accumulated + segment_length >= half_length {
                        // Interpolate position along this segment
                        let t = (half_length - accumulated) / segment_length;
                        return PageCoordinate::new(
                            window[0].x + t * (window[1].x - window[0].x),
                            window[0].y + t * (window[1].y - window[0].y),
                        );
                    }
                    accumulated += segment_length;
                }

                // Fallback to last point
                *points.last().unwrap()
            }

            // Rectangle: center
            AnnotationGeometry::Rectangle {
                top_left,
                bottom_right,
            } => PageCoordinate::new(
                (top_left.x + bottom_right.x) / 2.0,
                (top_left.y + bottom_right.y) / 2.0,
            ),

            // Circle and Ellipse: center
            AnnotationGeometry::Circle { center, .. }
            | AnnotationGeometry::Ellipse { center, .. } => *center,

            // Polygon: geometric center (centroid)
            AnnotationGeometry::Polygon { points } => {
                if points.is_empty() {
                    return PageCoordinate::new(0.0, 0.0);
                }
                let sum_x: f32 = points.iter().map(|p| p.x).sum();
                let sum_y: f32 = points.iter().map(|p| p.y).sum();
                let n = points.len() as f32;
                PageCoordinate::new(sum_x / n, sum_y / n)
            }

            // Freehand: center of bounding box
            AnnotationGeometry::Freehand { points } => {
                if points.is_empty() {
                    return PageCoordinate::new(0.0, 0.0);
                }
                let sum_x: f32 = points.iter().map(|p| p.x).sum();
                let sum_y: f32 = points.iter().map(|p| p.y).sum();
                let n = points.len() as f32;
                PageCoordinate::new(sum_x / n, sum_y / n)
            }

            // Text: use position directly
            AnnotationGeometry::Text { position, .. } => *position,

            // Note: use position directly
            AnnotationGeometry::Note { position, .. } => *position,
        }
    }
}

/// Collection of measurements and scale systems
#[derive(Debug, Default)]
pub struct MeasurementCollection {
    /// All measurements indexed by ID
    measurements: HashMap<MeasurementId, Measurement>,
    /// All scale systems indexed by ID
    scales: HashMap<ScaleSystemId, ScaleSystem>,
    /// Measurements grouped by page
    by_page: HashMap<u16, Vec<MeasurementId>>,
    /// Default scale system per page
    default_scales: HashMap<u16, ScaleSystemId>,
}

impl MeasurementCollection {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a scale system
    pub fn add_scale(&mut self, scale: ScaleSystem) -> ScaleSystemId {
        let id = scale.id();
        let page_index = scale.page_index();
        self.scales.insert(id, scale);

        // Set as default for page if none exists
        self.default_scales.entry(page_index).or_insert(id);

        id
    }

    /// Get a scale system by ID
    pub fn get_scale(&self, id: ScaleSystemId) -> Option<&ScaleSystem> {
        self.scales.get(&id)
    }

    /// Get the default scale system for a page
    pub fn get_default_scale(&self, page_index: u16) -> Option<&ScaleSystem> {
        self.default_scales
            .get(&page_index)
            .and_then(|id| self.scales.get(id))
    }

    /// Set the default scale system for a page
    pub fn set_default_scale(&mut self, page_index: u16, scale_id: ScaleSystemId) {
        if self.scales.contains_key(&scale_id) {
            self.default_scales.insert(page_index, scale_id);
        }
    }

    /// Get all scale systems for a page
    pub fn get_scales_for_page(&self, page_index: u16) -> Vec<&ScaleSystem> {
        self.scales
            .values()
            .filter(|s| s.page_index() == page_index)
            .collect()
    }

    /// Add a measurement
    pub fn add(&mut self, mut measurement: Measurement) {
        let id = measurement.id();
        let page_index = measurement.page_index();

        // Compute initial value if scale is available
        if let Some(scale) = self.get_scale(measurement.scale_system_id()) {
            measurement.compute_value(scale);
        }

        self.measurements.insert(id, measurement);
        self.by_page.entry(page_index).or_default().push(id);
    }

    /// Remove a measurement
    pub fn remove(&mut self, id: MeasurementId) {
        if let Some(measurement) = self.measurements.remove(&id) {
            if let Some(page_measurements) = self.by_page.get_mut(&measurement.page_index()) {
                page_measurements.retain(|&mid| mid != id);
            }
        }
    }

    /// Get a measurement by ID
    pub fn get(&self, id: MeasurementId) -> Option<&Measurement> {
        self.measurements.get(&id)
    }

    /// Get a mutable measurement by ID
    pub fn get_mut(&mut self, id: MeasurementId) -> Option<&mut Measurement> {
        self.measurements.get_mut(&id)
    }

    /// Get all measurements for a page
    pub fn get_for_page(&self, page_index: u16) -> Vec<&Measurement> {
        self.by_page
            .get(&page_index)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.measurements.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all visible measurements for a page, sorted by layer
    pub fn get_visible_for_page(&self, page_index: u16) -> Vec<&Measurement> {
        let mut measurements = self.get_for_page(page_index);
        measurements.retain(|m| m.is_visible());
        measurements.sort_by_key(|m| m.layer());
        measurements
    }

    /// Recompute all measurements on a page (e.g., after scale change)
    pub fn recompute_page(&mut self, page_index: u16) {
        if let Some(ids) = self.by_page.get(&page_index).cloned() {
            for id in ids {
                let scale_id = self.measurements.get(&id).map(|m| m.scale_system_id());
                if let (Some(measurement), Some(scale_id)) =
                    (self.measurements.get_mut(&id), scale_id)
                {
                    if let Some(scale) = self.scales.get(&scale_id) {
                        measurement.compute_value(scale);
                    }
                }
            }
        }
    }

    /// Get total count of measurements
    pub fn count(&self) -> usize {
        self.measurements.len()
    }

    /// Get count of measurements on a page
    pub fn count_for_page(&self, page_index: u16) -> usize {
        self.by_page
            .get(&page_index)
            .map(|ids| ids.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manual_scale() {
        let scale = ScaleSystem::manual(0, 72.0, "inches");
        assert_eq!(scale.page_index(), 0);
        assert_eq!(scale.unit(), "inches");
        assert_eq!(scale.ratio(), 72.0);
        assert_eq!(scale.to_real_world(144.0), 2.0); // 144 points = 2 inches
        assert_eq!(scale.to_page_coords(2.0), 144.0);
    }

    #[test]
    fn test_two_point_scale() {
        let p1 = PageCoordinate::new(0.0, 0.0);
        let p2 = PageCoordinate::new(100.0, 0.0);
        let scale = ScaleSystem::two_point(0, p1, p2, 10.0, "m");

        assert_eq!(scale.ratio(), 10.0); // 100 page units = 10 real units, ratio = 10
        assert_eq!(scale.to_real_world(100.0), 10.0);
    }

    #[test]
    fn test_measurement_distance() {
        let scale = ScaleSystem::manual(0, 72.0, "inches");
        let scale_id = scale.id();

        let geometry = AnnotationGeometry::Line {
            start: PageCoordinate::new(0.0, 0.0),
            end: PageCoordinate::new(72.0, 0.0),
        };

        let mut measurement = Measurement::new(0, geometry, MeasurementType::Distance, scale_id);
        measurement.compute_value(&scale);

        assert_eq!(measurement.value(), Some(1.0)); // 72 points = 1 inch
        assert_eq!(measurement.formatted_label(), Some("1.00inches"));
    }

    #[test]
    fn test_measurement_area() {
        let scale = ScaleSystem::manual(0, 72.0, "inches");
        let scale_id = scale.id();

        let geometry = AnnotationGeometry::Rectangle {
            top_left: PageCoordinate::new(0.0, 72.0),
            bottom_right: PageCoordinate::new(72.0, 0.0),
        };

        let mut measurement = Measurement::new(0, geometry, MeasurementType::Area, scale_id);
        measurement.compute_value(&scale);

        // Area = 72 * 72 = 5184 square points
        // In square inches: 5184 / (72 * 72) = 1.0
        assert_eq!(measurement.value(), Some(1.0));
        assert_eq!(measurement.formatted_label(), Some("1.00inches²"));
    }

    #[test]
    fn test_measurement_radius() {
        let scale = ScaleSystem::manual(0, 72.0, "inches");
        let scale_id = scale.id();

        let geometry = AnnotationGeometry::Circle {
            center: PageCoordinate::new(100.0, 100.0),
            radius: 36.0,
        };

        let mut measurement = Measurement::new(0, geometry, MeasurementType::Radius, scale_id);
        measurement.compute_value(&scale);

        assert_eq!(measurement.value(), Some(0.5)); // 36 points = 0.5 inches
    }

    #[test]
    fn test_measurement_angle() {
        let scale = ScaleSystem::manual(0, 72.0, "inches");
        let scale_id = scale.id();

        let geometry = AnnotationGeometry::Polyline {
            points: vec![
                PageCoordinate::new(0.0, 0.0),
                PageCoordinate::new(100.0, 0.0),
                PageCoordinate::new(100.0, 100.0),
            ],
        };

        let mut measurement = Measurement::new(0, geometry, MeasurementType::Angle, scale_id);
        measurement.compute_value(&scale);

        assert!((measurement.value().unwrap() - 90.0).abs() < 0.01); // 90 degree angle
        assert_eq!(measurement.formatted_label(), Some("90.00°"));
    }

    #[test]
    fn test_measurement_collection() {
        let mut collection = MeasurementCollection::new();

        let scale = ScaleSystem::manual(0, 72.0, "inches");
        let scale_id = collection.add_scale(scale);

        let geometry = AnnotationGeometry::Line {
            start: PageCoordinate::new(0.0, 0.0),
            end: PageCoordinate::new(72.0, 0.0),
        };

        let measurement = Measurement::new(0, geometry, MeasurementType::Distance, scale_id);
        let measurement_id = measurement.id();

        collection.add(measurement);

        assert_eq!(collection.count(), 1);
        assert_eq!(collection.count_for_page(0), 1);
        assert!(collection.get(measurement_id).is_some());
        assert_eq!(collection.get_for_page(0).len(), 1);
    }

    #[test]
    fn test_measurement_recompute() {
        let mut collection = MeasurementCollection::new();

        let scale = ScaleSystem::manual(0, 72.0, "inches");
        let scale_id = collection.add_scale(scale);

        let geometry = AnnotationGeometry::Line {
            start: PageCoordinate::new(0.0, 0.0),
            end: PageCoordinate::new(144.0, 0.0),
        };

        let measurement = Measurement::new(0, geometry, MeasurementType::Distance, scale_id);
        collection.add(measurement);

        {
            let measurements = collection.get_for_page(0);
            assert_eq!(measurements[0].value(), Some(2.0)); // 144 points = 2 inches
        }

        // Change scale
        let new_scale = ScaleSystem::manual(0, 36.0, "inches");
        let new_scale_id = new_scale.id();
        collection.add_scale(new_scale);
        collection.set_default_scale(0, new_scale_id);

        // Update measurement to use new scale
        let measurement_id = {
            let measurements = collection.get_for_page(0);
            measurements[0].id()
        };

        if let Some(m) = collection.get_mut(measurement_id) {
            let geometry = m.geometry().clone();
            *m = m.clone().with_geometry(geometry);
            m.scale_system_id = new_scale_id;
        }

        collection.recompute_page(0);

        let measurements = collection.get_for_page(0);
        assert_eq!(measurements[0].value(), Some(4.0)); // 144 points = 4 inches with new scale
    }

    #[test]
    fn test_polygon_area_shoelace() {
        let scale = ScaleSystem::manual(0, 1.0, "units");
        let scale_id = scale.id();

        // Triangle with vertices at (0,0), (10,0), (5,10)
        let geometry = AnnotationGeometry::Polygon {
            points: vec![
                PageCoordinate::new(0.0, 0.0),
                PageCoordinate::new(10.0, 0.0),
                PageCoordinate::new(5.0, 10.0),
            ],
        };

        let mut measurement = Measurement::new(0, geometry, MeasurementType::Area, scale_id);
        measurement.compute_value(&scale);

        // Triangle area = 0.5 * base * height = 0.5 * 10 * 10 = 50
        assert!((measurement.value().unwrap() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_label_position_line() {
        let geometry = AnnotationGeometry::Line {
            start: PageCoordinate::new(0.0, 0.0),
            end: PageCoordinate::new(100.0, 50.0),
        };
        let scale = ScaleSystem::manual(0, 1.0, "units");
        let measurement = Measurement::new(0, geometry, MeasurementType::Distance, scale.id());

        let pos = measurement.label_position();
        assert_eq!(pos.x, 50.0);
        assert_eq!(pos.y, 25.0);
    }

    #[test]
    fn test_label_position_rectangle() {
        let geometry = AnnotationGeometry::Rectangle {
            top_left: PageCoordinate::new(10.0, 50.0),
            bottom_right: PageCoordinate::new(30.0, 10.0),
        };
        let scale = ScaleSystem::manual(0, 1.0, "units");
        let measurement = Measurement::new(0, geometry, MeasurementType::Area, scale.id());

        let pos = measurement.label_position();
        assert_eq!(pos.x, 20.0);
        assert_eq!(pos.y, 30.0);
    }

    #[test]
    fn test_label_position_circle() {
        let geometry = AnnotationGeometry::Circle {
            center: PageCoordinate::new(100.0, 200.0),
            radius: 50.0,
        };
        let scale = ScaleSystem::manual(0, 1.0, "units");
        let measurement = Measurement::new(0, geometry, MeasurementType::Radius, scale.id());

        let pos = measurement.label_position();
        assert_eq!(pos.x, 100.0);
        assert_eq!(pos.y, 200.0);
    }

    #[test]
    fn test_label_position_polyline() {
        let geometry = AnnotationGeometry::Polyline {
            points: vec![
                PageCoordinate::new(0.0, 0.0),
                PageCoordinate::new(100.0, 0.0),
                PageCoordinate::new(100.0, 100.0),
            ],
        };
        let scale = ScaleSystem::manual(0, 1.0, "units");
        let measurement = Measurement::new(0, geometry, MeasurementType::Distance, scale.id());

        let pos = measurement.label_position();
        // Total length is 200 (100 + 100), midpoint is at 100
        // First segment is 100 units, so midpoint is at the junction
        assert_eq!(pos.x, 100.0);
        assert_eq!(pos.y, 0.0);
    }

    #[test]
    fn test_label_position_polygon() {
        let geometry = AnnotationGeometry::Polygon {
            points: vec![
                PageCoordinate::new(0.0, 0.0),
                PageCoordinate::new(60.0, 0.0),
                PageCoordinate::new(60.0, 60.0),
                PageCoordinate::new(0.0, 60.0),
            ],
        };
        let scale = ScaleSystem::manual(0, 1.0, "units");
        let measurement = Measurement::new(0, geometry, MeasurementType::Area, scale.id());

        let pos = measurement.label_position();
        // Centroid of square is at center
        assert_eq!(pos.x, 30.0);
        assert_eq!(pos.y, 30.0);
    }
}
