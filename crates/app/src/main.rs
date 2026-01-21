//! PDF Editor Application
//!
//! Main application entry point with GPU-rendered UI shell.

use pdf_editor_ui::gpu;
use pdf_editor_ui::input::InputHandler;
use pdf_editor_ui::renderer::SceneRenderer;
use pdf_editor_ui::scene::SceneGraph;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[cfg(target_os = "macos")]
#[allow(deprecated)]
use cocoa::base::id as cocoa_id;
#[cfg(target_os = "macos")]
use core_graphics_types::geometry::CGSize;
#[cfg(target_os = "macos")]
use metal::{CommandQueue, Device, MetalLayer};
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
    #[cfg(target_os = "macos")]
    command_queue: Option<CommandQueue>,
    gpu_context: Option<Box<dyn gpu::GpuContext>>,
    scene_graph: SceneGraph,
    renderer: Option<SceneRenderer>,
    input_handler: InputHandler,
    // Frame loop timing
    last_update: Instant,
    delta_time: Duration,
    frame_count: u64,
    fps_update_time: Instant,
    current_fps: f64,
    // Startup timing
    app_start: Instant,
    first_frame_rendered: bool,
}

impl App {
    fn new() -> Self {
        // Create an empty scene graph (test primitives removed for faster startup)
        let scene_graph = SceneGraph::new();

        let now = Instant::now();
        let input_handler = InputHandler::new(1200.0, 800.0); // Default window size

        Self {
            window: None,
            #[cfg(target_os = "macos")]
            metal_layer: None,
            #[cfg(target_os = "macos")]
            device: None,
            #[cfg(target_os = "macos")]
            command_queue: None,
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
            let viewport = self.input_handler.viewport();
            println!(
                "FPS: {:.1} | Frame time: {:.2}ms | Zoom: {}% | Pos: ({:.0}, {:.0}) | Page: {}",
                self.current_fps,
                self.delta_time.as_secs_f64() * 1000.0,
                viewport.zoom_level,
                viewport.x,
                viewport.y,
                viewport.page_index
            );
        }

        // Update input handler (smooth pan/zoom animations)
        let _viewport_changed = self.input_handler.update(self.delta_time);

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

        // Create command queue once (reused for all frames)
        let command_queue = device.new_command_queue();

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
        self.command_queue = Some(command_queue);
    }

    fn render(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(layer) = &self.metal_layer {
            if let Some(drawable) = layer.next_drawable() {
                // Track time to first frame
                if !self.first_frame_rendered {
                    let startup_time = Instant::now().duration_since(self.app_start);
                    println!("Startup time: {:.2}ms (time to first frame)", startup_time.as_secs_f64() * 1000.0);
                    self.first_frame_rendered = true;
                }

                // Render the scene graph using our renderer
                if let (Some(gpu_context), Some(renderer)) = (&mut self.gpu_context, &mut self.renderer) {
                    // Render scene graph (currently just validates the structure)
                    if let Err(e) = renderer.render(gpu_context.as_mut(), &self.scene_graph) {
                        eprintln!("Scene render failed: {}", e);
                    }
                }

                // Create a render pass that clears to a dark gray
                // In a full implementation, the scene renderer would draw primitives here
                if let Some(command_queue) = &self.command_queue {
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

            // Initialize GPU context and renderer
            if let Ok(context) = gpu::create_context() {
                if let Ok(renderer) = SceneRenderer::new(context.as_ref()) {
                    self.renderer = Some(renderer);
                }
                self.gpu_context = Some(context);
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

                // Update input handler viewport dimensions
                self.input_handler
                    .set_viewport_dimensions(size.width as f32, size.height as f32);
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
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Equal) | PhysicalKey::Code(KeyCode::NumpadAdd) => {
                            self.input_handler.zoom_in();
                        }
                        PhysicalKey::Code(KeyCode::Minus) | PhysicalKey::Code(KeyCode::NumpadSubtract) => {
                            self.input_handler.zoom_out();
                        }
                        PhysicalKey::Code(KeyCode::Digit0) | PhysicalKey::Code(KeyCode::Numpad0) => {
                            self.input_handler.zoom_reset();
                        }
                        PhysicalKey::Code(KeyCode::PageDown) | PhysicalKey::Code(KeyCode::ArrowDown) => {
                            self.input_handler.next_page();
                        }
                        PhysicalKey::Code(KeyCode::PageUp) | PhysicalKey::Code(KeyCode::ArrowUp) => {
                            self.input_handler.prev_page();
                        }
                        _ => {}
                    }
                }
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
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Failed to run event loop");
}
