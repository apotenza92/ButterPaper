//! Metal GPU backend for macOS

use super::{Buffer, BufferUsage, GpuContext, GpuError, Texture, TextureDescriptor, TextureFormat};
use metal::{CommandQueue, Device, MTLPixelFormat, MTLResourceOptions, MTLStorageMode};

/// Metal GPU context
pub struct MetalContext {
    device: Device,
    command_queue: CommandQueue,
}

impl MetalContext {
    /// Create a new Metal context
    pub fn new() -> Result<Self, GpuError> {
        let device = Device::system_default().ok_or_else(|| {
            GpuError::DeviceCreationFailed("No Metal device available".to_string())
        })?;

        let command_queue = device.new_command_queue();

        Ok(Self {
            device,
            command_queue,
        })
    }

    /// Get the Metal device
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get the Metal command queue
    pub fn command_queue(&self) -> &CommandQueue {
        &self.command_queue
    }
}

impl GpuContext for MetalContext {
    fn device_handle(&self) -> *const () {
        &self.device as *const _ as *const ()
    }

    fn create_texture(&self, descriptor: &TextureDescriptor) -> Result<Box<dyn Texture>, GpuError> {
        let metal_format = match descriptor.format {
            TextureFormat::Rgba8Srgb => MTLPixelFormat::RGBA8Unorm_sRGB,
            TextureFormat::Rgba8Unorm => MTLPixelFormat::RGBA8Unorm,
            TextureFormat::Bgra8Srgb => MTLPixelFormat::BGRA8Unorm_sRGB,
            TextureFormat::Bgra8Unorm => MTLPixelFormat::BGRA8Unorm,
        };

        let texture_descriptor = metal::TextureDescriptor::new();
        texture_descriptor.set_width(descriptor.width as u64);
        texture_descriptor.set_height(descriptor.height as u64);
        texture_descriptor.set_pixel_format(metal_format);
        texture_descriptor.set_storage_mode(MTLStorageMode::Managed);

        if descriptor.render_target {
            texture_descriptor.set_usage(
                metal::MTLTextureUsage::RenderTarget | metal::MTLTextureUsage::ShaderRead,
            );
        } else {
            texture_descriptor.set_usage(metal::MTLTextureUsage::ShaderRead);
        }

        let texture = self.device.new_texture(&texture_descriptor);

        Ok(Box::new(MetalTexture {
            texture,
            width: descriptor.width,
            height: descriptor.height,
            format: descriptor.format,
        }))
    }

    fn create_buffer(&self, size: usize, usage: BufferUsage) -> Result<Box<dyn Buffer>, GpuError> {
        let options = MTLResourceOptions::StorageModeManaged;
        let buffer = self.device.new_buffer(size as u64, options);

        Ok(Box::new(MetalBuffer {
            buffer,
            size,
            usage,
        }))
    }

    fn begin_frame(&mut self) -> Result<(), GpuError> {
        // Frame preparation will be implemented when we add the rendering pipeline
        Ok(())
    }

    fn end_frame(&mut self) -> Result<(), GpuError> {
        // Frame presentation will be implemented when we add the rendering pipeline
        Ok(())
    }
}

/// Metal texture
struct MetalTexture {
    texture: metal::Texture,
    width: u32,
    height: u32,
    format: TextureFormat,
}

impl Texture for MetalTexture {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn format(&self) -> TextureFormat {
        self.format
    }

    fn upload(&mut self, data: &[u8]) -> Result<(), GpuError> {
        let bytes_per_pixel = match self.format {
            TextureFormat::Rgba8Srgb | TextureFormat::Rgba8Unorm => 4,
            TextureFormat::Bgra8Srgb | TextureFormat::Bgra8Unorm => 4,
        };

        let bytes_per_row = self.width as usize * bytes_per_pixel;
        let expected_size = bytes_per_row * self.height as usize;

        if data.len() != expected_size {
            return Err(GpuError::TextureCreationFailed(format!(
                "Data size mismatch: expected {} bytes, got {}",
                expected_size,
                data.len()
            )));
        }

        let region = metal::MTLRegion {
            origin: metal::MTLOrigin { x: 0, y: 0, z: 0 },
            size: metal::MTLSize {
                width: self.width as u64,
                height: self.height as u64,
                depth: 1,
            },
        };

        self.texture
            .replace_region(region, 0, data.as_ptr() as *const _, bytes_per_row as u64);

        Ok(())
    }
}

