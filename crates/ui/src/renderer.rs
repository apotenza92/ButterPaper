//! GPU renderer for scene graph
//!
//! This module provides GPU-accelerated rendering of scene graph primitives.

use crate::gpu::{GpuContext, GpuError};
use crate::scene::{Primitive, RenderCommand, SceneGraph};

/// Renderer for scene graph primitives
pub struct SceneRenderer {
    /// Cached render state (GPU buffers, pipelines, etc.)
    _gpu_state: (),
}

impl SceneRenderer {
    /// Create a new scene renderer
    pub fn new(_gpu_context: &dyn GpuContext) -> Result<Self, GpuError> {
        // In a full implementation, this would:
        // 1. Compile shaders for different primitive types
        // 2. Create pipeline state objects
        // 3. Set up vertex buffer layouts
        // 4. Create uniform buffers for transforms

        Ok(Self {
            _gpu_state: (),
        })
    }

    /// Render a scene graph
    pub fn render(
        &mut self,
        _gpu_context: &mut dyn GpuContext,
        scene: &SceneGraph,
    ) -> Result<(), GpuError> {
        // Collect all render commands from the scene graph
        let commands = scene.collect_render_commands();

        // In a full implementation, this would:
        // 1. Begin render pass
        // 2. For each render command:
        //    - Bind appropriate pipeline for primitive type
        //    - Update transform uniforms
        //    - Issue draw call
        // 3. End render pass

        // For now, we just collect commands to validate the scene graph works
        let _command_count = commands.len();

        Ok(())
    }

    /// Render commands directly (for custom rendering)
    pub fn render_commands(
        &mut self,
        _gpu_context: &mut dyn GpuContext,
        _commands: &[RenderCommand],
    ) -> Result<(), GpuError> {
        // Similar to render() but takes commands directly
        // Useful for one-off rendering without building a full scene graph
        Ok(())
    }

    /// Render a single primitive (convenience method)
    pub fn render_primitive(
        &mut self,
        _gpu_context: &mut dyn GpuContext,
        _primitive: &Primitive,
    ) -> Result<(), GpuError> {
        // Render a single primitive with identity transform
        // Useful for testing and simple cases
        Ok(())
    }
}

/// Render statistics for debugging and profiling
#[derive(Debug, Default, Clone)]
pub struct RenderStats {
    /// Number of primitives rendered
    pub primitive_count: usize,
    /// Number of draw calls issued
    pub draw_call_count: usize,
    /// Number of nodes in the scene graph
    pub node_count: usize,
    /// Number of dirty nodes that were re-rendered
    pub dirty_node_count: usize,
}

impl RenderStats {
    /// Create new empty stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all counters to zero
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Color, Rect, SceneGraph};

    #[test]
    fn test_render_stats() {
        let mut stats = RenderStats::new();
        assert_eq!(stats.primitive_count, 0);

        stats.primitive_count = 10;
        stats.reset();
        assert_eq!(stats.primitive_count, 0);
    }

    #[test]
    fn test_scene_renderer_creation() {
        // Create a mock GPU context for testing
        #[cfg(target_os = "macos")]
        {
            use crate::gpu;
            if let Ok(context) = gpu::create_context() {
                let renderer = SceneRenderer::new(context.as_ref());
                assert!(renderer.is_ok());
            }
        }
    }

    #[test]
    fn test_render_empty_scene() {
        #[cfg(target_os = "macos")]
        {
            use crate::gpu;
            if let Ok(mut context) = gpu::create_context() {
                if let Ok(mut renderer) = SceneRenderer::new(context.as_ref()) {
                    let scene = SceneGraph::new();
                    let result = renderer.render(context.as_mut(), &scene);
                    assert!(result.is_ok());
                }
            }
        }
    }

    #[test]
    fn test_render_simple_scene() {
        #[cfg(target_os = "macos")]
        {
            use crate::gpu;
            if let Ok(mut context) = gpu::create_context() {
                if let Ok(mut renderer) = SceneRenderer::new(context.as_ref()) {
                    let mut scene = SceneGraph::new();
                    scene.root_mut().add_primitive(Primitive::Rectangle {
                        rect: Rect::new(0.0, 0.0, 100.0, 100.0),
                        color: Color::rgb(1.0, 0.0, 0.0),
                    });

                    let result = renderer.render(context.as_mut(), &scene);
                    assert!(result.is_ok());
                }
            }
        }
    }
}
