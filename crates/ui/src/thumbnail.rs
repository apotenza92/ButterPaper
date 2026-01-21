//! Thumbnail strip/page navigator component
//!
//! Provides a thumbnail strip UI component that displays page thumbnails
//! and allows users to navigate between pages in the PDF document.

use crate::scene::{Color, NodeId, Primitive, Rect, SceneNode};
use pdf_editor_cache::gpu::GpuTextureCache;
use pdf_editor_render::tile::{TileCoordinate, TileId, TileProfile};
use std::sync::Arc;

/// Configuration for thumbnail strip layout
#[derive(Debug, Clone)]
pub struct ThumbnailConfig {
    /// Thumbnail width in pixels
    pub thumbnail_width: f32,

    /// Thumbnail height in pixels
    pub thumbnail_height: f32,

    /// Spacing between thumbnails in pixels
    pub spacing: f32,

    /// Strip position (Left, Right, Top, Bottom)
    pub position: StripPosition,

    /// Background color for the strip
    pub background_color: Color,

    /// Border color for thumbnails
    pub border_color: Color,

    /// Border color for selected thumbnail
    pub selected_border_color: Color,

    /// Border width in pixels
    pub border_width: f32,

    /// Whether the strip is visible
    pub visible: bool,
}

impl Default for ThumbnailConfig {
    fn default() -> Self {
        Self {
            thumbnail_width: 120.0,
            thumbnail_height: 160.0,
            spacing: 8.0,
            position: StripPosition::Left,
            background_color: Color::rgba(0.15, 0.15, 0.15, 0.95),
            border_color: Color::rgba(0.4, 0.4, 0.4, 1.0),
            selected_border_color: Color::rgba(0.3, 0.6, 1.0, 1.0),
            border_width: 2.0,
            visible: true,
        }
    }
}

/// Position of the thumbnail strip
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripPosition {
    /// Left side of the window
    Left,
    /// Right side of the window
    Right,
    /// Top of the window
    Top,
    /// Bottom of the window
    Bottom,
}

/// Thumbnail strip component that displays page thumbnails for navigation
pub struct ThumbnailStrip {
    /// Configuration for layout and appearance
    config: ThumbnailConfig,

    /// GPU texture cache for thumbnail lookups
    texture_cache: Arc<GpuTextureCache>,

    /// Current page index
    current_page: u16,

    /// Total number of pages
    page_count: u16,

    /// Scene node for the thumbnail strip
    scene_node: Arc<SceneNode>,

    /// Node ID for the strip
    node_id: NodeId,

    /// Viewport dimensions (width, height)
    viewport_size: (f32, f32),

    /// Scroll offset for the thumbnail strip (for many pages)
    scroll_offset: f32,
}

impl ThumbnailStrip {
    /// Create a new thumbnail strip
    ///
    /// # Arguments
    /// * `texture_cache` - GPU texture cache for thumbnail lookups
    /// * `page_count` - Total number of pages in the document
    /// * `viewport_size` - Viewport dimensions (width, height)
    pub fn new(
        texture_cache: Arc<GpuTextureCache>,
        page_count: u16,
        viewport_size: (f32, f32),
    ) -> Self {
        let config = ThumbnailConfig::default();
        let node_id = NodeId::new();
        let scene_node = Arc::new(SceneNode::new());

        let mut strip = Self {
            config,
            texture_cache,
            current_page: 0,
            page_count,
            scene_node,
            node_id,
            viewport_size,
            scroll_offset: 0.0,
        };

        strip.rebuild();
        strip
    }

    /// Create with custom configuration
    pub fn with_config(
        texture_cache: Arc<GpuTextureCache>,
        page_count: u16,
        viewport_size: (f32, f32),
        config: ThumbnailConfig,
    ) -> Self {
        let node_id = NodeId::new();
        let scene_node = Arc::new(SceneNode::new());

        let mut strip = Self {
            config,
            texture_cache,
            current_page: 0,
            page_count,
            scene_node,
            node_id,
            viewport_size,
            scroll_offset: 0.0,
        };

        strip.rebuild();
        strip
    }

