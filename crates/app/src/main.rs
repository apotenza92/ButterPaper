//! PDF Editor Application
//!
//! Main application entry point with GPU-rendered UI shell.

use pdf_editor_ui::gpu;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[cfg(target_os = "macos")]
use cocoa::base::id as cocoa_id;
#[cfg(target_os = "macos")]
use core_graphics_types::geometry::CGSize;
#[cfg(target_os = "macos")]
use metal::{Device, MetalLayer};
#[cfg(target_os = "macos")]
use objc::runtime::YES;
#[cfg(target_os = "macos")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Application state
struct App {
    window: Option<Arc<Window>>,
    #[cfg(target_os = "macos")]
    metal_layer: Option<MetalLayer>,
    #[cfg(target_os = "macos")]
    device: Option<Device>,
    gpu_context: Option<Box<dyn gpu::GpuContext>>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            #[cfg(target_os = "macos")]
            metal_layer: None,
            #[cfg(target_os = "macos")]
            device: None,
            gpu_context: None,
        }
    }

    #[cfg(target_os = "macos")]
    fn setup_metal_layer(&mut self, window: &Window) {
        // Get the raw window handle
        let window_handle = window.window_handle().unwrap();
        let raw_handle = window_handle.as_raw();
        let RawWindowHandle::AppKit(handle) = raw_handle else {
            panic!("Expected AppKit window handle on macOS");
        };

        // Create Metal device
        let device = Device::system_default().expect("Failed to get Metal device");

        // Create Metal layer
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

        self.metal_layer = Some(layer);
        self.device = Some(device);
    }

    fn render(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(layer) = &self.metal_layer {
            if let Some(drawable) = layer.next_drawable() {
                // Create a simple render pass that clears to a dark gray
                if let Some(device) = &self.device {
                    let command_queue = device.new_command_queue();
                    let command_buffer = command_queue.new_command_buffer();

                    let render_pass_descriptor = metal::RenderPassDescriptor::new();
                    let color_attachment = render_pass_descriptor
                        .color_attachments()
                        .object_at(0)
                        .unwrap();

                    color_attachment.set_texture(Some(drawable.texture()));
                    color_attachment.set_load_action(metal::MTLLoadAction::Clear);
                    color_attachment.set_clear_color(metal::MTLClearColor::new(0.2, 0.2, 0.2, 1.0));
                    color_attachment.set_store_action(metal::MTLStoreAction::Store);

                    let encoder = command_buffer.new_render_command_encoder(render_pass_descriptor);
                    encoder.end_encoding();

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
                .with_title("PDF Editor")
                .with_inner_size(winit::dpi::LogicalSize::new(1200, 800));

            let window = Arc::new(
                event_loop
                    .create_window(window_attributes)
                    .expect("Failed to create window"),
            );

            #[cfg(target_os = "macos")]
            self.setup_metal_layer(&window);

            // Initialize GPU context
            match gpu::create_context() {
                Ok(context) => {
                    self.gpu_context = Some(context);
                    println!("GPU context initialized successfully");
                }
                Err(e) => {
                    eprintln!("Failed to initialize GPU context: {}", e);
                }
            }

            self.window = Some(window);
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
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    println!("Starting PDF Editor...");

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Failed to run event loop");
}
