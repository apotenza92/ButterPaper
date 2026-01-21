// Viewport compositor - combines tiles, annotations, labels, and guides into a GPU-rendered scene
//
// The compositor is the central orchestration layer that takes:
// - Viewport state (position, zoom, page)
// - Cached tile textures from GPU texture cache
// - Annotation geometry (when Phase 7 is implemented)
// - Measurement labels and guides (when Phase 8 is implemented)
//
// And produces a SceneGraph for GPU rendering.

use crate::scene::{NodeId, Primitive, Rect, SceneGraph, SceneNode};
use pdf_editor_cache::gpu::GpuTextureCache;
use pdf_editor_render::tile::{TileCoordinate, TileId, TileProfile, TILE_SIZE};
use pdf_editor_scheduler::Viewport;
use std::sync::Arc;

/// Viewport compositor - composes tiles and annotations into a scene graph
pub struct ViewportCompositor {
    /// GPU texture cache for tile lookups
    texture_cache: Arc<GpuTextureCache>,

    /// Root scene graph (retained across frames)
    scene_graph: SceneGraph,

    /// Node ID for tile layer (reserved for future use in node updating)
    #[allow(dead_code)]
    tile_layer_id: NodeId,

    /// Node ID for annotation layer (reserved for future use in node updating)
    #[allow(dead_code)]
    annotation_layer_id: NodeId,

    /// Node ID for guide layer (reserved for future use in node updating)
    #[allow(dead_code)]
    guide_layer_id: NodeId,

    /// Node ID for label layer (reserved for future use in node updating)
    #[allow(dead_code)]
    label_layer_id: NodeId,

    /// Current viewport state (cached to detect changes)
    current_viewport: Option<Viewport>,
}

impl ViewportCompositor {
    /// Create a new viewport compositor
    pub fn new(texture_cache: Arc<GpuTextureCache>) -> Self {
        let _scene_graph = SceneGraph::new();

        // Create layered structure:
        // - root
        //   - tile_layer (bottom - rendered first)
        //   - annotation_layer (middle)
        //   - guide_layer (above annotations)
        //   - label_layer (top - rendered last, always visible)

        let tile_layer_id = NodeId::new();
        let annotation_layer_id = NodeId::new();
        let guide_layer_id = NodeId::new();
        let label_layer_id = NodeId::new();

        // Build initial scene graph structure
        let tile_layer = SceneNode::new();

        let annotation_layer = SceneNode::new();

        let guide_layer = SceneNode::new();

        let label_layer = SceneNode::new();

        // Attach layers to root (order matters for rendering)
        let mut root = SceneNode::new();
        root.add_child(Arc::new(tile_layer));
        root.add_child(Arc::new(annotation_layer));
        root.add_child(Arc::new(guide_layer));
        root.add_child(Arc::new(label_layer));

        let mut scene_graph = SceneGraph::new();
        *scene_graph.root_mut() = root;

        Self {
            texture_cache,
            scene_graph,
            tile_layer_id,
            annotation_layer_id,
            guide_layer_id,
            label_layer_id,
            current_viewport: None,
        }
    }

    /// Update the compositor with new viewport state
    ///
    /// Returns true if the scene graph was modified (needs re-render)
    pub fn update(&mut self, viewport: &Viewport) -> bool {
        // Check if viewport changed
        let viewport_changed = match &self.current_viewport {
            Some(current) => !viewport_equals(current, viewport),
            None => true,
        };

        if !viewport_changed {
            return false;
        }

        // Update viewport
        self.current_viewport = Some(viewport.clone());

        // Rebuild tile layer
        self.rebuild_tile_layer(viewport);

        // Rebuild annotation layer (Phase 7 - placeholder for now)
        self.rebuild_annotation_layer(viewport);

        // Rebuild guide layer (Phase 8 - placeholder for now)
        self.rebuild_guide_layer(viewport);

        // Rebuild label layer (Phase 8 - placeholder for now)
        self.rebuild_label_layer(viewport);

        true
    }

    /// Get the scene graph for rendering
    pub fn scene_graph(&self) -> &SceneGraph {
        &self.scene_graph
    }

