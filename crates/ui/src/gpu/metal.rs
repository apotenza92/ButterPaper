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
            texture_descriptor
                .set_usage(metal::MTLTextureUsage::RenderTarget | metal::MTLTextureUsage::ShaderRead);
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

        self.texture.replace_region(
            region,
            0,
            data.as_ptr() as *const _,
            bytes_per_row as u64,
        );

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
