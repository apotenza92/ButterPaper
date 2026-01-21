//! Annotation engine data model
//!
//! Provides immutable geometry with editable metadata for CAD-style annotations.
//! All coordinates are stored in page-local coordinate space.

use std::sync::Arc;

/// Unique identifier for an annotation
///
/// Stable across document lifetime, persists in saved files.
/// Generated using UUID v4 for guaranteed uniqueness.
pub type AnnotationId = uuid::Uuid;

/// Page-local coordinate in PDF page space
///
/// Uses PDF coordinate system:
/// - Origin (0, 0) at bottom-left of page
/// - X increases to the right
/// - Y increases upward
/// - Units are in points (1/72 inch)
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PageCoordinate {
    pub x: f32,
    pub y: f32,
}

impl PageCoordinate {
    /// Create a new page coordinate
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Calculate distance to another coordinate
    pub fn distance_to(&self, other: &PageCoordinate) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// RGBA color representation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Create a new color
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create an opaque color
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Convert to normalized RGBA values (0.0 to 1.0)
    pub fn to_normalized(&self) -> (f32, f32, f32, f32) {
        (
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        )
    }
}

/// Common annotation styles
impl Color {
    pub const RED: Color = Color { r: 255, g: 0, b: 0, a: 255 };
    pub const GREEN: Color = Color { r: 0, g: 255, b: 0, a: 255 };
    pub const BLUE: Color = Color { r: 0, g: 0, b: 255, a: 255 };
    pub const YELLOW: Color = Color { r: 255, g: 255, b: 0, a: 255 };
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
}

/// Immutable annotation geometry
///
/// Geometry is immutable once created. To modify geometry, create a new annotation.
/// This ensures predictable rendering and caching behavior.
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationGeometry {
    /// Line segment from start to end point
    Line {
        start: PageCoordinate,
        end: PageCoordinate,
    },

    /// Polyline connecting multiple points
    Polyline {
        points: Vec<PageCoordinate>,
    },

    /// Closed polygon
    Polygon {
        points: Vec<PageCoordinate>,
    },

    /// Rectangle defined by two corners
    Rectangle {
        top_left: PageCoordinate,
        bottom_right: PageCoordinate,
    },

    /// Circle defined by center and radius (in points)
    Circle {
        center: PageCoordinate,
        radius: f32,
    },

    /// Ellipse defined by center and radii
    Ellipse {
        center: PageCoordinate,
        radius_x: f32,
        radius_y: f32,
    },

    /// Freehand drawing path
    Freehand {
        points: Vec<PageCoordinate>,
    },

    /// Text annotation at a specific point
    Text {
        position: PageCoordinate,
        /// Maximum width for text wrapping (in points), None for no wrapping
        max_width: Option<f32>,
    },

    /// Arrow from start to end point with arrowhead at end
    Arrow {
        start: PageCoordinate,
        end: PageCoordinate,
    },
}

impl AnnotationGeometry {
    /// Get the bounding box for this geometry
    ///
    /// Returns (min_x, min_y, max_x, max_y) in page coordinates.
    pub fn bounding_box(&self) -> (f32, f32, f32, f32) {
        match self {
            AnnotationGeometry::Line { start, end } => {
                let min_x = start.x.min(end.x);
                let max_x = start.x.max(end.x);
                let min_y = start.y.min(end.y);
                let max_y = start.y.max(end.y);
                (min_x, min_y, max_x, max_y)
            }
            AnnotationGeometry::Polyline { points }
            | AnnotationGeometry::Polygon { points }
            | AnnotationGeometry::Freehand { points } => {
                if points.is_empty() {
                    return (0.0, 0.0, 0.0, 0.0);
                }
                let mut min_x = points[0].x;
                let mut max_x = points[0].x;
                let mut min_y = points[0].y;
                let mut max_y = points[0].y;
                for point in points.iter().skip(1) {
                    min_x = min_x.min(point.x);
                    max_x = max_x.max(point.x);
                    min_y = min_y.min(point.y);
                    max_y = max_y.max(point.y);
                }
                (min_x, min_y, max_x, max_y)
            }
            AnnotationGeometry::Rectangle {
                top_left,
                bottom_right,
            } => (top_left.x, bottom_right.y, bottom_right.x, top_left.y),
            AnnotationGeometry::Circle { center, radius } => (
                center.x - radius,
                center.y - radius,
                center.x + radius,
                center.y + radius,
            ),
            AnnotationGeometry::Ellipse {
                center,
                radius_x,
                radius_y,
            } => (
                center.x - radius_x,
                center.y - radius_y,
                center.x + radius_x,
                center.y + radius_y,
            ),
            AnnotationGeometry::Text {
                position,
                max_width,
            } => {
                // Conservative estimate - actual bounds depend on text rendering
                let width = max_width.unwrap_or(200.0);
                (position.x, position.y, position.x + width, position.y + 50.0)
            }
            AnnotationGeometry::Arrow { start, end } => {
                let min_x = start.x.min(end.x);
                let max_x = start.x.max(end.x);
                let min_y = start.y.min(end.y);
                let max_y = start.y.max(end.y);
                (min_x, min_y, max_x, max_y)
            }
        }
    }