    /// Rebuild the tile layer based on viewport
    fn rebuild_tile_layer(&mut self, viewport: &Viewport) {
        let mut tile_primitives = Vec::new();

        // Calculate visible tile range
        let tiles = calculate_visible_tiles(viewport);

        // Query GPU texture cache for each tile
        for tile_id in tiles {
            if let Some(texture) = self.texture_cache.try_get(tile_id.cache_key()) {
                // Convert tile coordinate to viewport pixel position
                let (tile_x, tile_y) = tile_id.coordinate.to_pixel_offset(TILE_SIZE);

                // Apply viewport transform (pan, zoom)
                let zoom_scale = viewport.zoom_level as f32 / 100.0;
                let screen_x = (tile_x as f32 * zoom_scale) - viewport.x;
                let screen_y = (tile_y as f32 * zoom_scale) - viewport.y;

                let metadata = texture.metadata();
                let tile_width = metadata.width as f32;
                let tile_height = metadata.height as f32;

                // Create textured quad primitive
                let primitive = Primitive::TexturedQuad {
                    rect: Rect {
                        x: screen_x,
                        y: screen_y,
                        width: tile_width,
                        height: tile_height,
                    },
                    texture_id: tile_id.cache_key(),
                };

                tile_primitives.push(primitive);
            }
            // If tile not in cache, render job should be submitted by job scheduler
            // The compositor only renders what's available in cache
        }

        // Update tile layer node
        let mut tile_layer = SceneNode::new();
        tile_layer.set_primitives(tile_primitives);

        self.update_layer(0, Arc::new(tile_layer));
    }

    /// Rebuild annotation layer (Phase 7 - placeholder)
    fn rebuild_annotation_layer(&mut self, _viewport: &Viewport) {
        // Phase 7 will add:
        // - Query annotation database for visible annotations
        // - Convert annotation geometry to GPU primitives
        // - Apply viewport transform to annotation coordinates
        // - Render as lines, rectangles, circles, etc.

        // For now, create empty layer
        let annotation_layer = SceneNode::new();

        self.update_layer(1, Arc::new(annotation_layer));
    }

    /// Rebuild guide layer (Phase 8 - placeholder)
    fn rebuild_guide_layer(&mut self, _viewport: &Viewport) {
        // Phase 8 will add:
        // - Render measurement guides (snapping lines)
        // - Render scale indicators
        // - Render grid overlays (if enabled)

        // For now, create empty layer
        let guide_layer = SceneNode::new();

        self.update_layer(2, Arc::new(guide_layer));
    }

    /// Rebuild label layer (Phase 8 - placeholder)
    fn rebuild_label_layer(&mut self, _viewport: &Viewport) {
        // Phase 8 will add:
        // - Render measurement labels with real-time values
        // - Render scale text
        // - Render annotation tooltips

        // For now, create empty layer
        let label_layer = SceneNode::new();

        self.update_layer(3, Arc::new(label_layer));
    }

    /// Update a specific layer in the scene graph
    fn update_layer(&mut self, layer_index: usize, new_layer: Arc<SceneNode>) {
        // Get current root
        let current_root = self.scene_graph.root();

        // Clone children and replace the specified layer
        let mut new_children: Vec<Arc<SceneNode>> = current_root
            .children()
            .iter()
            .enumerate()
            .map(|(i, child)| {
                if i == layer_index {
                    Arc::clone(&new_layer)
                } else {
                    Arc::clone(child)
                }
            })
            .collect();

        // If layer_index >= children.len(), pad with empty layers
        while new_children.len() <= layer_index {
            new_children.push(Arc::new(SceneNode::new()));
        }

        // Set the target layer
        new_children[layer_index] = new_layer;

        // Create new root with updated children
        let mut new_root = SceneNode::new();
        new_root.set_transform(*current_root.transform());
        new_root.set_primitives(current_root.primitives().to_vec());
        for child in new_children {
            new_root.add_child(child);
        }

        *self.scene_graph.root_mut() = new_root;
    }

    /// Add an annotation primitive to the annotation layer
    /// (Phase 7 - exposed for future annotation engine integration)
    pub fn add_annotation(&mut self, primitive: Primitive) {
        let current_layer = self.get_layer(1);
        let mut primitives = current_layer.primitives().to_vec();
        primitives.push(primitive);

        let mut new_layer = SceneNode::new();
        new_layer.set_transform(*current_layer.transform());
        new_layer.set_primitives(primitives);
        for child in current_layer.children() {
            new_layer.add_child(Arc::clone(child));
        }

        self.update_layer(1, Arc::new(new_layer));
    }

    /// Add a guide primitive to the guide layer
    /// (Phase 8 - exposed for future measurement engine integration)
    pub fn add_guide(&mut self, primitive: Primitive) {
        let current_layer = self.get_layer(2);
        let mut primitives = current_layer.primitives().to_vec();
        primitives.push(primitive);

        let mut new_layer = SceneNode::new();
        new_layer.set_transform(*current_layer.transform());
        new_layer.set_primitives(primitives);
        for child in current_layer.children() {
            new_layer.add_child(Arc::clone(child));
        }

        self.update_layer(2, Arc::new(new_layer));
    }

    /// Add a label primitive to the label layer
    /// (Phase 8 - exposed for future measurement engine integration)
    pub fn add_label(&mut self, primitive: Primitive) {
        let current_layer = self.get_layer(3);
        let mut primitives = current_layer.primitives().to_vec();
        primitives.push(primitive);

        let mut new_layer = SceneNode::new();
        new_layer.set_transform(*current_layer.transform());
        new_layer.set_primitives(primitives);
        for child in current_layer.children() {
            new_layer.add_child(Arc::clone(child));
        }

        self.update_layer(3, Arc::new(new_layer));
    }

