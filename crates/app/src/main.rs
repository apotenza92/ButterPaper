//! PDF Editor Application
//!
//! Main application entry point with GPU-rendered UI shell.

use pdf_editor_ui::gpu;
use pdf_editor_ui::renderer::SceneRenderer;
use pdf_editor_ui::scene::{Color, Primitive, Rect, SceneGraph};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[cfg(target_os = "macos")]
#[allow(deprecated)]
use cocoa::base::id as cocoa_id;
#[cfg(target_os = "macos")]
use core_graphics_types::geometry::CGSize;
#[cfg(target_os = "macos")]
use metal::{Device, MetalLayer};
#[cfg(target_os = "macos")]
use objc::runtime::YES;
#[cfg(target_os = "macos")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Target frame rate (60 FPS)
const TARGET_FPS: u64 = 60;
const TARGET_FRAME_TIME: Duration = Duration::from_micros(1_000_000 / TARGET_FPS);

/// Application state
struct App {
    window: Option<Arc<Window>>,
    #[cfg(target_os = "macos")]
    metal_layer: Option<MetalLayer>,
    #[cfg(target_os = "macos")]
    device: Option<Device>,
    gpu_context: Option<Box<dyn gpu::GpuContext>>,
    scene_graph: SceneGraph,
    renderer: Option<SceneRenderer>,
    // Frame loop timing
    last_update: Instant,
    delta_time: Duration,
    frame_count: u64,
    fps_update_time: Instant,
    current_fps: f64,
}

impl App {
    fn new() -> Self {
        // Create a sample scene graph with some test primitives
        let mut scene_graph = SceneGraph::new();
        let root = scene_graph.root_mut();

        // Add a red rectangle in the center
        root.add_primitive(Primitive::Rectangle {
            rect: Rect::new(500.0, 300.0, 200.0, 200.0),
            color: Color::rgb(0.8, 0.2, 0.2),
        });

        // Add a blue rectangle in the top-left
        root.add_primitive(Primitive::Rectangle {
            rect: Rect::new(50.0, 50.0, 150.0, 100.0),
            color: Color::rgb(0.2, 0.2, 0.8),
        });

        // Add a green circle
        root.add_primitive(Primitive::Circle {
            center: [900.0, 400.0],
            radius: 75.0,
            color: Color::rgb(0.2, 0.8, 0.2),
        });

        let now = Instant::now();
        Self {
            window: None,
            #[cfg(target_os = "macos")]
            metal_layer: None,
            #[cfg(target_os = "macos")]
            device: None,
            gpu_context: None,
            scene_graph,
            renderer: None,
            last_update: now,
            delta_time: Duration::ZERO,
            frame_count: 0,
            fps_update_time: now,
            current_fps: 0.0,
        }
    }

    /// Update game state (called every frame)
    fn update(&mut self) {
        let now = Instant::now();
        self.delta_time = now.duration_since(self.last_update);
        self.last_update = now;

        // Update frame counter
        self.frame_count += 1;

        // Update FPS counter every second
        let fps_elapsed = now.duration_since(self.fps_update_time);
        if fps_elapsed >= Duration::from_secs(1) {
            self.current_fps = self.frame_count as f64 / fps_elapsed.as_secs_f64();
            self.frame_count = 0;
            self.fps_update_time = now;

            // Log FPS for debugging
            println!(
                "FPS: {:.1} | Frame time: {:.2}ms",
                self.current_fps,
                self.delta_time.as_secs_f64() * 1000.0
            );
        }

        // Future: Update scene graph animations, physics, etc.
        // For now, this is where frame-by-frame updates will happen
    }

    #[cfg(target_os = "macos")]
    #[allow(deprecated)]
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
                // Render the scene graph using our renderer
                if let (Some(gpu_context), Some(renderer)) = (&mut self.gpu_context, &mut self.renderer) {
                    // Render scene graph (currently just validates the structure)
                    if let Err(e) = renderer.render(gpu_context.as_mut(), &self.scene_graph) {
                        eprintln!("Scene render failed: {}", e);
                    }
                }

                // Create a render pass that clears to a dark gray
                // In a full implementation, the scene renderer would draw primitives here
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
                    // Initialize scene renderer
                    match SceneRenderer::new(context.as_ref()) {
                        Ok(renderer) => {
                            self.renderer = Some(renderer);
                            println!("Scene renderer initialized successfully");
                        }
                        Err(e) => {
                            eprintln!("Failed to initialize scene renderer: {}", e);
                        }
                    }

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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            // Game-style frame loop: update every frame
            self.update();

            // Request immediate redraw for continuous rendering
            if let Some(window) = &self.window {
                window.request_redraw();
            }

            // Optional: Sleep to maintain target frame rate
            // This prevents excessive CPU usage while maintaining smooth updates
            let frame_time = Instant::now().duration_since(self.last_update);
            if frame_time < TARGET_FRAME_TIME {
                std::thread::sleep(TARGET_FRAME_TIME - frame_time);
            }

            // Set control flow to poll continuously for game-style updates
            event_loop.set_control_flow(ControlFlow::Poll);
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