    /// Check if a point is near this geometry (within tolerance)
    ///
    /// Used for hit testing during selection.
    pub fn contains_point(&self, point: &PageCoordinate, tolerance: f32) -> bool {
        match self {
            AnnotationGeometry::Line { start, end } => {
                point_near_line_segment(point, start, end, tolerance)
            }
            AnnotationGeometry::Arrow { start, end } => {
                point_near_line_segment(point, start, end, tolerance)
            }
            AnnotationGeometry::Polyline { points } | AnnotationGeometry::Freehand { points } => {
                for i in 0..points.len().saturating_sub(1) {
                    if point_near_line_segment(point, &points[i], &points[i + 1], tolerance) {
                        return true;
                    }
                }
                false
            }
            AnnotationGeometry::Polygon { points } => {
                // Check if point is near any edge
                for i in 0..points.len() {
                    let next = (i + 1) % points.len();
                    if point_near_line_segment(point, &points[i], &points[next], tolerance) {
                        return true;
                    }
                }
                false
            }
            AnnotationGeometry::Rectangle {
                top_left: _,
                bottom_right: _,
            } => {
                let (min_x, min_y, max_x, max_y) = self.bounding_box();
                point.x >= min_x - tolerance
                    && point.x <= max_x + tolerance
                    && point.y >= min_y - tolerance
                    && point.y <= max_y + tolerance
            }
            AnnotationGeometry::Circle { center, radius } => {
                let dist = point.distance_to(center);
                (dist - radius).abs() <= tolerance
            }
            AnnotationGeometry::Ellipse {
                center,
                radius_x,
                radius_y,
            } => {
                let dx = (point.x - center.x) / radius_x;
                let dy = (point.y - center.y) / radius_y;
                let dist = (dx * dx + dy * dy).sqrt();
                (dist - 1.0).abs() * radius_x.max(*radius_y) <= tolerance
            }
            AnnotationGeometry::Text {
                position: _,
                max_width: _,
            } => {
                let (min_x, min_y, max_x, max_y) = self.bounding_box();
                point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y
            }
        }
    }
}

/// Helper function for point-to-line-segment distance check
fn point_near_line_segment(
    point: &PageCoordinate,
    start: &PageCoordinate,
    end: &PageCoordinate,
    tolerance: f32,
) -> bool {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_sq = dx * dx + dy * dy;

    if length_sq < 1e-6 {
        // Degenerate line segment
        return point.distance_to(start) <= tolerance;
    }

    // Project point onto line segment
    let t = ((point.x - start.x) * dx + (point.y - start.y) * dy) / length_sq;
    let t = t.clamp(0.0, 1.0);

    let closest = PageCoordinate::new(start.x + t * dx, start.y + t * dy);
    point.distance_to(&closest) <= tolerance
}

/// Editable annotation metadata
///
/// Metadata can be changed without affecting geometry or rendering.
/// Changes to metadata don't invalidate tile cache.
#[derive(Debug, Clone)]
pub struct AnnotationMetadata {
    /// User-provided label or description
    pub label: Option<String>,

