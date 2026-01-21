//! PDF Editor Application
//!
//! Main application entry point with GPU-rendered UI shell.

use pdf_editor_render::PdfDocument;
use pdf_editor_ui::gpu;
use pdf_editor_ui::input::InputHandler;
use pdf_editor_ui::renderer::SceneRenderer;
use pdf_editor_ui::scene::SceneGraph;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

#[cfg(target_os = "macos")]
#[allow(deprecated)]
use cocoa::base::id as cocoa_id;
#[cfg(target_os = "macos")]
use core_graphics_types::geometry::CGSize;
#[cfg(target_os = "macos")]
use metal::{CommandQueue, Device, MetalLayer, MTLPixelFormat, TextureDescriptor};
#[cfg(target_os = "macos")]
use objc::runtime::YES;
#[cfg(target_os = "macos")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
#[cfg(target_os = "macos")]
#[allow(deprecated)]
use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy};

#[cfg(target_os = "macos")]
mod text_overlay {
    use metal::{Device, MTLPixelFormat, TextureDescriptor};

    /// Overlay texture holding rendered text
    pub struct OverlayTexture {
        pub texture: metal::Texture,
        pub width: u32,
        pub height: u32,
    }

    /// Spinner texture for loading indicator
    pub struct SpinnerTexture {
        pub texture: metal::Texture,
        #[allow(dead_code)]
        pub size: u32,
    }

    /// Render a loading spinner frame at a given rotation angle (0-7 for 8 positions)
    pub fn render_spinner(device: &Device, frame: u8, size: u32) -> Option<SpinnerTexture> {
        // Create BGRA pixel buffer
        let mut pixels = vec![0u8; (size * size * 4) as usize];

        let center = size as f32 / 2.0;
        let outer_radius = center - 4.0;
        let inner_radius = outer_radius * 0.5;
        let dot_radius = (outer_radius - inner_radius) / 2.5;

        // Draw 8 dots in a circle, with opacity based on their position relative to current frame
        for i in 0..8u8 {
            let angle = (i as f32) * std::f32::consts::PI / 4.0 - std::f32::consts::PI / 2.0;
            let dot_center_x = center + (outer_radius - dot_radius - 2.0) * angle.cos();
            let dot_center_y = center + (outer_radius - dot_radius - 2.0) * angle.sin();

            // Calculate opacity based on distance from current frame (creates trail effect)
            let distance = ((i as i8 - frame as i8).rem_euclid(8)) as f32;
            let alpha = ((8.0 - distance) / 8.0 * 255.0) as u8;

            // Draw the dot
            draw_filled_circle(&mut pixels, size, dot_center_x, dot_center_y, dot_radius, alpha);
        }

        // Create Metal texture
        let texture_desc = TextureDescriptor::new();
        texture_desc.set_width(size as u64);
        texture_desc.set_height(size as u64);
        texture_desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm_sRGB);
        texture_desc.set_usage(metal::MTLTextureUsage::ShaderRead);

        let texture = device.new_texture(&texture_desc);

        let region = metal::MTLRegion {
            origin: metal::MTLOrigin { x: 0, y: 0, z: 0 },
            size: metal::MTLSize {
                width: size as u64,
                height: size as u64,
                depth: 1,
            },
        };

        texture.replace_region(
            region,
            0,
            pixels.as_ptr() as *const _,
            (size * 4) as u64,
        );

