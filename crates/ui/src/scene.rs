//! Retained scene graph for GPU-accelerated UI rendering
//!
//! This module provides a retained scene graph system optimized for GPU rendering.
//! Nodes are retained in memory and only re-rendered when they change (dirty tracking).

use std::sync::Arc;

/// Unique identifier for a scene node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u64);

impl NodeId {
    /// Create a new unique node ID
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// 2D transformation matrix
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// Translation (x, y)
    pub translation: [f32; 2],
    /// Scale (x, y)
    pub scale: [f32; 2],
    /// Rotation in radians
    pub rotation: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0],
            scale: [1.0, 1.0],
            rotation: 0.0,
        }
    }
}

impl Transform {
    /// Create a translation transform
    pub fn translate(x: f32, y: f32) -> Self {
        Self {
            translation: [x, y],
            ..Default::default()
        }
    }

    /// Create a scale transform
    pub fn scale(x: f32, y: f32) -> Self {
        Self {
            scale: [x, y],
            ..Default::default()
        }
    }

    /// Combine this transform with another (matrix multiplication)
    pub fn combine(&self, other: &Transform) -> Transform {
        // Simplified transform combination
        // For a full 2D engine, this would use proper 3x3 matrix multiplication
        Transform {
            translation: [
                self.translation[0] + other.translation[0] * self.scale[0],
                self.translation[1] + other.translation[1] * self.scale[1],
            ],
            scale: [self.scale[0] * other.scale[0], self.scale[1] * other.scale[1]],
            rotation: self.rotation + other.rotation,
        }
    }
}

/// RGBA color value
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Create a new color from RGBA values (0.0 to 1.0)
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create a new opaque color from RGB values
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }
}

/// Rectangle primitive
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Top-left x coordinate
    pub x: f32,
    /// Top-left y coordinate
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
}

impl Rect {
    /// Create a new rectangle
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
}

/// Visual primitive types that can be rendered
#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    /// Filled rectangle with solid color
    Rectangle {
        rect: Rect,
        color: Color,
    },
    /// Textured quad (for images and cached text)
    TexturedQuad {
        rect: Rect,
        texture_id: u64,
    },
    /// Line segment
    Line {
        start: [f32; 2],
        end: [f32; 2],
        width: f32,
        color: Color,
    },
    /// Circle (filled or stroke-only based on fill_color)
    Circle {
        center: [f32; 2],
        radius: f32,
        color: Color,
    },
    /// Polyline (connected line segments)
    Polyline {
        points: Vec<[f32; 2]>,
        width: f32,
        color: Color,
        closed: bool,
    },
    /// Polygon (closed shape with optional fill and stroke)
    Polygon {
        points: Vec<[f32; 2]>,
        fill_color: Option<Color>,
        stroke_color: Color,
        stroke_width: f32,
    },
    /// Ellipse
    Ellipse {
        center: [f32; 2],
        radius_x: f32,
        radius_y: f32,
        fill_color: Option<Color>,
        stroke_color: Color,
        stroke_width: f32,
    },
    /// Arrow (line with arrowhead at the end)
    Arrow {
        start: [f32; 2],
        end: [f32; 2],
        width: f32,
        color: Color,
        head_size: f32,
    },
}

/// Scene node representing a visual element in the scene graph
#[derive(Clone)]
pub struct SceneNode {
    /// Unique identifier
    id: NodeId,
    /// Local transform (relative to parent)
    transform: Transform,
    /// Visual primitives to render for this node
    primitives: Vec<Primitive>,
    /// Child nodes
    children: Vec<Arc<SceneNode>>,
    /// Whether this node needs to be re-rendered
    dirty: bool,
    /// Whether this node is visible
    visible: bool,
}

impl SceneNode {
    /// Create a new scene node
    pub fn new() -> Self {
        Self {
            id: NodeId::new(),
            transform: Transform::default(),
            primitives: Vec::new(),
            children: Vec::new(),
            dirty: true,
            visible: true,
        }
    }

    /// Get the node's unique ID
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Set the local transform
    pub fn set_transform(&mut self, transform: Transform) {
        if self.transform != transform {
            self.transform = transform;
            self.mark_dirty();
        }
    }