    /// Author or creator of the annotation
    pub author: Option<String>,

    /// Creation timestamp (Unix timestamp in seconds)
    pub created_at: i64,

    /// Last modification timestamp (Unix timestamp in seconds)
    pub modified_at: i64,

    /// User-defined tags for organization
    pub tags: Vec<String>,

    /// Custom key-value metadata
    pub custom: std::collections::HashMap<String, String>,
}

impl AnnotationMetadata {
    /// Create new metadata with current timestamp
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Self {
            label: None,
            author: None,
            created_at: now,
            modified_at: now,
            tags: Vec::new(),
            custom: std::collections::HashMap::new(),
        }
    }

    /// Update the modified timestamp to now
    pub fn touch(&mut self) {
        self.modified_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
    }
}

impl Default for AnnotationMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Visual styling for annotation rendering
///
/// Immutable like geometry. To change appearance, create a new annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationStyle {
    /// Stroke color for lines and outlines
    pub stroke_color: Color,

    /// Stroke width in points
    pub stroke_width: f32,

    /// Fill color for closed shapes (None for no fill)
    pub fill_color: Option<Color>,

    /// Line dash pattern (empty for solid line)
    pub dash_pattern: Vec<f32>,

    /// Opacity (0.0 = transparent, 1.0 = opaque)
    pub opacity: f32,

    /// Font size for text annotations (in points)
    pub font_size: f32,

    /// Font family for text annotations
    pub font_family: String,
}

impl AnnotationStyle {
    /// Create default style (black stroke, 2pt width, no fill)
    pub fn new() -> Self {
        Self {
            stroke_color: Color::BLACK,
            stroke_width: 2.0,
            fill_color: None,
            dash_pattern: Vec::new(),
            opacity: 1.0,
            font_size: 12.0,
            font_family: "Helvetica".to_string(),
        }
    }

    /// Create style with red stroke (common for markups)
    pub fn red_markup() -> Self {
        Self {
            stroke_color: Color::RED,
            stroke_width: 2.0,
            fill_color: None,
            dash_pattern: Vec::new(),
            opacity: 1.0,
            font_size: 12.0,
            font_family: "Helvetica".to_string(),
        }
    }

    /// Create style with yellow highlight fill
    pub fn yellow_highlight() -> Self {
        Self {
            stroke_color: Color::YELLOW,
            stroke_width: 0.0,
            fill_color: Some(Color::new(255, 255, 0, 128)), // Semi-transparent yellow
            dash_pattern: Vec::new(),
            opacity: 0.5,
            font_size: 12.0,
            font_family: "Helvetica".to_string(),
        }
    }
}

impl Default for AnnotationStyle {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete annotation with immutable geometry and editable metadata
///
/// Annotations are stored per-page and rendered as GPU vector primitives.
#[derive(Debug, Clone)]
pub struct Annotation {
    /// Stable unique identifier
    id: AnnotationId,

    /// Page index this annotation belongs to (0-based)
    page_index: u16,

    /// Immutable geometry (to modify, create new annotation)
    geometry: Arc<AnnotationGeometry>,

    /// Immutable visual style (to modify, create new annotation)
    style: Arc<AnnotationStyle>,

    /// Editable metadata (can be changed without affecting rendering)
    metadata: AnnotationMetadata,

    /// Whether this annotation is currently selected
    selected: bool,

    /// Whether this annotation is visible
    visible: bool,

    /// Layer/z-index for rendering order (higher = on top)
    layer: u32,
}

impl Annotation {
    /// Create a new annotation with generated ID
    pub fn new(
        page_index: u16,
        geometry: AnnotationGeometry,
        style: AnnotationStyle,
    ) -> Self {
        Self {
            id: AnnotationId::new_v4(),
            page_index,
            geometry: Arc::new(geometry),
            style: Arc::new(style),
            metadata: AnnotationMetadata::new(),
            selected: false,
            visible: true,
            layer: 0,
        }
    }