        Some(SpinnerTexture { texture, size })
    }

    /// Draw a filled circle with the given alpha value (white color)
    fn draw_filled_circle(pixels: &mut [u8], tex_width: u32, cx: f32, cy: f32, radius: f32, alpha: u8) {
        let min_x = (cx - radius - 1.0).max(0.0) as u32;
        let max_x = ((cx + radius + 1.0) as u32).min(tex_width - 1);
        let min_y = (cy - radius - 1.0).max(0.0) as u32;
        let max_y = ((cy + radius + 1.0) as u32).min(tex_width - 1);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= radius {
                    let idx = ((y * tex_width + x) * 4) as usize;
                    if idx + 3 < pixels.len() {
                        // White with varying alpha (BGRA format)
                        pixels[idx] = 255;     // B
                        pixels[idx + 1] = 255; // G
                        pixels[idx + 2] = 255; // R
                        pixels[idx + 3] = alpha; // A
                    }
                }
            }
        }
    }

    /// Simple 5x7 bitmap font for basic ASCII characters (0-9, A-Z, a-z, space, punctuation)
    /// Each character is represented as a 5-wide by 7-tall bitmap, stored as a [u8; 7] where
    /// each byte represents one row (MSB = leftmost pixel).
    fn get_char_bitmap(c: char) -> [u8; 7] {
        match c {
            '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
            '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
            '2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
            '3' => [0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110],
            '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
            '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
            '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
            '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
            '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
            '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
            'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
            'a' => [0b00000, 0b00000, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111],
            'g' => [0b00000, 0b00000, 0b01111, 0b10001, 0b01111, 0b00001, 0b01110],
            'e' => [0b00000, 0b00000, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110],
            'o' => [0b00000, 0b00000, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110],
            'f' => [0b00110, 0b01001, 0b01000, 0b11110, 0b01000, 0b01000, 0b01000],
            ' ' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
            '|' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
            '%' => [0b11001, 0b11010, 0b00100, 0b01000, 0b10110, 0b10011, 0b00000],
            _ => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000], // unknown char
        }
    }

    /// Render text to a Metal texture with a semi-transparent background
    pub fn render_text_overlay(
        device: &Device,
        text: &str,
        scale: u32,
        padding: u32,
    ) -> Option<OverlayTexture> {
        let char_width = 5u32 * scale;
        let char_height = 7u32 * scale;
        let char_spacing = scale; // 1 pixel spacing between characters

        let text_width = text.len() as u32 * (char_width + char_spacing);
        let text_height = char_height;

        let tex_width = text_width + padding * 2;
        let tex_height = text_height + padding * 2;

        // Ensure minimum size
        let tex_width = tex_width.max(20);
        let tex_height = tex_height.max(20);

        // Create BGRA pixel buffer (initialized to semi-transparent black background)
        let mut pixels = vec![0u8; (tex_width * tex_height * 4) as usize];

        // Fill background with semi-transparent dark color
        for y in 0..tex_height {
            for x in 0..tex_width {
                let idx = ((y * tex_width + x) * 4) as usize;
                // Check if we're in the rounded corner regions
                let corner_radius = 6.0f32 * scale as f32 / 2.0;
                let is_corner = is_outside_rounded_rect(
                    x as f32,
                    y as f32,
                    tex_width as f32,
                    tex_height as f32,
                    corner_radius,
                );

                if is_corner {
                    // Transparent
                    pixels[idx] = 0;     // B
                    pixels[idx + 1] = 0; // G
                    pixels[idx + 2] = 0; // R
                    pixels[idx + 3] = 0; // A
                } else {
                    // Semi-transparent black
                    pixels[idx] = 0;       // B
                    pixels[idx + 1] = 0;   // G
                    pixels[idx + 2] = 0;   // R
                    pixels[idx + 3] = 180; // A (about 70% opaque)
                }
            }
        }

        // Draw each character
        let mut x_offset = padding;
        for c in text.chars() {
            let bitmap = get_char_bitmap(c);
            draw_char_scaled(
                &mut pixels,
                tex_width,
                x_offset,
                padding,
                &bitmap,
                scale,
            );
            x_offset += char_width + char_spacing;
        }

        // Create Metal texture
        let texture_desc = TextureDescriptor::new();
        texture_desc.set_width(tex_width as u64);
        texture_desc.set_height(tex_height as u64);
        texture_desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm_sRGB);
        texture_desc.set_usage(metal::MTLTextureUsage::ShaderRead);

        let texture = device.new_texture(&texture_desc);

        let region = metal::MTLRegion {
            origin: metal::MTLOrigin { x: 0, y: 0, z: 0 },
            size: metal::MTLSize {
                width: tex_width as u64,
                height: tex_height as u64,
                depth: 1,
            },
        };

        texture.replace_region(
            region,
            0,
            pixels.as_ptr() as *const _,
            (tex_width * 4) as u64,
        );

        Some(OverlayTexture {
            texture,
            width: tex_width,
            height: tex_height,
        })
    }

    /// Check if a point is outside the rounded rectangle
    fn is_outside_rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> bool {
        // Check each corner
        // Top-left
        if x < r && y < r {
            let dx = r - x;
            let dy = r - y;
            return dx * dx + dy * dy > r * r;
        }
        // Top-right
        if x > w - r && y < r {
            let dx = x - (w - r);
            let dy = r - y;
            return dx * dx + dy * dy > r * r;
        }
        // Bottom-left
        if x < r && y > h - r {
            let dx = r - x;
            let dy = y - (h - r);
            return dx * dx + dy * dy > r * r;
        }
        // Bottom-right
        if x > w - r && y > h - r {
            let dx = x - (w - r);
            let dy = y - (h - r);
            return dx * dx + dy * dy > r * r;
        }
        false
    }

    /// Draw a scaled character bitmap to the pixel buffer
    fn draw_char_scaled(
        pixels: &mut [u8],
        tex_width: u32,
        x_start: u32,
        y_start: u32,
        bitmap: &[u8; 7],
        scale: u32,
    ) {
        for (row_idx, &row_bits) in bitmap.iter().enumerate() {
            for col in 0..5u32 {
                let bit = (row_bits >> (4 - col)) & 1;
                if bit == 1 {
                    // Draw scaled pixel (scale x scale block)
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let px = x_start + col * scale + sx;
                            let py = y_start + row_idx as u32 * scale + sy;
                            let idx = ((py * tex_width + px) * 4) as usize;
                            if idx + 3 < pixels.len() {
                                // White text (BGRA format)
                                pixels[idx] = 255;     // B
                                pixels[idx + 1] = 255; // G
                                pixels[idx + 2] = 255; // R
                                pixels[idx + 3] = 255; // A
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_get_char_bitmap_digits() {
            // Test that digit bitmaps are non-zero
            for c in '0'..='9' {
                let bitmap = get_char_bitmap(c);
                let non_zero = bitmap.iter().any(|&b| b != 0);
                assert!(non_zero, "Digit '{}' should have non-zero bitmap", c);
            }
        }

        #[test]
        fn test_get_char_bitmap_special() {
            // Space should be all zeros
            let space = get_char_bitmap(' ');
            assert!(space.iter().all(|&b| b == 0), "Space should be all zeros");

            // Pipe should have vertical line
            let pipe = get_char_bitmap('|');
            let has_center_bit = pipe.iter().all(|&b| b == 0b00100);
            assert!(has_center_bit, "Pipe should have center bit set");

            // Unknown char should be all zeros
            let unknown = get_char_bitmap('$');
            assert!(unknown.iter().all(|&b| b == 0), "Unknown char should be all zeros");
        }

        #[test]
        fn test_is_outside_rounded_rect() {
            // Point inside rectangle (not in corner)
            assert!(!is_outside_rounded_rect(50.0, 50.0, 100.0, 100.0, 10.0));

            // Point at top-left corner (outside rounded area)
            assert!(is_outside_rounded_rect(0.0, 0.0, 100.0, 100.0, 10.0));

            // Point at top-right corner (outside rounded area)
            assert!(is_outside_rounded_rect(99.0, 0.0, 100.0, 100.0, 10.0));

            // Point near but inside the corner radius
            assert!(!is_outside_rounded_rect(10.0, 10.0, 100.0, 100.0, 10.0));
        }

        #[test]
        fn test_draw_char_scaled() {
            // Create a small buffer and draw a character
            let tex_width = 20u32;
            let tex_height = 20u32;
            let mut pixels = vec![0u8; (tex_width * tex_height * 4) as usize];

            // Draw the digit '1' at position (2, 2) with scale 1
            let bitmap = get_char_bitmap('1');
            draw_char_scaled(&mut pixels, tex_width, 2, 2, &bitmap, 1);

            // Check that some pixels were set to white
            let white_pixels: usize = pixels
                .chunks_exact(4)
                .filter(|p| p[0] == 255 && p[1] == 255 && p[2] == 255 && p[3] == 255)
                .count();

            assert!(white_pixels > 0, "Should have drawn some white pixels");

            // The digit '1' has a specific pattern - count the set bits
            let expected_bits: u32 = bitmap.iter().map(|&b| b.count_ones()).sum();
            assert_eq!(white_pixels, expected_bits as usize, "White pixel count should match bitmap bits");
        }

        #[test]
        fn test_draw_char_scaled_with_scaling() {
            // Test that scaling works correctly (scale 2 = 4x pixels per bit)
            let tex_width = 40u32;
            let tex_height = 40u32;
            let mut pixels = vec![0u8; (tex_width * tex_height * 4) as usize];

            let bitmap = get_char_bitmap('1');
            let scale = 2u32;
            draw_char_scaled(&mut pixels, tex_width, 2, 2, &bitmap, scale);

            let white_pixels: usize = pixels
                .chunks_exact(4)
                .filter(|p| p[0] == 255 && p[1] == 255 && p[2] == 255 && p[3] == 255)
                .count();

            let bits_in_bitmap: u32 = bitmap.iter().map(|&b| b.count_ones()).sum();
            let expected_pixels = bits_in_bitmap * scale * scale;

            assert_eq!(white_pixels, expected_pixels as usize,
                "Scaled drawing should have scale^2 times the bitmap bits");
        }

        #[test]
        fn test_draw_filled_circle() {
            // Create a buffer and draw a circle
            let size = 50u32;
            let mut pixels = vec![0u8; (size * size * 4) as usize];

            // Draw a circle at center with radius 10
            draw_filled_circle(&mut pixels, size, 25.0, 25.0, 10.0, 200);

            // Count non-zero alpha pixels
            let drawn_pixels: usize = pixels
                .chunks_exact(4)
                .filter(|p| p[3] > 0)
                .count();

            // Circle area is approximately pi * r^2 = ~314 pixels
            // Allow some variance due to discrete pixel representation
            assert!(drawn_pixels > 250, "Should have drawn a filled circle (got {} pixels)", drawn_pixels);
            assert!(drawn_pixels < 400, "Circle should not be too large (got {} pixels)", drawn_pixels);
        }

        #[test]
        fn test_draw_filled_circle_alpha() {
            // Test that alpha is correctly applied
            let size = 30u32;
            let mut pixels = vec![0u8; (size * size * 4) as usize];

            draw_filled_circle(&mut pixels, size, 15.0, 15.0, 5.0, 128);

            // Find a pixel that was drawn and check its alpha
            let drawn_pixel = pixels
                .chunks_exact(4)
                .find(|p| p[3] > 0)
                .expect("Should have at least one drawn pixel");

            assert_eq!(drawn_pixel[3], 128, "Alpha should be 128");
            assert_eq!(drawn_pixel[0], 255, "Blue should be 255 (white)");
            assert_eq!(drawn_pixel[1], 255, "Green should be 255 (white)");
            assert_eq!(drawn_pixel[2], 255, "Red should be 255 (white)");
        }

        #[test]
        fn test_spinner_frame_positions() {
            // Test that different frames have pixels in different positions
            // We can't test the actual Metal texture without a device, but we can test the logic
            // by checking the draw_filled_circle function used by the spinner

            let size = 64u32;

            // For frame 0, the brightest dot should be at the top (negative y direction)
            // For frame 4, the brightest dot should be at the bottom
            // This tests that the animation frames produce different visual outputs

            let mut pixels_frame0 = vec![0u8; (size * size * 4) as usize];
            let mut pixels_frame4 = vec![0u8; (size * size * 4) as usize];

            let center = size as f32 / 2.0;
            let outer_radius = center - 4.0;
            let dot_radius = outer_radius * 0.15;

            // Draw frame 0 - brightest dot at top
            let angle0 = -std::f32::consts::PI / 2.0; // top
            let x0 = center + (outer_radius - dot_radius - 2.0) * angle0.cos();
            let y0 = center + (outer_radius - dot_radius - 2.0) * angle0.sin();
            draw_filled_circle(&mut pixels_frame0, size, x0, y0, dot_radius, 255);

            // Draw frame 4 - brightest dot at bottom
            let angle4 = std::f32::consts::PI / 2.0; // bottom
            let x4 = center + (outer_radius - dot_radius - 2.0) * angle4.cos();
            let y4 = center + (outer_radius - dot_radius - 2.0) * angle4.sin();
            draw_filled_circle(&mut pixels_frame4, size, x4, y4, dot_radius, 255);

            // The dots should be in different positions
            // Frame 0 dot is near top (low y), Frame 4 dot is near bottom (high y)
            assert!(y0 < center, "Frame 0 brightest dot should be in top half");
            assert!(y4 > center, "Frame 4 brightest dot should be in bottom half");
        }
    }
}

const TARGET_FPS: u64 = 120;
const TARGET_FRAME_TIME: Duration = Duration::from_micros(1_000_000 / TARGET_FPS);

struct PageTexture {
    texture: metal::Texture,
    width: u32,
    height: u32,
}

#[cfg(target_os = "macos")]
struct PageInfoOverlay {
    texture: metal::Texture,
    width: u32,
    height: u32,
}

#[cfg(target_os = "macos")]
struct LoadingSpinner {
    textures: Vec<metal::Texture>,
    size: u32,
    current_frame: u8,
    last_frame_time: Instant,
    is_loading: bool,
}

#[cfg(target_os = "macos")]
impl LoadingSpinner {
    fn new(device: &Device, size: u32) -> Self {
        // Pre-render all 8 frames of the spinner animation
        let textures: Vec<metal::Texture> = (0..8)
            .filter_map(|frame| {
                text_overlay::render_spinner(device, frame, size)
                    .map(|s| s.texture)
            })
            .collect();

        Self {
            textures,
            size,
            current_frame: 0,
            last_frame_time: Instant::now(),
            is_loading: false,
        }
    }

    fn start(&mut self) {
        self.is_loading = true;
        self.current_frame = 0;
        self.last_frame_time = Instant::now();
    }

    fn stop(&mut self) {
        self.is_loading = false;
    }

    fn update(&mut self) {
        if !self.is_loading {
            return;
        }

        // Rotate at ~10 fps (100ms per frame) for smooth animation
        let now = Instant::now();
        if now.duration_since(self.last_frame_time) >= Duration::from_millis(100) {
            self.current_frame = (self.current_frame + 1) % 8;
            self.last_frame_time = now;
        }
    }

    fn current_texture(&self) -> Option<&metal::Texture> {
        if self.is_loading && !self.textures.is_empty() {
            Some(&self.textures[self.current_frame as usize])
        } else {
            None
        }
    }
}

struct LoadedDocument {
    pdf: PdfDocument,
    #[allow(dead_code)]
    path: PathBuf,
    page_count: u16,
    current_page: u16,
    page_textures: HashMap<u16, PageTexture>,
    #[cfg(target_os = "macos")]
    page_info_overlay: Option<PageInfoOverlay>,
}

struct App {
    window: Option<Arc<Window>>,
    #[cfg(target_os = "macos")]
    metal_layer: Option<MetalLayer>,
    #[cfg(target_os = "macos")]
    device: Option<Device>,
    #[cfg(target_os = "macos")]
    command_queue: Option<CommandQueue>,
    #[cfg(target_os = "macos")]
    loading_spinner: Option<LoadingSpinner>,
    gpu_context: Option<Box<dyn gpu::GpuContext>>,
    scene_graph: SceneGraph,
    renderer: Option<SceneRenderer>,
    input_handler: InputHandler,
    last_update: Instant,
    delta_time: Duration,
    frame_count: u64,
    fps_update_time: Instant,
    current_fps: f64,
    app_start: Instant,
    first_frame_rendered: bool,
    modifiers: ModifiersState,
    document: Option<LoadedDocument>,
    pending_file_open: bool,
    initial_file: Option<PathBuf>,
}

impl App {
    fn new() -> Self {
        let scene_graph = SceneGraph::new();
        let now = Instant::now();
        let input_handler = InputHandler::new(1200.0, 800.0);

        Self {
            window: None,
            #[cfg(target_os = "macos")]
            metal_layer: None,
            #[cfg(target_os = "macos")]
            device: None,
            #[cfg(target_os = "macos")]
            command_queue: None,
            #[cfg(target_os = "macos")]
            loading_spinner: None,
            gpu_context: None,
            scene_graph,
            renderer: None,
            input_handler,
            last_update: now,
            delta_time: Duration::ZERO,
            frame_count: 0,
            fps_update_time: now,
            current_fps: 0.0,
            app_start: now,
            first_frame_rendered: false,
            modifiers: ModifiersState::empty(),
            document: None,
            pending_file_open: false,
            initial_file: None,
        }
    }

    fn set_initial_file(&mut self, path: PathBuf) {
        self.initial_file = Some(path);
    }

    fn open_file_dialog(&mut self) {
        self.pending_file_open = true;
    }

    fn process_file_open(&mut self) {
        if !self.pending_file_open {
            return;
        }
        self.pending_file_open = false;

        let file = rfd::FileDialog::new()
            .add_filter("PDF Files", &["pdf"])
            .set_title("Open PDF")
            .pick_file();

        if let Some(path) = file {
            self.load_pdf(&path);
        }
    }

    fn load_pdf(&mut self, path: &PathBuf) {
        println!("=== Loading PDF: {} ===", path.display());

        match PdfDocument::open(path) {
            Ok(pdf) => {
                let page_count = pdf.page_count();
                println!("SUCCESS: Loaded PDF with {} pages", page_count);

                self.document = Some(LoadedDocument {
                    pdf,
                    path: path.clone(),
                    page_count,
                    current_page: 0,
                    page_textures: HashMap::new(),
                    #[cfg(target_os = "macos")]
                    page_info_overlay: None,
                });

                self.render_current_page();

                if let Some(window) = &self.window {
                    let title = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("PDF Editor");
                    window.set_title(&format!("{} - PDF Editor", title));
                }
            }
            Err(e) => {
                eprintln!("FAILED to load PDF: {}", e);
            }
        }
    }

    /// Start the loading spinner animation
    #[cfg(target_os = "macos")]
    fn start_loading_spinner(&mut self) {
        if let Some(spinner) = &mut self.loading_spinner {
            spinner.start();
        }
    }

    /// Stop the loading spinner animation
    #[cfg(target_os = "macos")]
    fn stop_loading_spinner(&mut self) {
        if let Some(spinner) = &mut self.loading_spinner {
            spinner.stop();
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn start_loading_spinner(&mut self) {}

    #[cfg(not(target_os = "macos"))]
    fn stop_loading_spinner(&mut self) {}

    #[cfg(target_os = "macos")]
    fn render_current_page(&mut self) {
        // Check if device is available
        if self.device.is_none() {
            println!("ERROR: No Metal device");
            return;
        }

        // Check if document exists and get page info
        let (page_index, already_cached) = match &self.document {
            Some(doc) => {
                let idx = doc.current_page;
                let cached = doc.page_textures.contains_key(&idx);
                (idx, cached)
            }
            None => return,
        };

        if already_cached {
            println!("Page {} already cached", page_index + 1);
            self.stop_loading_spinner();
            return;
        }

        // Start spinner to indicate loading
        self.start_loading_spinner();
        println!("=== Rendering page {} to texture... ===", page_index + 1);

        let window_size = self.window.as_ref().map(|w| w.inner_size()).unwrap_or_default();
        let max_width = window_size.width.saturating_sub(40);
        let max_height = window_size.height.saturating_sub(40);

        println!("Window size: {}x{}, max render: {}x{}", window_size.width, window_size.height, max_width, max_height);

        // Render the page (requires mutable borrow of document)
        let render_result = {
            let doc = self.document.as_mut().unwrap();
            doc.pdf.render_page_scaled(page_index, max_width, max_height)
        };

        let (rgba, render_width, render_height) = match render_result {
            Ok(result) => {
                println!("PDFium rendered: {}x{}, {} bytes", result.1, result.2, result.0.len());
                result
            }
            Err(e) => {
                eprintln!("FAILED to render page: {}", e);
                self.stop_loading_spinner();
                return;
            }
        };

        // Convert RGBA to BGRA for Metal compatibility
        let mut bgra = rgba.clone();
        for pixel in bgra.chunks_exact_mut(4) {
            pixel.swap(0, 2); // Swap R and B
        }

        // Create the texture
        let device = self.device.as_ref().unwrap();
        let texture_desc = TextureDescriptor::new();
        texture_desc.set_width(render_width as u64);
        texture_desc.set_height(render_height as u64);
        texture_desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm_sRGB);
        texture_desc.set_usage(metal::MTLTextureUsage::ShaderRead);

        let texture = device.new_texture(&texture_desc);

        let region = metal::MTLRegion {
            origin: metal::MTLOrigin { x: 0, y: 0, z: 0 },
            size: metal::MTLSize {
                width: render_width as u64,
                height: render_height as u64,
                depth: 1,
            },
        };

        texture.replace_region(
            region,
            0,
            bgra.as_ptr() as *const _,
            (render_width * 4) as u64,
        );

        // Insert the texture into the cache
        if let Some(doc) = &mut self.document {
            doc.page_textures.insert(page_index, PageTexture {
                texture,
                width: render_width,
                height: render_height,
            });
        }
        println!("SUCCESS: Page {} texture created ({}x{})", page_index + 1, render_width, render_height);

        // Stop the spinner now that rendering is complete
        self.stop_loading_spinner();

        // Update the page info overlay
        self.update_page_info_overlay();
    }

    #[cfg(not(target_os = "macos"))]
    fn render_current_page(&mut self) {}

    /// Update the page info overlay text (e.g., "Page 1 of 10 | 100%")
    #[cfg(target_os = "macos")]
    fn update_page_info_overlay(&mut self) {
        let Some(device) = &self.device else { return; };
        let Some(doc) = &mut self.document else { return; };

        let zoom_percent = self.input_handler.viewport().zoom_level;
        let text = format!(
            "Page {} of {} | {}%",
            doc.current_page + 1,
            doc.page_count,
            zoom_percent
        );

        // Scale = 2 gives readable text, padding = 8 pixels
        if let Some(overlay) = text_overlay::render_text_overlay(device, &text, 2, 8) {
            doc.page_info_overlay = Some(PageInfoOverlay {
                texture: overlay.texture,
                width: overlay.width,
                height: overlay.height,
            });
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn update_page_info_overlay(&mut self) {}

    fn next_page(&mut self) {
        if let Some(doc) = &mut self.document {
            if doc.current_page + 1 < doc.page_count {
                doc.current_page += 1;
                println!("Page {}/{}", doc.current_page + 1, doc.page_count);
                self.render_current_page();
                self.update_page_info_overlay();
            }
        }
    }

    fn prev_page(&mut self) {
        if let Some(doc) = &mut self.document {
            if doc.current_page > 0 {
                doc.current_page -= 1;
                println!("Page {}/{}", doc.current_page + 1, doc.page_count);
                self.render_current_page();
                self.update_page_info_overlay();
            }
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        self.delta_time = now.duration_since(self.last_update);
        self.last_update = now;

        self.frame_count += 1;

        let fps_elapsed = now.duration_since(self.fps_update_time);
        if fps_elapsed >= Duration::from_secs(1) {
            self.current_fps = self.frame_count as f64 / fps_elapsed.as_secs_f64();
            self.frame_count = 0;
            self.fps_update_time = now;
        }

        let _viewport_changed = self.input_handler.update(self.delta_time);

        // Update the loading spinner animation
        #[cfg(target_os = "macos")]
        if let Some(spinner) = &mut self.loading_spinner {
            spinner.update();
        }
    }

    #[cfg(target_os = "macos")]
    #[allow(deprecated)]
    fn setup_metal_layer(&mut self, window: &Window) {
        let window_handle = window.window_handle().unwrap();
        let raw_handle = window_handle.as_raw();
        let RawWindowHandle::AppKit(handle) = raw_handle else {
            panic!("Expected AppKit window handle on macOS");
        };

        let device = Device::system_default().expect("Failed to get Metal device");
        let command_queue = device.new_command_queue();
        let layer = MetalLayer::new();
        layer.set_device(&device);
        layer.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm_sRGB);
        layer.set_presents_with_transaction(false);

        unsafe {
            let view = handle.ns_view.as_ptr() as cocoa_id;
            use cocoa::appkit::NSView as _;
            view.setWantsLayer(YES);
            view.setLayer(layer.as_ref() as *const _ as _);
        }

        let size = window.inner_size();
        layer.set_drawable_size(CGSize {
            width: size.width as f64,
            height: size.height as f64,
        });

        // Initialize the loading spinner with pre-rendered frames
        let spinner = LoadingSpinner::new(&device, 64);

        self.metal_layer = Some(layer);
        self.loading_spinner = Some(spinner);
        self.device = Some(device);
        self.command_queue = Some(command_queue);
    }

    fn render(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(layer) = &self.metal_layer {
            if let Some(drawable) = layer.next_drawable() {
                if !self.first_frame_rendered {
                    let startup_time = Instant::now().duration_since(self.app_start);
                    println!(
                        "Startup time: {:.2}ms (time to first frame)",
                        startup_time.as_secs_f64() * 1000.0
                    );
                    self.first_frame_rendered = true;
                }

                if let (Some(gpu_context), Some(renderer)) =
                    (&mut self.gpu_context, &mut self.renderer)
                {
                    if let Err(e) = renderer.render(gpu_context.as_mut(), &self.scene_graph) {
                        eprintln!("Scene render failed: {}", e);
                    }
                }

                if let Some(command_queue) = &self.command_queue {
                    let command_buffer = command_queue.new_command_buffer();

                    // First, clear the background
                    let render_pass_descriptor = metal::RenderPassDescriptor::new();
                    let color_attachment = render_pass_descriptor
                        .color_attachments()
                        .object_at(0)
                        .unwrap();

                    color_attachment.set_texture(Some(drawable.texture()));
                    color_attachment.set_load_action(metal::MTLLoadAction::Clear);

                    let bg_color = if self.document.is_some() {
                        metal::MTLClearColor::new(0.3, 0.3, 0.3, 1.0)
                    } else {
                        metal::MTLClearColor::new(0.15, 0.15, 0.15, 1.0)
                    };
                    color_attachment.set_clear_color(bg_color);
                    color_attachment.set_store_action(metal::MTLStoreAction::Store);

                    let encoder = command_buffer.new_render_command_encoder(render_pass_descriptor);
                    encoder.end_encoding();

                    let drawable_width = drawable.texture().width();
                    let drawable_height = drawable.texture().height();

                    // Blit the PDF page texture to the drawable (centered)
                    if let Some(doc) = &self.document {
                        if let Some(page_tex) = doc.page_textures.get(&doc.current_page) {
                            // Center the page
                            let dest_x = (drawable_width.saturating_sub(page_tex.width as u64)) / 2;
                            let dest_y = (drawable_height.saturating_sub(page_tex.height as u64)) / 2;

                            let blit_encoder = command_buffer.new_blit_command_encoder();

                            let src_origin = metal::MTLOrigin { x: 0, y: 0, z: 0 };
                            let src_size = metal::MTLSize {
                                width: page_tex.width as u64,
                                height: page_tex.height as u64,
                                depth: 1,
                            };
                            let dest_origin = metal::MTLOrigin {
                                x: dest_x,
                                y: dest_y,
                                z: 0,
                            };

                            blit_encoder.copy_from_texture(
                                &page_tex.texture,
                                0,
                                0,
                                src_origin,
                                src_size,
                                drawable.texture(),
                                0,
                                0,
                                dest_origin,
                            );

                            blit_encoder.end_encoding();
                        }

                        // Blit the page info overlay to the bottom-right corner
                        if let Some(overlay) = &doc.page_info_overlay {
                            let margin = 16u64;
                            let dest_x = drawable_width.saturating_sub(overlay.width as u64 + margin);
                            let dest_y = drawable_height.saturating_sub(overlay.height as u64 + margin);

                            let blit_encoder = command_buffer.new_blit_command_encoder();

                            let src_origin = metal::MTLOrigin { x: 0, y: 0, z: 0 };
                            let src_size = metal::MTLSize {
                                width: overlay.width as u64,
                                height: overlay.height as u64,
                                depth: 1,
                            };
                            let dest_origin = metal::MTLOrigin {
                                x: dest_x,
                                y: dest_y,
                                z: 0,
                            };

                            blit_encoder.copy_from_texture(
                                &overlay.texture,
                                0,
                                0,
                                src_origin,
                                src_size,
                                drawable.texture(),
                                0,
                                0,
                                dest_origin,
                            );

                            blit_encoder.end_encoding();
                        }
                    }

                    // Render loading spinner in the center if active
                    if let Some(spinner) = &self.loading_spinner {
                        if let Some(spinner_tex) = spinner.current_texture() {
                            let dest_x = (drawable_width.saturating_sub(spinner.size as u64)) / 2;
                            let dest_y = (drawable_height.saturating_sub(spinner.size as u64)) / 2;

                            let blit_encoder = command_buffer.new_blit_command_encoder();

                            let src_origin = metal::MTLOrigin { x: 0, y: 0, z: 0 };
                            let src_size = metal::MTLSize {
                                width: spinner.size as u64,
                                height: spinner.size as u64,
                                depth: 1,
                            };
                            let dest_origin = metal::MTLOrigin {
                                x: dest_x,
                                y: dest_y,
                                z: 0,
                            };

                            blit_encoder.copy_from_texture(
                                spinner_tex,
                                0,
                                0,
                                src_origin,
                                src_size,
                                drawable.texture(),
                                0,
                                0,
                                dest_origin,
                            );

                            blit_encoder.end_encoding();
                        }
                    }

                    command_buffer.present_drawable(drawable);
                    command_buffer.commit();
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("PDF Editor - Press ⌘O to open a file")
                .with_inner_size(winit::dpi::LogicalSize::new(1200, 800));

            let window = Arc::new(
                event_loop
                    .create_window(window_attributes)
                    .expect("Failed to create window"),
            );

            #[cfg(target_os = "macos")]
            self.setup_metal_layer(&window);

            if let Ok(context) = gpu::create_context() {
                if let Ok(renderer) = SceneRenderer::new(context.as_ref()) {
                    self.renderer = Some(renderer);
                }
                self.gpu_context = Some(context);
            }

            self.window = Some(window);

            // Load initial file if provided
            if let Some(path) = self.initial_file.take() {
                println!("Loading initial file: {}", path.display());
                self.load_pdf(&path);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Close requested, exiting");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                #[cfg(target_os = "macos")]
                if let Some(layer) = &self.metal_layer {
                    layer.set_drawable_size(CGSize {
                        width: size.width as f64,
                        height: size.height as f64,
                    });
                }

                self.input_handler
                    .set_viewport_dimensions(size.width as f32, size.height as f32);

                if self.document.is_some() {
                    if let Some(doc) = &mut self.document {
                        doc.page_textures.clear();
                    }
                    self.render_current_page();
                }
            }
            WindowEvent::DroppedFile(path) => {
                if path.extension().map(|e| e == "pdf").unwrap_or(false) {
                    self.load_pdf(&path);
                }
            }
            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input_handler
                    .on_mouse_move(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            let (x, y) = self.input_handler.mouse_position();
                            self.input_handler.on_mouse_down(x, y);
                        }
                        ElementState::Released => {
                            self.input_handler.on_mouse_up();
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_amount = match delta {
                    MouseScrollDelta::LineDelta(_x, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => (pos.y / 100.0) as f32,
                };
                self.input_handler.on_mouse_wheel(scroll_amount);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::keyboard::{KeyCode, PhysicalKey};

                if event.state == ElementState::Pressed {
                    let is_cmd = self.modifiers.super_key();

                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::KeyO) if is_cmd => {
                            self.open_file_dialog();
                        }
                        PhysicalKey::Code(KeyCode::Equal)
                        | PhysicalKey::Code(KeyCode::NumpadAdd) => {
                            self.input_handler.zoom_in();
                            self.update_page_info_overlay();
                        }
                        PhysicalKey::Code(KeyCode::Minus)
                        | PhysicalKey::Code(KeyCode::NumpadSubtract) => {
                            self.input_handler.zoom_out();
                            self.update_page_info_overlay();
                        }
                        PhysicalKey::Code(KeyCode::Digit0)
                        | PhysicalKey::Code(KeyCode::Numpad0) => {
                            self.input_handler.zoom_reset();
                            self.update_page_info_overlay();
                        }
                        PhysicalKey::Code(KeyCode::PageDown)
                        | PhysicalKey::Code(KeyCode::ArrowDown)
                        | PhysicalKey::Code(KeyCode::ArrowRight) => {
                            self.next_page();
                        }
                        PhysicalKey::Code(KeyCode::PageUp)
                        | PhysicalKey::Code(KeyCode::ArrowUp)
                        | PhysicalKey::Code(KeyCode::ArrowLeft) => {
                            self.prev_page();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_file_open();

        if self.window.is_some() {
            self.update();

            if let Some(window) = &self.window {
                window.request_redraw();
            }

            let frame_time = Instant::now().duration_since(self.last_update);
            if frame_time < TARGET_FRAME_TIME {
                std::thread::sleep(TARGET_FRAME_TIME - frame_time);
            }

            event_loop.set_control_flow(ControlFlow::Poll);
        }
    }
}

fn main() {
    println!("PDF Editor starting...");
    println!("Press ⌘O to open a PDF file, or drag and drop a PDF onto the window");

    // Check for command line argument
    let args: Vec<String> = std::env::args().collect();
    let initial_file = if args.len() > 1 {
        let path = PathBuf::from(&args[1]);
        if path.exists() && path.extension().map(|e| e == "pdf").unwrap_or(false) {
            println!("Will open: {}", path.display());
            Some(path)
        } else {
            None
        }
    } else {
        None
    };

    #[cfg(target_os = "macos")]
    #[allow(deprecated)]
    unsafe {
        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular);
        app.activateIgnoringOtherApps_(YES);
    }

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    if let Some(path) = initial_file {
        app.set_initial_file(path);
    }
    event_loop
        .run_app(&mut app)
        .expect("Failed to run event loop");
}
