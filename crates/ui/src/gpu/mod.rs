//! GPU abstraction layer
//!
//! Platform-agnostic GPU interface with backend-specific implementations.

#[cfg(target_os = "macos")]
pub mod metal;

use std::error::Error;
use std::fmt;

/// GPU backend error
#[derive(Debug)]
pub enum GpuError {
    /// Backend initialization failed
    InitializationFailed(String),
    /// Device creation failed
    DeviceCreationFailed(String),
    /// Command queue creation failed
    CommandQueueCreationFailed(String),
    /// Texture creation failed
    TextureCreationFailed(String),
    /// Buffer creation failed
    BufferCreationFailed(String),
    /// Shader compilation failed
    ShaderCompilationFailed(String),
    /// Pipeline creation failed
    PipelineCreationFailed(String),
    /// Other error
    Other(String),
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuError::InitializationFailed(msg) => write!(f, "GPU initialization failed: {}", msg),
            GpuError::DeviceCreationFailed(msg) => write!(f, "Device creation failed: {}", msg),
            GpuError::CommandQueueCreationFailed(msg) => {
                write!(f, "Command queue creation failed: {}", msg)
            }
            GpuError::TextureCreationFailed(msg) => write!(f, "Texture creation failed: {}", msg),
            GpuError::BufferCreationFailed(msg) => write!(f, "Buffer creation failed: {}", msg),
            GpuError::ShaderCompilationFailed(msg) => {
                write!(f, "Shader compilation failed: {}", msg)
            }
            GpuError::PipelineCreationFailed(msg) => {
                write!(f, "Pipeline creation failed: {}", msg)
            }
            GpuError::Other(msg) => write!(f, "GPU error: {}", msg),
        }
    }
}

impl Error for GpuError {}

/// Texture format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    /// 8-bit RGBA with sRGB color space
    Rgba8Srgb,
    /// 8-bit RGBA with linear color space
    Rgba8Unorm,
    /// 8-bit BGRA with sRGB color space (commonly used for Metal)
    Bgra8Srgb,
    /// 8-bit BGRA with linear color space
    Bgra8Unorm,
}

/// Texture descriptor
#[derive(Debug, Clone)]
pub struct TextureDescriptor {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Pixel format
    pub format: TextureFormat,
    /// Whether the texture is used as a render target
    pub render_target: bool,
}

/// Buffer usage flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferUsage {
    /// Vertex buffer
    Vertex,
    /// Index buffer
    Index,
    /// Uniform/constant buffer
    Uniform,
}

/// GPU context trait
///
/// Platform-agnostic interface for GPU operations.
pub trait GpuContext {
    /// Get the underlying device handle (platform-specific)
    fn device_handle(&self) -> *const ();

    /// Create a new texture
    fn create_texture(&self, descriptor: &TextureDescriptor) -> Result<Box<dyn Texture>, GpuError>;

    /// Create a new buffer
    fn create_buffer(&self, size: usize, usage: BufferUsage) -> Result<Box<dyn Buffer>, GpuError>;

    /// Begin a new frame
    fn begin_frame(&mut self) -> Result<(), GpuError>;

    /// End the current frame and present
    fn end_frame(&mut self) -> Result<(), GpuError>;
}

/// GPU texture trait
pub trait Texture {
    /// Get texture width
    fn width(&self) -> u32;

    /// Get texture height
    fn height(&self) -> u32;

    /// Get texture format
    fn format(&self) -> TextureFormat;

    /// Upload data to the texture
    fn upload(&mut self, data: &[u8]) -> Result<(), GpuError>;
}

/// GPU buffer trait
pub trait Buffer {
    /// Get buffer size in bytes
    fn size(&self) -> usize;

    /// Get buffer usage
    fn usage(&self) -> BufferUsage;

    /// Write data to the buffer
    fn write(&mut self, data: &[u8]) -> Result<(), GpuError>;
}

/// Create a GPU context for the current platform
#[cfg(target_os = "macos")]
pub fn create_context() -> Result<Box<dyn GpuContext>, GpuError> {
    metal::MetalContext::new().map(|ctx| Box::new(ctx) as Box<dyn GpuContext>)
}

#[cfg(not(target_os = "macos"))]
pub fn create_context() -> Result<Box<dyn GpuContext>, GpuError> {
    Err(GpuError::InitializationFailed(
        "No GPU backend available for this platform".to_string(),
    ))
}