    /// Create a new annotation with specific ID (for deserialization)
    pub fn with_id(
        id: AnnotationId,
        page_index: u16,
        geometry: AnnotationGeometry,
        style: AnnotationStyle,
    ) -> Self {
        Self {
            id,
            page_index,
            geometry: Arc::new(geometry),
            style: Arc::new(style),
            metadata: AnnotationMetadata::new(),
            selected: false,
            visible: true,
            layer: 0,
        }
    }

    /// Get the annotation ID
    pub fn id(&self) -> AnnotationId {
        self.id
    }

    /// Get the page index
    pub fn page_index(&self) -> u16 {
        self.page_index
    }

    /// Get the geometry (immutable reference)
    pub fn geometry(&self) -> &AnnotationGeometry {
        &self.geometry
    }

    /// Get the style (immutable reference)
    pub fn style(&self) -> &AnnotationStyle {
        &self.style
    }

    /// Get the metadata (mutable reference)
    pub fn metadata(&self) -> &AnnotationMetadata {
        &self.metadata
    }

    /// Get mutable metadata reference
    pub fn metadata_mut(&mut self) -> &mut AnnotationMetadata {
        &mut self.metadata
    }

    /// Check if annotation is selected
    pub fn is_selected(&self) -> bool {
        self.selected
    }

    /// Set selection state
    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }

    /// Check if annotation is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Set visibility
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Get the layer/z-index
    pub fn layer(&self) -> u32 {
        self.layer
    }

    /// Set the layer/z-index
    pub fn set_layer(&mut self, layer: u32) {
        self.layer = layer;
    }

    /// Get the bounding box in page coordinates
    pub fn bounding_box(&self) -> (f32, f32, f32, f32) {
        self.geometry.bounding_box()
    }

    /// Check if a point hits this annotation (for selection)
    pub fn hit_test(&self, point: &PageCoordinate, tolerance: f32) -> bool {
        if !self.visible {
            return false;
        }
        self.geometry.contains_point(point, tolerance)
    }

    /// Create a modified copy with new geometry (preserves ID and metadata)
    pub fn with_geometry(&self, geometry: AnnotationGeometry) -> Self {
        let mut new_annotation = self.clone();
        new_annotation.geometry = Arc::new(geometry);
        new_annotation.metadata.touch();
        new_annotation
    }

    /// Create a modified copy with new style (preserves ID and metadata)
    pub fn with_style(&self, style: AnnotationStyle) -> Self {
        let mut new_annotation = self.clone();
        new_annotation.style = Arc::new(style);
        new_annotation.metadata.touch();
        new_annotation
    }
}

/// Collection of annotations for a document
///
/// Manages annotations across all pages with efficient lookup and rendering.
pub struct AnnotationCollection {
    /// All annotations indexed by ID
    annotations: std::collections::HashMap<AnnotationId, Annotation>,

    /// Annotations organized by page for efficient page rendering
    by_page: std::collections::HashMap<u16, Vec<AnnotationId>>,
}