    /// Get a layer by index
    fn get_layer(&self, layer_index: usize) -> Arc<SceneNode> {
        let root = self.scene_graph.root();
        Arc::clone(&root.children()[layer_index])
    }
}

/// Calculate visible tiles for the current viewport
fn calculate_visible_tiles(viewport: &Viewport) -> Vec<TileId> {
    let mut tiles = Vec::new();

    // Calculate tile range from viewport bounds
    let zoom_scale = viewport.zoom_level as f32 / 100.0;
    let start_tile_x = (viewport.x / (TILE_SIZE as f32 * zoom_scale)).floor() as u32;
    let start_tile_y = (viewport.y / (TILE_SIZE as f32 * zoom_scale)).floor() as u32;

    let end_tile_x = ((viewport.x + viewport.width) / (TILE_SIZE as f32 * zoom_scale)).ceil() as u32;
    let end_tile_y = ((viewport.y + viewport.height) / (TILE_SIZE as f32 * zoom_scale)).ceil() as u32;

    // Iterate through visible tile grid
    for tile_y in start_tile_y..=end_tile_y {
        for tile_x in start_tile_x..=end_tile_x {
            let tile_id = TileId::new(
                viewport.page_index,
                TileCoordinate {
                    x: tile_x,
                    y: tile_y,
                },
                viewport.zoom_level,
                0, // No rotation for now (Phase 6 will add rotation)
                TileProfile::Crisp, // Prefer crisp tiles for visible viewport
            );

            tiles.push(tile_id);
        }
    }

    tiles
}

/// Check if two viewports are equal (for change detection)
fn viewport_equals(a: &Viewport, b: &Viewport) -> bool {
    a.page_index == b.page_index
        && (a.x - b.x).abs() < 0.001
        && (a.y - b.y).abs() < 0.001
        && a.width == b.width
        && a.height == b.height
        && a.zoom_level == b.zoom_level
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Color;
    use pdf_editor_cache::config::CacheConfig;

    #[test]
    fn test_compositor_creation() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let compositor = ViewportCompositor::new(cache);

        // Verify scene graph structure
        let root = compositor.scene_graph().root();
        assert_eq!(root.children().len(), 4, "Should have 4 layers");
    }

    #[test]
    fn test_viewport_update() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut compositor = ViewportCompositor::new(cache);

        let viewport = Viewport::new(0, 0.0, 0.0, 1024.0, 768.0, 100);

        // First update should return true (viewport changed)
        assert!(compositor.update(&viewport));

        // Second update with same viewport should return false
        assert!(!compositor.update(&viewport));

        // Update with different viewport should return true
        let new_viewport = Viewport::new(0, 100.0, 50.0, 1024.0, 768.0, 100);
        assert!(compositor.update(&new_viewport));
    }

    #[test]
    fn test_calculate_visible_tiles() {
        let viewport = Viewport::new(0, 0.0, 0.0, 512.0, 512.0, 100);

        let tiles = calculate_visible_tiles(&viewport);

        // At 100% zoom, 512x512 viewport should cover 2x2 tiles (256px each)
        assert!(tiles.len() >= 4, "Should have at least 4 visible tiles");

        // Verify all tiles are on page 0
        assert!(tiles.iter().all(|t| t.page_index == 0));
    }

    #[test]
    fn test_viewport_equals() {
        let v1 = Viewport::new(0, 100.0, 200.0, 1024.0, 768.0, 150);

        let v2 = v1.clone();
        assert!(viewport_equals(&v1, &v2));

        let v3 = Viewport::new(1, 100.0, 200.0, 1024.0, 768.0, 150);
        assert!(!viewport_equals(&v1, &v3));

        let v4 = Viewport::new(0, 101.0, 200.0, 1024.0, 768.0, 150);
        assert!(!viewport_equals(&v1, &v4));
    }

    #[test]
    fn test_add_annotation() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut compositor = ViewportCompositor::new(cache);

        // Add an annotation
        let annotation = Primitive::Line {
            start: [0.0, 0.0],
            end: [100.0, 100.0],
            width: 2.0,
            color: Color {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        };

        compositor.add_annotation(annotation);

        // Verify annotation layer has 1 primitive
        let annotation_layer = compositor.get_layer(1);
        assert_eq!(annotation_layer.primitives().len(), 1);
    }

    #[test]
    fn test_layer_ordering() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let compositor = ViewportCompositor::new(cache);

        let root = compositor.scene_graph().root();
        let layers = root.children();

        // Verify layer IDs match expected order
        assert_eq!(layers[0].id(), compositor.tile_layer_id);
        assert_eq!(layers[1].id(), compositor.annotation_layer_id);
        assert_eq!(layers[2].id(), compositor.guide_layer_id);
        assert_eq!(layers[3].id(), compositor.label_layer_id);
    }
}