    /// Get the local transform
    pub fn transform(&self) -> &Transform {
        &self.transform
    }

    /// Add a primitive to this node
    pub fn add_primitive(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
        self.mark_dirty();
    }

    /// Set all primitives for this node (replaces existing)
    pub fn set_primitives(&mut self, primitives: Vec<Primitive>) {
        self.primitives = primitives;
        self.mark_dirty();
    }

    /// Get primitives
    pub fn primitives(&self) -> &[Primitive] {
        &self.primitives
    }

    /// Add a child node
    pub fn add_child(&mut self, child: Arc<SceneNode>) {
        self.children.push(child);
        self.mark_dirty();
    }

    /// Get child nodes
    pub fn children(&self) -> &[Arc<SceneNode>] {
        &self.children
    }

    /// Mark this node as dirty (needs re-render)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Check if this node is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear dirty flag
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Set visibility
    pub fn set_visible(&mut self, visible: bool) {
        if self.visible != visible {
            self.visible = visible;
            self.mark_dirty();
        }
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl Default for SceneNode {
    fn default() -> Self {
        Self::new()
    }
}

/// Scene graph root containing all visible nodes
pub struct SceneGraph {
    root: Arc<SceneNode>,
}

impl SceneGraph {
    /// Create a new empty scene graph
    pub fn new() -> Self {
        Self {
            root: Arc::new(SceneNode::new()),
        }
    }

    /// Get the root node
    pub fn root(&self) -> &Arc<SceneNode> {
        &self.root
    }

    /// Get mutable access to root (requires Arc::make_mut)
    pub fn root_mut(&mut self) -> &mut SceneNode {
        Arc::make_mut(&mut self.root)
    }

    /// Traverse the scene graph and collect all visible primitives with transforms
    pub fn collect_render_commands(&self) -> Vec<RenderCommand> {
        let mut commands = Vec::new();
        self.traverse_node(&self.root, &Transform::default(), &mut commands);
        commands
    }

    fn traverse_node(
        &self,
        node: &Arc<SceneNode>,
        parent_transform: &Transform,
        commands: &mut Vec<RenderCommand>,
    ) {
        if !node.is_visible() {
            return;
        }

        // Combine parent transform with node's local transform
        let world_transform = parent_transform.combine(&node.transform);

        // Add primitives with world transform
        for primitive in node.primitives() {
            commands.push(RenderCommand {
                transform: world_transform,
                primitive: primitive.clone(),
            });
        }

        // Recursively traverse children
        for child in node.children() {
            self.traverse_node(child, &world_transform, commands);
        }
    }
}

impl Default for SceneGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// A render command combining a primitive with its world transform
#[derive(Debug, Clone)]
pub struct RenderCommand {
    /// World-space transform
    pub transform: Transform,
    /// Primitive to render
    pub primitive: Primitive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_uniqueness() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_transform_combine() {
        let t1 = Transform::translate(10.0, 20.0);
        let t2 = Transform::translate(5.0, 5.0);
        let combined = t1.combine(&t2);
        assert_eq!(combined.translation, [15.0, 25.0]);
    }

    #[test]
    fn test_scene_graph_basic() {
        let graph = SceneGraph::new();
        assert!(graph.root().is_visible());
        assert_eq!(graph.root().primitives().len(), 0);
    }

    #[test]
    fn test_dirty_tracking() {
        let mut node = SceneNode::new();
        node.clear_dirty();
        assert!(!node.is_dirty());

        node.add_primitive(Primitive::Rectangle {
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::rgb(1.0, 0.0, 0.0),
        });
        assert!(node.is_dirty());
    }

    #[test]
    fn test_visibility() {
        let mut graph = SceneGraph::new();

        graph.root_mut().add_primitive(Primitive::Rectangle {
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::rgb(1.0, 0.0, 0.0),
        });

        let commands = graph.collect_render_commands();
        assert_eq!(commands.len(), 1);

        graph.root_mut().set_visible(false);
        let commands = graph.collect_render_commands();
        assert_eq!(commands.len(), 0);
    }
}