impl AnnotationCollection {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self {
            annotations: std::collections::HashMap::new(),
            by_page: std::collections::HashMap::new(),
        }
    }

    /// Add an annotation to the collection
    pub fn add(&mut self, annotation: Annotation) {
        let id = annotation.id();
        let page_index = annotation.page_index();

        self.annotations.insert(id, annotation);
        self.by_page
            .entry(page_index)
            .or_default()
            .push(id);
    }

    /// Remove an annotation by ID
    pub fn remove(&mut self, id: AnnotationId) -> Option<Annotation> {
        if let Some(annotation) = self.annotations.remove(&id) {
            let page_index = annotation.page_index();
            if let Some(page_annotations) = self.by_page.get_mut(&page_index) {
                page_annotations.retain(|&aid| aid != id);
                if page_annotations.is_empty() {
                    self.by_page.remove(&page_index);
                }
            }
            Some(annotation)
        } else {
            None
        }
    }

    /// Get an annotation by ID
    pub fn get(&self, id: AnnotationId) -> Option<&Annotation> {
        self.annotations.get(&id)
    }

    /// Get a mutable reference to an annotation by ID
    pub fn get_mut(&mut self, id: AnnotationId) -> Option<&mut Annotation> {
        self.annotations.get_mut(&id)
    }

    /// Get all annotations for a specific page, sorted by layer
    pub fn get_page_annotations(&self, page_index: u16) -> Vec<&Annotation> {
        let mut annotations: Vec<&Annotation> = self
            .by_page
            .get(&page_index)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.annotations.get(id))
                    .collect()
            })
            .unwrap_or_default();

        // Sort by layer (ascending), so lower layers render first
        annotations.sort_by_key(|a| a.layer());
        annotations
    }

    /// Get all annotations in the collection
    pub fn all(&self) -> Vec<&Annotation> {
        self.annotations.values().collect()
    }

    /// Get count of annotations
    pub fn len(&self) -> usize {
        self.annotations.len()
    }

    /// Check if collection is empty
    pub fn is_empty(&self) -> bool {
        self.annotations.is_empty()
    }

    /// Clear all annotations
    pub fn clear(&mut self) {
        self.annotations.clear();
        self.by_page.clear();
    }

    /// Hit test to find annotations at a point on a page
    ///
    /// Returns annotations sorted by layer (top to bottom), so the first
    /// result is the topmost annotation at the point.
    pub fn hit_test(
        &self,
        page_index: u16,
        point: &PageCoordinate,
        tolerance: f32,
    ) -> Vec<&Annotation> {
        let mut hits: Vec<&Annotation> = self
            .by_page
            .get(&page_index)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.annotations.get(id))
                    .filter(|a| a.hit_test(point, tolerance))
                    .collect()
            })
            .unwrap_or_default();

        // Sort by layer (descending), so topmost annotations come first
        hits.sort_by_key(|a| std::cmp::Reverse(a.layer()));
        hits
    }
}