/// Metal buffer
struct MetalBuffer {
    buffer: metal::Buffer,
    size: usize,
    usage: BufferUsage,
}

impl Buffer for MetalBuffer {
    fn size(&self) -> usize {
        self.size
    }

    fn usage(&self) -> BufferUsage {
        self.usage
    }

    fn write(&mut self, data: &[u8]) -> Result<(), GpuError> {
        if data.len() > self.size {
            return Err(GpuError::BufferCreationFailed(format!(
                "Data size {} exceeds buffer size {}",
                data.len(),
                self.size
            )));
        }

        let contents = self.buffer.contents() as *mut u8;
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), contents, data.len());
        }

        let range = metal::NSRange {
            location: 0,
            length: data.len() as u64,
        };
        self.buffer.did_modify_range(range);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metal_context_creation() {
        // Test that Metal context can be created on macOS
        let context = MetalContext::new();
        assert!(
            context.is_ok(),
            "Metal context creation should succeed on macOS"
        );
    }

    #[test]
    fn test_metal_device_availability() {
        // Test that Metal device is available
        let context = MetalContext::new().expect("Failed to create Metal context");
        let device = context.device();

        // Verify device has a name (all Metal devices should have one)
        let name = device.name();
        assert!(!name.is_empty(), "Metal device should have a name");
    }

    #[test]
    fn test_metal_command_queue() {
        // Test command queue creation and command buffer generation
        let context = MetalContext::new().expect("Failed to create Metal context");
        let queue = context.command_queue();

        // Create a command buffer from the queue and verify it can be committed
        let command_buffer = queue.new_command_buffer();

        // Commit the empty command buffer - this verifies the queue and buffer work
        command_buffer.commit();
        command_buffer.wait_until_completed();

        // If we got here without panicking, the command queue works
    }

    #[test]
    fn test_texture_creation_bgra8_srgb() {
        let context = MetalContext::new().expect("Failed to create Metal context");
        let descriptor = TextureDescriptor {
            width: 256,
            height: 256,
            format: TextureFormat::Bgra8Srgb,
            render_target: false,
        };

        let texture = context.create_texture(&descriptor);
        assert!(texture.is_ok(), "Texture creation should succeed");

        let tex = texture.unwrap();
        assert_eq!(tex.width(), 256);
        assert_eq!(tex.height(), 256);
        assert_eq!(tex.format(), TextureFormat::Bgra8Srgb);
    }

    #[test]
    fn test_texture_creation_rgba8_unorm() {
        let context = MetalContext::new().expect("Failed to create Metal context");
        let descriptor = TextureDescriptor {
            width: 512,
            height: 512,
            format: TextureFormat::Rgba8Unorm,
            render_target: true,
        };

        let texture = context.create_texture(&descriptor);
        assert!(texture.is_ok(), "Render target texture should be created");

        let tex = texture.unwrap();
        assert_eq!(tex.width(), 512);
        assert_eq!(tex.height(), 512);
        assert_eq!(tex.format(), TextureFormat::Rgba8Unorm);
    }

    #[test]
    fn test_texture_creation_all_formats() {
        let context = MetalContext::new().expect("Failed to create Metal context");

        let formats = [
            TextureFormat::Rgba8Srgb,
            TextureFormat::Rgba8Unorm,
            TextureFormat::Bgra8Srgb,
            TextureFormat::Bgra8Unorm,
        ];

        for format in formats {
            let descriptor = TextureDescriptor {
                width: 128,
                height: 128,
                format,
                render_target: false,
            };

            let texture = context.create_texture(&descriptor);
            assert!(
                texture.is_ok(),
                "Texture creation should succeed for format {:?}",
                format
            );
        }
    }

    #[test]
    fn test_texture_upload() {
        let context = MetalContext::new().expect("Failed to create Metal context");
        let descriptor = TextureDescriptor {
            width: 64,
            height: 64,
            format: TextureFormat::Rgba8Unorm,
            render_target: false,
        };

        let mut texture = context.create_texture(&descriptor).unwrap();

        // Create test pixel data (64x64 RGBA = 16384 bytes)
        let data: Vec<u8> = (0..64 * 64 * 4).map(|i| (i % 256) as u8).collect();

        let result = texture.upload(&data);
        assert!(result.is_ok(), "Texture upload should succeed");
    }

    #[test]
    fn test_texture_upload_size_mismatch() {
        let context = MetalContext::new().expect("Failed to create Metal context");
        let descriptor = TextureDescriptor {
            width: 64,
            height: 64,
            format: TextureFormat::Rgba8Unorm,
            render_target: false,
        };

        let mut texture = context.create_texture(&descriptor).unwrap();

        // Wrong size data (too small)
        let data: Vec<u8> = vec![0; 100];

        let result = texture.upload(&data);
        assert!(result.is_err(), "Upload should fail with wrong data size");
    }

    #[test]
    fn test_buffer_creation_vertex() {
        let context = MetalContext::new().expect("Failed to create Metal context");

        let buffer = context.create_buffer(1024, BufferUsage::Vertex);
        assert!(buffer.is_ok(), "Vertex buffer creation should succeed");

        let buf = buffer.unwrap();
        assert_eq!(buf.size(), 1024);
        assert_eq!(buf.usage(), BufferUsage::Vertex);
    }

    #[test]
    fn test_buffer_creation_index() {
        let context = MetalContext::new().expect("Failed to create Metal context");

        let buffer = context.create_buffer(512, BufferUsage::Index);
        assert!(buffer.is_ok(), "Index buffer creation should succeed");

        let buf = buffer.unwrap();
        assert_eq!(buf.size(), 512);
        assert_eq!(buf.usage(), BufferUsage::Index);
    }

    #[test]
    fn test_buffer_creation_uniform() {
        let context = MetalContext::new().expect("Failed to create Metal context");

        let buffer = context.create_buffer(256, BufferUsage::Uniform);
        assert!(buffer.is_ok(), "Uniform buffer creation should succeed");

        let buf = buffer.unwrap();
        assert_eq!(buf.size(), 256);
        assert_eq!(buf.usage(), BufferUsage::Uniform);
    }

    #[test]
    fn test_buffer_write() {
        let context = MetalContext::new().expect("Failed to create Metal context");

        let mut buffer = context.create_buffer(1024, BufferUsage::Vertex).unwrap();

        // Write some vertex data
        let data: Vec<u8> = (0..512).map(|i| (i % 256) as u8).collect();

        let result = buffer.write(&data);
        assert!(result.is_ok(), "Buffer write should succeed");
    }

    #[test]
    fn test_buffer_write_overflow() {
        let context = MetalContext::new().expect("Failed to create Metal context");

        let mut buffer = context.create_buffer(100, BufferUsage::Vertex).unwrap();

        // Try to write more data than buffer size
        let data: Vec<u8> = vec![0; 200];

        let result = buffer.write(&data);
        assert!(
            result.is_err(),
            "Buffer write should fail when data exceeds size"
        );
    }

    #[test]
    fn test_frame_lifecycle() {
        let mut context = MetalContext::new().expect("Failed to create Metal context");

        // Test begin/end frame cycle
        let begin_result = context.begin_frame();
        assert!(begin_result.is_ok(), "begin_frame should succeed");

        let end_result = context.end_frame();
        assert!(end_result.is_ok(), "end_frame should succeed");
    }

    #[test]
    fn test_device_handle() {
        let context = MetalContext::new().expect("Failed to create Metal context");

        let handle = context.device_handle();
        assert!(!handle.is_null(), "Device handle should not be null");
    }

    #[test]
    fn test_multiple_textures() {
        let context = MetalContext::new().expect("Failed to create Metal context");

        // Create multiple textures to ensure GPU resources are managed properly
        let mut textures = Vec::new();
        for i in 0..10 {
            let descriptor = TextureDescriptor {
                width: 128 + i * 16,
                height: 128 + i * 16,
                format: TextureFormat::Bgra8Srgb,
                render_target: i % 2 == 0,
            };

            let texture = context.create_texture(&descriptor);
            assert!(texture.is_ok(), "Texture {} creation should succeed", i);
            textures.push(texture.unwrap());
        }

        // Verify all textures have correct dimensions
        for (i, tex) in textures.iter().enumerate() {
            assert_eq!(tex.width(), 128 + i as u32 * 16);
            assert_eq!(tex.height(), 128 + i as u32 * 16);
        }
    }

    #[test]
    fn test_multiple_buffers() {
        let context = MetalContext::new().expect("Failed to create Metal context");

        // Create multiple buffers
        let usages = [
            BufferUsage::Vertex,
            BufferUsage::Index,
            BufferUsage::Uniform,
        ];
        let mut buffers = Vec::new();

        for (i, usage) in usages.iter().cycle().take(10).enumerate() {
            let buffer = context.create_buffer(256 * (i + 1), *usage);
            assert!(buffer.is_ok(), "Buffer {} creation should succeed", i);
            buffers.push(buffer.unwrap());
        }

        // Verify all buffers have correct sizes
        for (i, buf) in buffers.iter().enumerate() {
            assert_eq!(buf.size(), 256 * (i + 1));
        }
    }

    #[test]
    fn test_render_target_texture() {
        let context = MetalContext::new().expect("Failed to create Metal context");
        let descriptor = TextureDescriptor {
            width: 1024,
            height: 768,
            format: TextureFormat::Bgra8Srgb,
            render_target: true,
        };

        let texture = context.create_texture(&descriptor);
        assert!(
            texture.is_ok(),
            "Large render target texture should be created"
        );

        let tex = texture.unwrap();
        assert_eq!(tex.width(), 1024);
        assert_eq!(tex.height(), 768);
    }

    #[test]
    fn test_command_buffer_sequential() {
        let context = MetalContext::new().expect("Failed to create Metal context");
        let queue = context.command_queue();

        // Create and commit multiple sequential command buffers
        for _ in 0..5 {
            let command_buffer = queue.new_command_buffer();
            command_buffer.commit();
            command_buffer.wait_until_completed();
        }
    }

    #[test]
    fn test_small_texture() {
        let context = MetalContext::new().expect("Failed to create Metal context");
        let descriptor = TextureDescriptor {
            width: 1,
            height: 1,
            format: TextureFormat::Rgba8Unorm,
            render_target: false,
        };

        let mut texture = context.create_texture(&descriptor).unwrap();

        // Upload single pixel
        let data: Vec<u8> = vec![255, 0, 0, 255]; // Red pixel
        let result = texture.upload(&data);
        assert!(result.is_ok(), "Single pixel texture upload should succeed");
    }

    #[test]
    fn test_large_texture() {
        let context = MetalContext::new().expect("Failed to create Metal context");
        let descriptor = TextureDescriptor {
            width: 4096,
            height: 4096,
            format: TextureFormat::Bgra8Unorm,
            render_target: false,
        };

        let texture = context.create_texture(&descriptor);
        assert!(texture.is_ok(), "Large texture creation should succeed");

        let tex = texture.unwrap();
        assert_eq!(tex.width(), 4096);
        assert_eq!(tex.height(), 4096);
    }
}