    /// Update the current page (marks selected thumbnail)
    pub fn set_current_page(&mut self, page_index: u16) {
        if page_index != self.current_page && page_index < self.page_count {
            self.current_page = page_index;
            self.auto_scroll_to_current();
            self.rebuild();
        }
    }

    /// Get the current page index
    pub fn current_page(&self) -> u16 {
        self.current_page
    }

    /// Update viewport size (e.g., on window resize)
    pub fn set_viewport_size(&mut self, width: f32, height: f32) {
        if (self.viewport_size.0 - width).abs() > 0.1
            || (self.viewport_size.1 - height).abs() > 0.1 {
            self.viewport_size = (width, height);
            self.rebuild();
        }
    }

    /// Set strip visibility
    pub fn set_visible(&mut self, visible: bool) {
        if self.config.visible != visible {
            self.config.visible = visible;
            self.rebuild();
        }
    }

    /// Check if strip is visible
    pub fn is_visible(&self) -> bool {
        self.config.visible
    }

    /// Set strip position
    pub fn set_position(&mut self, position: StripPosition) {
        if self.config.position != position {
            self.config.position = position;
            self.rebuild();
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

    /// Handle scroll input for the thumbnail strip
    pub fn scroll(&mut self, delta: f32) {
        let max_scroll = self.calculate_max_scroll();
        self.scroll_offset = (self.scroll_offset + delta).clamp(0.0, max_scroll);
        self.rebuild();
    }

    /// Auto-scroll to show the current page
    fn auto_scroll_to_current(&mut self) {
        let thumbnail_height = self.config.thumbnail_height + self.config.spacing;
        let current_pos = self.current_page as f32 * thumbnail_height;

        // Calculate visible range
        let visible_height = match self.config.position {
            StripPosition::Left | StripPosition::Right => self.viewport_size.1,
            StripPosition::Top | StripPosition::Bottom => self.viewport_size.0,
        };

        // Scroll to show current page
        if current_pos < self.scroll_offset {
            self.scroll_offset = current_pos;
        } else if current_pos + thumbnail_height > self.scroll_offset + visible_height {
            self.scroll_offset = current_pos + thumbnail_height - visible_height;
        }

        let max_scroll = self.calculate_max_scroll();
        self.scroll_offset = self.scroll_offset.clamp(0.0, max_scroll);
    }

    /// Calculate maximum scroll offset
    fn calculate_max_scroll(&self) -> f32 {
        let thumbnail_height = self.config.thumbnail_height + self.config.spacing;
        let total_height = self.page_count as f32 * thumbnail_height;

        let visible_height = match self.config.position {
            StripPosition::Left | StripPosition::Right => self.viewport_size.1,
            StripPosition::Top | StripPosition::Bottom => self.viewport_size.0,
        };

        (total_height - visible_height).max(0.0)
    }

    /// Hit test - check if a point is within a thumbnail and return page index
    pub fn hit_test(&self, x: f32, y: f32) -> Option<u16> {
        if !self.config.visible {
            return None;
        }

        let strip_rect = self.calculate_strip_bounds();

        // Check if point is within strip bounds
        if x < strip_rect.x
            || x > strip_rect.x + strip_rect.width
            || y < strip_rect.y
            || y > strip_rect.y + strip_rect.height {
            return None;
        }

        // Calculate which thumbnail was clicked
        match self.config.position {
            StripPosition::Left | StripPosition::Right => {
                let relative_y = y - strip_rect.y + self.scroll_offset;
                let thumbnail_height = self.config.thumbnail_height + self.config.spacing;
                let page_index = (relative_y / thumbnail_height).floor() as u16;

                if page_index < self.page_count {
                    Some(page_index)
                } else {
                    None
                }
            }
            StripPosition::Top | StripPosition::Bottom => {
                let relative_x = x - strip_rect.x + self.scroll_offset;
                let thumbnail_width = self.config.thumbnail_width + self.config.spacing;
                let page_index = (relative_x / thumbnail_width).floor() as u16;

                if page_index < self.page_count {
                    Some(page_index)
                } else {
                    None
                }
            }
        }
    }

    /// Calculate the bounds of the strip
    fn calculate_strip_bounds(&self) -> Rect {
        let (viewport_width, viewport_height) = self.viewport_size;

        match self.config.position {
            StripPosition::Left => Rect::new(
                0.0,
                0.0,
                self.config.thumbnail_width + self.config.spacing * 2.0,
                viewport_height,
            ),
            StripPosition::Right => Rect::new(
                viewport_width - (self.config.thumbnail_width + self.config.spacing * 2.0),
                0.0,
                self.config.thumbnail_width + self.config.spacing * 2.0,
                viewport_height,
            ),
            StripPosition::Top => Rect::new(
                0.0,
                0.0,
                viewport_width,
                self.config.thumbnail_height + self.config.spacing * 2.0,
            ),
            StripPosition::Bottom => Rect::new(
                0.0,
                viewport_height - (self.config.thumbnail_height + self.config.spacing * 2.0),
                viewport_width,
                self.config.thumbnail_height + self.config.spacing * 2.0,
            ),
        }
    }

    /// Rebuild the scene node with current state
    fn rebuild(&mut self) {
        let mut new_node = SceneNode::new();

        if !self.config.visible {
            new_node.set_visible(false);
            self.scene_node = Arc::new(new_node);
            return;
        }

        let strip_rect = self.calculate_strip_bounds();

        // Add background rectangle
        new_node.add_primitive(Primitive::Rectangle {
            rect: strip_rect,
            color: self.config.background_color,
        });

        // Calculate thumbnail layout based on position
        let (start_x, start_y, dx, dy) = match self.config.position {
            StripPosition::Left | StripPosition::Right => {
                let x = strip_rect.x + self.config.spacing;
                let y = strip_rect.y + self.config.spacing - self.scroll_offset;
                (x, y, 0.0, self.config.thumbnail_height + self.config.spacing)
            }
            StripPosition::Top | StripPosition::Bottom => {
                let x = strip_rect.x + self.config.spacing - self.scroll_offset;
                let y = strip_rect.y + self.config.spacing;
                (x, y, self.config.thumbnail_width + self.config.spacing, 0.0)
            }
        };

        // Add thumbnails for all pages
        for page_index in 0..self.page_count {
            let thumb_x = start_x + dx * page_index as f32;
            let thumb_y = start_y + dy * page_index as f32;

            // Check if thumbnail is visible in current scroll position
            if !self.is_thumbnail_visible(thumb_x, thumb_y, &strip_rect) {
                continue;
            }

            // Border color (highlight current page)
            let border_color = if page_index == self.current_page {
                self.config.selected_border_color
            } else {
                self.config.border_color
            };

            // Border rectangle
            new_node.add_primitive(Primitive::Rectangle {
                rect: Rect::new(
                    thumb_x - self.config.border_width,
                    thumb_y - self.config.border_width,
                    self.config.thumbnail_width + 2.0 * self.config.border_width,
                    self.config.thumbnail_height + 2.0 * self.config.border_width,
                ),
                color: border_color,
            });

            // Try to get thumbnail texture from cache
            // Thumbnails are rendered at a fixed small zoom level (e.g., 25%)
            let thumbnail_tile_id = self.create_thumbnail_tile_id(page_index);

            if let Some(_texture) = self.texture_cache.try_get(thumbnail_tile_id.cache_key()) {
                // Render thumbnail texture
                new_node.add_primitive(Primitive::TexturedQuad {
                    rect: Rect::new(
                        thumb_x,
                        thumb_y,
                        self.config.thumbnail_width,
                        self.config.thumbnail_height,
                    ),
                    texture_id: thumbnail_tile_id.cache_key(),
                });
            } else {
                // Placeholder when thumbnail not yet cached
                new_node.add_primitive(Primitive::Rectangle {
                    rect: Rect::new(
                        thumb_x,
                        thumb_y,
                        self.config.thumbnail_width,
                        self.config.thumbnail_height,
                    ),
                    color: Color::rgba(0.25, 0.25, 0.25, 1.0),
                });
            }
        }

        self.scene_node = Arc::new(new_node);
    }

    /// Check if a thumbnail at given position is visible within strip bounds
    fn is_thumbnail_visible(&self, thumb_x: f32, thumb_y: f32, strip_rect: &Rect) -> bool {
        match self.config.position {
            StripPosition::Left | StripPosition::Right => {
                thumb_y + self.config.thumbnail_height >= strip_rect.y
                    && thumb_y <= strip_rect.y + strip_rect.height
            }
            StripPosition::Top | StripPosition::Bottom => {
                thumb_x + self.config.thumbnail_width >= strip_rect.x
                    && thumb_x <= strip_rect.x + strip_rect.width
            }
        }
    }

    /// Create a tile ID for a thumbnail (single tile at low zoom)
    fn create_thumbnail_tile_id(&self, page_index: u16) -> TileId {
        // Thumbnails use a single tile at 25% zoom with preview profile
        TileId::new(
            page_index,
            TileCoordinate { x: 0, y: 0 },
            25, // 25% zoom for thumbnails
            0,  // No rotation
            TileProfile::Preview,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_editor_cache::config::CacheConfig;

    #[test]
    fn test_thumbnail_strip_creation() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        assert_eq!(strip.page_count, 10);
        assert_eq!(strip.current_page, 0);
        assert!(strip.is_visible());
    }

    #[test]
    fn test_set_current_page() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        strip.set_current_page(5);
        assert_eq!(strip.current_page(), 5);

        // Out of bounds should not change
        strip.set_current_page(20);
        assert_eq!(strip.current_page(), 5);
    }

    #[test]
    fn test_visibility_toggle() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        assert!(strip.is_visible());
        strip.set_visible(false);
        assert!(!strip.is_visible());
        strip.set_visible(true);
        assert!(strip.is_visible());
    }

    #[test]
    fn test_viewport_resize() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        assert_eq!(strip.viewport_size, (1200.0, 800.0));
        strip.set_viewport_size(1920.0, 1080.0);
        assert_eq!(strip.viewport_size, (1920.0, 1080.0));
    }

