//! Thumbnail strip/page navigator component
//!
//! Provides a thumbnail strip UI component that displays page thumbnails
//! and allows users to navigate between pages in the PDF document.
//!
//! The sidebar supports smooth scrolling with momentum-based animation,
//! providing a natural, fluid scrolling experience similar to native
//! macOS scrolling behavior.

use crate::scene::{Color, NodeId, Primitive, Rect, SceneNode};
use pdf_editor_cache::gpu::GpuTextureCache;
use pdf_editor_render::tile::{TileCoordinate, TileId, TileProfile};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

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

/// State for smooth scroll animation
#[derive(Debug, Clone)]
struct ScrollState {
    /// Current scroll offset (visual position)
    offset: f32,

    /// Target scroll offset (where we're animating toward)
    target_offset: f32,

    /// Scroll velocity (pixels per second) for momentum scrolling
    velocity: f32,

    /// Interpolation speed (0.0 - 1.0) - fraction of remaining distance per frame
    interpolation_speed: f32,

    /// Velocity decay factor (0.0 - 1.0) for momentum deceleration
    momentum_decay: f32,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            offset: 0.0,
            target_offset: 0.0,
            velocity: 0.0,
            interpolation_speed: 0.15, // 15% of remaining distance per frame
            momentum_decay: 0.92,      // Decay to 8% per frame at 60fps
        }
    }
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

    /// Smooth scroll state
    scroll_state: ScrollState,

    /// Set of page indices for which thumbnail rendering has been requested
    /// but not yet completed. Used to avoid duplicate render requests.
    pending_requests: HashSet<u16>,
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
            scroll_state: ScrollState::default(),
            pending_requests: HashSet::new(),
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
            scroll_state: ScrollState::default(),
            pending_requests: HashSet::new(),
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
        if (self.viewport_size.0 - width).abs() > 0.1 || (self.viewport_size.1 - height).abs() > 0.1
        {
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

    /// Handle scroll input for the thumbnail strip (with momentum)
    ///
    /// This initiates smooth scrolling by adding velocity to the scroll state.
    /// Call `update()` each frame to animate the scroll.
    pub fn scroll(&mut self, delta: f32) {
        // Add to velocity for momentum-based scrolling
        // Scale delta for natural feeling (larger delta = more momentum)
        self.scroll_state.velocity += delta * 60.0; // Convert to per-second velocity
    }

    /// Handle scroll input for the thumbnail strip (immediate, no animation)
    ///
    /// Use this for direct scroll position changes without animation.
    pub fn scroll_immediate(&mut self, delta: f32) {
        let max_scroll = self.calculate_max_scroll();
        self.scroll_state.offset = (self.scroll_state.offset + delta).clamp(0.0, max_scroll);
        self.scroll_state.target_offset = self.scroll_state.offset;
        self.scroll_state.velocity = 0.0;
        self.rebuild();
    }

    /// Scroll to a specific offset with smooth animation
    pub fn scroll_to(&mut self, offset: f32) {
        let max_scroll = self.calculate_max_scroll();
        self.scroll_state.target_offset = offset.clamp(0.0, max_scroll);
        // Stop any existing momentum
        self.scroll_state.velocity = 0.0;
    }

    /// Get the current scroll offset (visual position)
    pub fn scroll_offset(&self) -> f32 {
        self.scroll_state.offset
    }

    /// Get the target scroll offset (where animation is heading)
    pub fn target_scroll_offset(&self) -> f32 {
        self.scroll_state.target_offset
    }

    /// Check if scroll animation is currently in progress
    pub fn is_scroll_animating(&self) -> bool {
        let offset_diff = (self.scroll_state.offset - self.scroll_state.target_offset).abs();
        let has_velocity = self.scroll_state.velocity.abs() > 0.5;
        offset_diff > 0.5 || has_velocity
    }

    /// Update animation state (call every frame)
    ///
    /// Returns true if the scroll position changed and a rebuild is needed.
    pub fn update(&mut self, delta_time: Duration) -> bool {
        let delta_seconds = delta_time.as_secs_f32();
        let max_scroll = self.calculate_max_scroll();
        let mut changed = false;

        // Apply momentum-based scrolling
        if self.scroll_state.velocity.abs() > 0.5 {
            // Apply velocity decay
            self.scroll_state.velocity *= self.scroll_state.momentum_decay;

            // Update target based on velocity
            let velocity_delta = self.scroll_state.velocity * delta_seconds;
            self.scroll_state.target_offset =
                (self.scroll_state.target_offset + velocity_delta).clamp(0.0, max_scroll);

            // If we hit the bounds, stop velocity
            if self.scroll_state.target_offset <= 0.0
                || self.scroll_state.target_offset >= max_scroll
            {
                self.scroll_state.velocity = 0.0;
            }

            changed = true;
        } else {
            // Stop momentum completely when very small
            self.scroll_state.velocity = 0.0;
        }

        // Smooth interpolation toward target
        let offset_diff = self.scroll_state.target_offset - self.scroll_state.offset;
        if offset_diff.abs() > 0.5 {
            // Interpolate toward target
            let new_offset = self.scroll_state.offset
                + offset_diff * self.scroll_state.interpolation_speed;
            self.scroll_state.offset = new_offset.clamp(0.0, max_scroll);
            changed = true;
        } else if offset_diff.abs() > 0.01 {
            // Snap to target when very close
            self.scroll_state.offset = self.scroll_state.target_offset;
            changed = true;
        }

        // Rebuild scene if scroll position changed
        if changed {
            self.rebuild();
        }

        changed
    }

    /// Auto-scroll to show the current page with smooth animation
    fn auto_scroll_to_current(&mut self) {
        let thumbnail_height = self.config.thumbnail_height + self.config.spacing;
        let current_pos = self.current_page as f32 * thumbnail_height;

        // Calculate visible range
        let visible_height = match self.config.position {
            StripPosition::Left | StripPosition::Right => self.viewport_size.1,
            StripPosition::Top | StripPosition::Bottom => self.viewport_size.0,
        };

        // Calculate target scroll to show current page
        let current_offset = self.scroll_state.offset;
        let mut target = self.scroll_state.target_offset;

        if current_pos < current_offset {
            // Page is above visible area, scroll up
            target = current_pos;
        } else if current_pos + thumbnail_height > current_offset + visible_height {
            // Page is below visible area, scroll down
            target = current_pos + thumbnail_height - visible_height;
        }

        let max_scroll = self.calculate_max_scroll();
        self.scroll_state.target_offset = target.clamp(0.0, max_scroll);
        // Stop any existing momentum when programmatically scrolling
        self.scroll_state.velocity = 0.0;
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
            || y > strip_rect.y + strip_rect.height
        {
            return None;
        }

        // Calculate which thumbnail was clicked
        match self.config.position {
            StripPosition::Left | StripPosition::Right => {
                let relative_y = y - strip_rect.y + self.scroll_state.offset;
                let thumbnail_height = self.config.thumbnail_height + self.config.spacing;
                let page_index = (relative_y / thumbnail_height).floor() as u16;

                if page_index < self.page_count {
                    Some(page_index)
                } else {
                    None
                }
            }
            StripPosition::Top | StripPosition::Bottom => {
                let relative_x = x - strip_rect.x + self.scroll_state.offset;
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
                let y = strip_rect.y + self.config.spacing - self.scroll_state.offset;
                (
                    x,
                    y,
                    0.0,
                    self.config.thumbnail_height + self.config.spacing,
                )
            }
            StripPosition::Top | StripPosition::Bottom => {
                let x = strip_rect.x + self.config.spacing - self.scroll_state.offset;
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

    // =========================================================================
    // Lazy Loading API
    // =========================================================================

    /// Get a list of page indices for thumbnails that need to be rendered.
    ///
    /// This implements the "visible first" lazy loading strategy:
    /// 1. Returns only thumbnails that are currently visible in the viewport
    /// 2. Excludes thumbnails that are already cached
    /// 3. Excludes thumbnails that have already been requested (pending)
    /// 4. Orders results by distance from current page (closest first)
    ///
    /// The caller should use this list to submit rendering jobs to the scheduler
    /// with `JobPriority::Thumbnails`, then call `mark_thumbnail_requested()`
    /// for each submitted job.
    ///
    /// # Returns
    /// A vector of page indices that need thumbnail rendering, ordered by priority
    /// (closest to current page first).
    pub fn get_needed_thumbnails(&self) -> Vec<u16> {
        if !self.config.visible || self.page_count == 0 {
            return Vec::new();
        }

        let strip_rect = self.calculate_strip_bounds();
        let mut needed: Vec<u16> = Vec::new();

        // Calculate thumbnail positions
        let (start_x, start_y, dx, dy) = match self.config.position {
            StripPosition::Left | StripPosition::Right => {
                let x = strip_rect.x + self.config.spacing;
                let y = strip_rect.y + self.config.spacing - self.scroll_state.offset;
                (
                    x,
                    y,
                    0.0,
                    self.config.thumbnail_height + self.config.spacing,
                )
            }
            StripPosition::Top | StripPosition::Bottom => {
                let x = strip_rect.x + self.config.spacing - self.scroll_state.offset;
                let y = strip_rect.y + self.config.spacing;
                (x, y, self.config.thumbnail_width + self.config.spacing, 0.0)
            }
        };

        // Find visible thumbnails that need rendering
        for page_index in 0..self.page_count {
            let thumb_x = start_x + dx * page_index as f32;
            let thumb_y = start_y + dy * page_index as f32;

            // Skip if not visible
            if !self.is_thumbnail_visible(thumb_x, thumb_y, &strip_rect) {
                continue;
            }

            // Skip if already in cache
            let tile_id = self.create_thumbnail_tile_id(page_index);
            if self.texture_cache.try_get(tile_id.cache_key()).is_some() {
                continue;
            }

            // Skip if already requested
            if self.pending_requests.contains(&page_index) {
                continue;
            }

            needed.push(page_index);
        }

        // Sort by distance from current page (closest first)
        let current = self.current_page;
        needed.sort_by_key(|&page| (page as i32 - current as i32).abs());

        needed
    }

    /// Get thumbnails needed for prefetching (pages adjacent to visible area).
    ///
    /// This returns page indices for thumbnails that are just outside the
    /// visible area but likely to become visible soon during scrolling.
    /// These should be rendered at a lower priority than visible thumbnails.
    ///
    /// # Arguments
    /// * `prefetch_count` - Number of pages to prefetch above and below visible area
    ///
    /// # Returns
    /// A vector of page indices that should be prefetched, ordered by distance
    /// from visible area (closest first).
    pub fn get_prefetch_thumbnails(&self, prefetch_count: u16) -> Vec<u16> {
        if !self.config.visible || self.page_count == 0 {
            return Vec::new();
        }

        let strip_rect = self.calculate_strip_bounds();
        let mut prefetch: Vec<u16> = Vec::new();

        // Calculate which pages are visible
        let (first_visible, last_visible) = self.get_visible_page_range(&strip_rect);

        // Get pages just before visible area
        let prefetch_start = first_visible.saturating_sub(prefetch_count);
        for page in prefetch_start..first_visible {
            let tile_id = self.create_thumbnail_tile_id(page);
            if self.texture_cache.try_get(tile_id.cache_key()).is_none()
                && !self.pending_requests.contains(&page)
            {
                prefetch.push(page);
            }
        }

        // Get pages just after visible area
        let prefetch_end = (last_visible + prefetch_count + 1).min(self.page_count);
        for page in (last_visible + 1)..prefetch_end {
            let tile_id = self.create_thumbnail_tile_id(page);
            if self.texture_cache.try_get(tile_id.cache_key()).is_none()
                && !self.pending_requests.contains(&page)
            {
                prefetch.push(page);
            }
        }

        // Sort by distance from visible area
        let mid_visible = (first_visible + last_visible) / 2;
        prefetch.sort_by_key(|&page| (page as i32 - mid_visible as i32).abs());

        prefetch
    }

    /// Get the range of visible page indices.
    ///
    /// # Returns
    /// Tuple of (first_visible_page, last_visible_page)
    fn get_visible_page_range(&self, strip_rect: &Rect) -> (u16, u16) {
        let thumbnail_size = match self.config.position {
            StripPosition::Left | StripPosition::Right => {
                self.config.thumbnail_height + self.config.spacing
            }
            StripPosition::Top | StripPosition::Bottom => {
                self.config.thumbnail_width + self.config.spacing
            }
        };

        let visible_size = match self.config.position {
            StripPosition::Left | StripPosition::Right => strip_rect.height,
            StripPosition::Top | StripPosition::Bottom => strip_rect.width,
        };

        // Calculate first and last visible page based on scroll offset
        let first_visible = (self.scroll_state.offset / thumbnail_size).floor() as u16;
        let visible_count = (visible_size / thumbnail_size).ceil() as u16 + 1;
        let last_visible = (first_visible + visible_count).min(self.page_count.saturating_sub(1));

        (first_visible.min(self.page_count.saturating_sub(1)), last_visible)
    }

    /// Mark a thumbnail as having a pending render request.
    ///
    /// Call this after submitting a thumbnail render job to the scheduler.
    /// This prevents duplicate render requests for the same thumbnail.
    ///
    /// # Arguments
    /// * `page_index` - The page index of the thumbnail being rendered
    pub fn mark_thumbnail_requested(&mut self, page_index: u16) {
        if page_index < self.page_count {
            self.pending_requests.insert(page_index);
        }
    }

    /// Mark multiple thumbnails as having pending render requests.
    ///
    /// Convenience method for marking multiple thumbnails at once.
    ///
    /// # Arguments
    /// * `page_indices` - Iterator of page indices to mark as requested
    pub fn mark_thumbnails_requested<I>(&mut self, page_indices: I)
    where
        I: IntoIterator<Item = u16>,
    {
        for page_index in page_indices {
            self.mark_thumbnail_requested(page_index);
        }
    }

    /// Mark a thumbnail render as completed (successfully or not).
    ///
    /// Call this when a thumbnail render job completes to clear its pending state.
    /// The thumbnail will be displayed from the cache if rendering succeeded,
    /// or can be re-requested if it failed.
    ///
    /// # Arguments
    /// * `page_index` - The page index of the completed thumbnail
    pub fn mark_thumbnail_loaded(&mut self, page_index: u16) {
        self.pending_requests.remove(&page_index);
    }

    /// Clear all pending thumbnail requests.
    ///
    /// Call this when the document changes or thumbnails need to be re-rendered.
    pub fn clear_pending_requests(&mut self) {
        self.pending_requests.clear();
    }

    /// Check if a specific thumbnail has a pending render request.
    ///
    /// # Arguments
    /// * `page_index` - The page index to check
    ///
    /// # Returns
    /// `true` if a render request is pending for this thumbnail
    pub fn is_thumbnail_pending(&self, page_index: u16) -> bool {
        self.pending_requests.contains(&page_index)
    }

    /// Get the number of pending thumbnail requests.
    pub fn pending_count(&self) -> usize {
        self.pending_requests.len()
    }

    /// Get the cache key for a thumbnail's tile.
    ///
    /// This can be used by the caller to insert rendered thumbnails into the cache.
    ///
    /// # Arguments
    /// * `page_index` - The page index
    ///
    /// # Returns
    /// The cache key for the thumbnail tile
    pub fn thumbnail_cache_key(&self, page_index: u16) -> u64 {
        self.create_thumbnail_tile_id(page_index).cache_key()
    }

    /// Get the zoom level used for thumbnail rendering.
    ///
    /// This is useful for the caller to calculate the render dimensions.
    pub fn thumbnail_zoom_level(&self) -> u32 {
        25 // 25% zoom for thumbnails
    }

    /// Get the configured thumbnail dimensions.
    ///
    /// # Returns
    /// Tuple of (width, height) in pixels
    pub fn thumbnail_dimensions(&self) -> (f32, f32) {
        (self.config.thumbnail_width, self.config.thumbnail_height)
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
    fn test_scroll_immediate() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 50, (1200.0, 800.0));

        assert_eq!(strip.scroll_offset(), 0.0);
        strip.scroll_immediate(100.0);
        assert!(strip.scroll_offset() > 0.0);

        // Scroll should be clamped to max
        strip.scroll_immediate(100000.0);
        let max_scroll = strip.calculate_max_scroll();
        assert_eq!(strip.scroll_offset(), max_scroll);

        // Scroll back
        strip.scroll_immediate(-100000.0);
        assert_eq!(strip.scroll_offset(), 0.0);
    }

    #[test]
    fn test_smooth_scroll_animation() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 50, (1200.0, 800.0));

        // Start with no scroll
        assert_eq!(strip.scroll_offset(), 0.0);
        assert!(!strip.is_scroll_animating());

        // Scroll to a target position
        strip.scroll_to(500.0);
        assert_eq!(strip.target_scroll_offset(), 500.0);
        assert!(strip.is_scroll_animating());

        // Update for one frame (16ms)
        let delta = Duration::from_millis(16);
        let changed = strip.update(delta);

        assert!(changed);
        // Offset should have moved toward target
        assert!(strip.scroll_offset() > 0.0);
        assert!(strip.scroll_offset() < 500.0);
    }

    #[test]
    fn test_smooth_scroll_completes() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 50, (1200.0, 800.0));

        // Scroll to target
        strip.scroll_to(200.0);

        // Run many frames to complete animation
        let delta = Duration::from_millis(16);
        for _ in 0..100 {
            strip.update(delta);
        }

        // Animation should be complete
        assert!(!strip.is_scroll_animating());
        assert!((strip.scroll_offset() - 200.0).abs() < 1.0);
    }

    #[test]
    fn test_momentum_scrolling() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 50, (1200.0, 800.0));

        // Apply scroll with momentum (simulating trackpad swipe)
        strip.scroll(100.0);
        assert!(strip.is_scroll_animating());

        // Update for several frames
        let delta = Duration::from_millis(16);
        strip.update(delta);

        // Scroll offset should have moved
        let offset_after_first = strip.scroll_offset();
        assert!(offset_after_first > 0.0, "Scroll should have started moving");

        // Continue updating - momentum should carry through
        strip.update(delta);
        let offset_after_second = strip.scroll_offset();
        assert!(
            offset_after_second > offset_after_first,
            "Momentum should keep scrolling"
        );
    }

    #[test]
    fn test_scroll_clamped_to_bounds() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 50, (1200.0, 800.0));

        // Try to scroll past max
        let max_scroll = strip.calculate_max_scroll();
        strip.scroll_to(max_scroll + 1000.0);

        // Run animation
        let delta = Duration::from_millis(16);
        for _ in 0..100 {
            strip.update(delta);
        }

        // Should be clamped to max
        assert!(strip.scroll_offset() <= max_scroll + 0.1);

        // Try to scroll below 0
        strip.scroll_to(-1000.0);
        for _ in 0..100 {
            strip.update(delta);
        }

        // Should be clamped to 0
        assert!(strip.scroll_offset() >= -0.1);
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

        let strip = ThumbnailStrip::with_config(cache, 10, (1200.0, 800.0), custom_config);

        assert_eq!(strip.config.thumbnail_width, 200.0);
        assert_eq!(strip.config.thumbnail_height, 250.0);
        assert_eq!(strip.config.position, StripPosition::Right);
    }

    // =========================================================================
    // Lazy Loading Tests
    // =========================================================================

    #[test]
    fn test_get_needed_thumbnails_returns_visible_pages() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        // With default config (160px height + 8px spacing), about 4-5 pages fit in 800px viewport
        let strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        let needed = strip.get_needed_thumbnails();

        // Should return some visible thumbnails (not all 10)
        assert!(!needed.is_empty());
        assert!(needed.len() < 10);

        // First few pages should be needed (they're visible)
        // Note: exact count depends on thumbnail height (160) + spacing (8)
        // 800 / 168 ≈ 4.76, so roughly 5 pages visible
        assert!(needed.contains(&0));
    }

    #[test]
    fn test_get_needed_thumbnails_empty_when_not_visible() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        strip.set_visible(false);
        let needed = strip.get_needed_thumbnails();

        assert!(needed.is_empty());
    }

    #[test]
    fn test_get_needed_thumbnails_empty_when_no_pages() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let strip = ThumbnailStrip::new(cache, 0, (1200.0, 800.0));

        let needed = strip.get_needed_thumbnails();

        assert!(needed.is_empty());
    }

    #[test]
    fn test_mark_thumbnail_requested_prevents_duplicates() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        // Get initial needed thumbnails
        let needed1 = strip.get_needed_thumbnails();
        assert!(!needed1.is_empty());

        // Mark all as requested
        for &page in &needed1 {
            strip.mark_thumbnail_requested(page);
        }

        // Now get_needed_thumbnails should return empty (all pending)
        let needed2 = strip.get_needed_thumbnails();
        assert!(needed2.is_empty());
    }

    #[test]
    fn test_mark_thumbnails_requested_batch() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        // Get needed thumbnails
        let needed = strip.get_needed_thumbnails();
        let needed_clone: Vec<u16> = needed.clone();

        // Mark all using batch method
        strip.mark_thumbnails_requested(needed_clone);

        // Verify all are pending
        for page in needed {
            assert!(strip.is_thumbnail_pending(page));
        }
    }

    #[test]
    fn test_mark_thumbnail_loaded_clears_pending() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        // Mark page 0 as requested
        strip.mark_thumbnail_requested(0);
        assert!(strip.is_thumbnail_pending(0));
        assert_eq!(strip.pending_count(), 1);

        // Mark as loaded
        strip.mark_thumbnail_loaded(0);
        assert!(!strip.is_thumbnail_pending(0));
        assert_eq!(strip.pending_count(), 0);
    }

    #[test]
    fn test_clear_pending_requests() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        // Mark several as requested
        strip.mark_thumbnail_requested(0);
        strip.mark_thumbnail_requested(1);
        strip.mark_thumbnail_requested(2);
        assert_eq!(strip.pending_count(), 3);

        // Clear all
        strip.clear_pending_requests();
        assert_eq!(strip.pending_count(), 0);
    }

    #[test]
    fn test_needed_thumbnails_sorted_by_current_page() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 20, (1200.0, 800.0));

        // Set current page to somewhere in the middle of visible range
        strip.set_current_page(2);

        let needed = strip.get_needed_thumbnails();

        // Should have multiple pages
        assert!(needed.len() >= 2);

        // Pages should be sorted by distance from current page (2)
        // So page 2 should come first (distance 0), then 1 or 3 (distance 1), etc.
        if !needed.is_empty() {
            let current = strip.current_page();
            let mut prev_distance = 0i32;
            for &page in &needed {
                let distance = (page as i32 - current as i32).abs();
                assert!(
                    distance >= prev_distance,
                    "Pages should be sorted by distance from current. Got page {} (distance {}) after page with distance {}",
                    page, distance, prev_distance
                );
                prev_distance = distance;
            }
        }
    }

    #[test]
    fn test_get_prefetch_thumbnails() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 50, (1200.0, 800.0));

        // Scroll to middle-ish
        strip.scroll_immediate(500.0);

        let prefetch = strip.get_prefetch_thumbnails(3);

        // Prefetch should not include currently visible pages
        let needed = strip.get_needed_thumbnails();
        for page in &prefetch {
            assert!(
                !needed.contains(page),
                "Prefetch should not include visible pages"
            );
        }
    }

    #[test]
    fn test_thumbnail_cache_key() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        let key0 = strip.thumbnail_cache_key(0);
        let key1 = strip.thumbnail_cache_key(1);

        // Keys should be different for different pages
        assert_ne!(key0, key1);

        // Key should match the tile ID's cache key
        let tile_id = strip.create_thumbnail_tile_id(0);
        assert_eq!(key0, tile_id.cache_key());
    }

    #[test]
    fn test_thumbnail_dimensions() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        let (width, height) = strip.thumbnail_dimensions();

        assert_eq!(width, 120.0); // Default width
        assert_eq!(height, 160.0); // Default height
    }

    #[test]
    fn test_thumbnail_zoom_level() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        assert_eq!(strip.thumbnail_zoom_level(), 25);
    }

    #[test]
    fn test_mark_thumbnail_requested_ignores_invalid_index() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        // Try to mark invalid page
        strip.mark_thumbnail_requested(100);

        // Should not be tracked
        assert!(!strip.is_thumbnail_pending(100));
        assert_eq!(strip.pending_count(), 0);
    }

    #[test]
    fn test_pending_requests_cleared_on_document_change_scenario() {
        let config = CacheConfig::default();
        let cache = Arc::new(GpuTextureCache::new(config.gpu_cache_size));
        let mut strip = ThumbnailStrip::new(cache, 10, (1200.0, 800.0));

        // Mark some pages as pending
        strip.mark_thumbnail_requested(0);
        strip.mark_thumbnail_requested(1);
        assert_eq!(strip.pending_count(), 2);

        // Simulate document change by clearing pending
        strip.clear_pending_requests();

        // Should be able to request again
        let needed = strip.get_needed_thumbnails();
        assert!(needed.contains(&0) || !strip.is_thumbnail_pending(0));
    }
}