impl Default for AnnotationCollection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_coordinate_distance() {
        let p1 = PageCoordinate::new(0.0, 0.0);
        let p2 = PageCoordinate::new(3.0, 4.0);
        assert!((p1.distance_to(&p2) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_color_normalization() {
        let color = Color::rgb(255, 128, 0);
        let (r, g, b, a) = color.to_normalized();
        assert!((r - 1.0).abs() < 0.001);
        assert!((g - 0.502).abs() < 0.01);
        assert!((b - 0.0).abs() < 0.001);
        assert!((a - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_line_bounding_box() {
        let geometry = AnnotationGeometry::Line {
            start: PageCoordinate::new(10.0, 20.0),
            end: PageCoordinate::new(50.0, 80.0),
        };
        let (min_x, min_y, max_x, max_y) = geometry.bounding_box();
        assert_eq!(min_x, 10.0);
        assert_eq!(min_y, 20.0);
        assert_eq!(max_x, 50.0);
        assert_eq!(max_y, 80.0);
    }

    #[test]
    fn test_circle_bounding_box() {
        let geometry = AnnotationGeometry::Circle {
            center: PageCoordinate::new(100.0, 100.0),
            radius: 25.0,
        };
        let (min_x, min_y, max_x, max_y) = geometry.bounding_box();
        assert_eq!(min_x, 75.0);
        assert_eq!(min_y, 75.0);
        assert_eq!(max_x, 125.0);
        assert_eq!(max_y, 125.0);
    }

    #[test]
    fn test_annotation_creation() {
        let geometry = AnnotationGeometry::Line {
            start: PageCoordinate::new(0.0, 0.0),
            end: PageCoordinate::new(100.0, 100.0),
        };
        let style = AnnotationStyle::red_markup();
        let annotation = Annotation::new(0, geometry, style);

        assert_eq!(annotation.page_index(), 0);
        assert!(annotation.is_visible());
        assert!(!annotation.is_selected());
        assert_eq!(annotation.layer(), 0);
    }

    #[test]
    fn test_annotation_metadata() {
        let geometry = AnnotationGeometry::Circle {
            center: PageCoordinate::new(50.0, 50.0),
            radius: 10.0,
        };
        let style = AnnotationStyle::new();
        let mut annotation = Annotation::new(0, geometry, style);

        annotation.metadata_mut().label = Some("Test Circle".to_string());
        annotation.metadata_mut().tags.push("markup".to_string());

        assert_eq!(annotation.metadata().label, Some("Test Circle".to_string()));
        assert_eq!(annotation.metadata().tags.len(), 1);
    }

    #[test]
    fn test_annotation_collection() {
        let mut collection = AnnotationCollection::new();

        let geom1 = AnnotationGeometry::Line {
            start: PageCoordinate::new(0.0, 0.0),
            end: PageCoordinate::new(100.0, 100.0),
        };
        let geom2 = AnnotationGeometry::Circle {
            center: PageCoordinate::new(50.0, 50.0),
            radius: 25.0,
        };

        let annotation1 = Annotation::new(0, geom1, AnnotationStyle::new());
        let annotation2 = Annotation::new(1, geom2, AnnotationStyle::new());

        let id1 = annotation1.id();
        collection.add(annotation1);
        collection.add(annotation2);

        assert_eq!(collection.len(), 2);
        assert_eq!(collection.get_page_annotations(0).len(), 1);
        assert_eq!(collection.get_page_annotations(1).len(), 1);

        collection.remove(id1);
        assert_eq!(collection.len(), 1);
        assert_eq!(collection.get_page_annotations(0).len(), 0);
    }

    #[test]
    fn test_hit_testing() {
        let geometry = AnnotationGeometry::Circle {
            center: PageCoordinate::new(100.0, 100.0),
            radius: 25.0,
        };
        let style = AnnotationStyle::new();
        let annotation = Annotation::new(0, geometry, style);

        // Point on the circle
        let point_on = PageCoordinate::new(125.0, 100.0);
        assert!(annotation.hit_test(&point_on, 5.0));

        // Point inside (should not hit because it's measuring distance to edge)
        let point_inside = PageCoordinate::new(100.0, 100.0);
        assert!(!annotation.hit_test(&point_inside, 5.0));

        // Point far outside
        let point_outside = PageCoordinate::new(200.0, 200.0);
        assert!(!annotation.hit_test(&point_outside, 5.0));
    }

    #[test]
    fn test_annotation_layer_sorting() {
        let mut collection = AnnotationCollection::new();

        let mut annotation1 = Annotation::new(
            0,
            AnnotationGeometry::Circle {
                center: PageCoordinate::new(50.0, 50.0),
                radius: 10.0,
            },
            AnnotationStyle::new(),
        );
        annotation1.set_layer(2);

        let mut annotation2 = Annotation::new(
            0,
            AnnotationGeometry::Circle {
                center: PageCoordinate::new(60.0, 60.0),
                radius: 10.0,
            },
            AnnotationStyle::new(),
        );
        annotation2.set_layer(1);

        let mut annotation3 = Annotation::new(
            0,
            AnnotationGeometry::Circle {
                center: PageCoordinate::new(70.0, 70.0),
                radius: 10.0,
            },
            AnnotationStyle::new(),
        );
        annotation3.set_layer(3);

        collection.add(annotation1);
        collection.add(annotation2);
        collection.add(annotation3);

        let page_annotations = collection.get_page_annotations(0);
        assert_eq!(page_annotations.len(), 3);
        // Should be sorted by layer ascending
        assert_eq!(page_annotations[0].layer(), 1);
        assert_eq!(page_annotations[1].layer(), 2);
        assert_eq!(page_annotations[2].layer(), 3);
    }

    #[test]
    fn test_annotation_with_geometry() {
        let original_geometry = AnnotationGeometry::Circle {
            center: PageCoordinate::new(50.0, 50.0),
            radius: 10.0,
        };
        let annotation = Annotation::new(0, original_geometry, AnnotationStyle::new());
        let original_id = annotation.id();
        let created_at = annotation.metadata().created_at;

        // Simulate a short delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(100));

        let new_geometry = AnnotationGeometry::Circle {
            center: PageCoordinate::new(100.0, 100.0),
            radius: 20.0,
        };
        let modified = annotation.with_geometry(new_geometry);

        // ID should be preserved
        assert_eq!(modified.id(), original_id);
        // Modified timestamp should be updated or equal (in case of low resolution)
        assert!(modified.metadata().modified_at >= created_at);
        // Geometry should be changed
        let (_, _, max_x, _) = modified.bounding_box();
        assert_eq!(max_x, 120.0); // 100 + 20
    }
}