    #[test]
    fn test_position_change() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        strip.set_position(StripPosition::Right);
        strip.set_position(StripPosition::Top);
        strip.set_position(StripPosition::Bottom);
        strip.set_position(StripPosition::Left);
    }

    #[test]
    fn test_scroll() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 50, (1200.0, 800.0));

        assert_eq!(strip.scroll_offset, 0.0);
        strip.scroll(100.0);
        assert!(strip.scroll_offset > 0.0);

        // Scroll should be clamped to max
        strip.scroll(100000.0);
        let max_scroll = strip.calculate_max_scroll();
        assert_eq!(strip.scroll_offset, max_scroll);

        // Scroll back
        strip.scroll(-100000.0);
        assert_eq!(strip.scroll_offset, 0.0);
    }

    #[test]
    fn test_hit_test_left_position() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));
        strip.set_position(StripPosition::Left);

        // Click on first thumbnail (should be around y=8 to y=168)
        let result = strip.hit_test(60.0, 80.0);
        assert_eq!(result, Some(0));

        // Click outside strip
        let result = strip.hit_test(200.0, 80.0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_hit_test_when_invisible() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        strip.set_visible(false);
        let result = strip.hit_test(60.0, 80.0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_thumbnail_tile_id() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        let tile_id = strip.create_thumbnail_tile_id(3);
        assert_eq!(tile_id.page_index, 3);
        assert_eq!(tile_id.zoom_level, 25);
        assert_eq!(tile_id.profile, TileProfile::Preview);
    }

    #[test]
    fn test_custom_config() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));

        let custom_config = ThumbnailConfig {
            thumbnail_width: 200.0,
            thumbnail_height: 250.0,
            position: StripPosition::Right,
            ..Default::default()
        };

        let strip = ThumbnailStrip::with_config(
            cache,
            10,
            (1200.0, 800.0),
            custom_config,
        );

        assert_eq!(strip.config.thumbnail_width, 200.0);
        assert_eq!(strip.config.thumbnail_height, 250.0);
        assert_eq!(strip.config.position, StripPosition::Right);
    }
}
