//! Mouse coordinate calibration tool
//!
//! Creates a window with markers at known positions to verify coordinate calculations.
//!
//! With transparent titlebar, GPUI coordinates start at (0,0) at the top-left of the
//! entire window area (including the 32px title bar region). Mouse events report in
//! these GPUI coordinates.
//!
//! When clicking via enigo:
//!   screen_x = window_x + gpui_x  
//!   screen_y = window_y + gpui_y
//!
//! No additional title bar offset needed because GPUI coords include it.

use gpui::{
    div, point, prelude::*, px, rgb, size, App, Application, Bounds, Context, MouseMoveEvent,
    TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use pdf_editor_gpui::ui;

struct CalibrationWindow {
    mouse_pos: Option<gpui::Point<gpui::Pixels>>,
    clicks: Vec<(i32, i32)>,
}

impl CalibrationWindow {
    fn new() -> Self {
        Self {
            mouse_pos: None,
            clicks: Vec::new(),
        }
    }
}

impl Render for CalibrationWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _mouse_text = if let Some(pos) = self.mouse_pos {
            format!("({:.0}, {:.0})", pos.x.0, pos.y.0)
        } else {
            "(-, -)".to_string()
        };

        let _last_click = self.clicks.last().map(|(x, y)| format!("({}, {})", x, y));

        div()
            .id("calibration")
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                this.mouse_pos = Some(event.position);
                cx.notify();
            }))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, _window, cx| {
                    let x = event.position.x.0 as i32;
                    let y = event.position.y.0 as i32;
                    this.clicks.push((x, y));
                    eprintln!("CLICK at GPUI: ({}, {})", x, y);
                    cx.notify();
                }),
            )
            // Title bar with centered title
            .child(ui::title_bar(
                "Calibration",
                rgb(0xcdd6f4),
                rgb(0x45475a),
            ))
            // Content area with grid of markers
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .relative()
                    .bg(rgb(0x181825))
                    .overflow_hidden()
                    // Grid markers - well spaced, no overlaps
                    // Row 1: y=40 (content-relative)
                    .child(marker(80, 40, "80,72"))
                    .child(marker(200, 40, "200,72"))
                    .child(marker(320, 40, "320,72"))
                    // Row 2: y=120
                    .child(marker(80, 120, "80,152"))
                    .child(marker(200, 120, "200,152"))
                    .child(marker(320, 120, "320,152"))
                    // Row 3: y=200
                    .child(marker(80, 200, "80,232"))
                    .child(marker(200, 200, "200,232"))
                    .child(marker(320, 200, "320,232"))
                    // Corner markers
                    .child(marker(440, 40, "440,72"))
                    .child(marker(440, 200, "440,232"))
                    .child(marker(440, 280, "440,312")),
            )
            // Footer with instructions
            .child(
                div()
                    .h(px(28.0))
                    .w_full()
                    .px(px(16.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(rgb(0x313244))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x6c7086))
                            .child("Click markers to verify • Labels show GPUI coords"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x6c7086))
                            .child(format!("Clicks: {}", self.clicks.len())),
                    ),
            )
    }
}

/// Create a marker at content-area position
/// Label shows the GPUI Y coordinate (content_y + 32)
fn marker(x: i32, y: i32, gpui_label: &str) -> impl IntoElement {
    div()
        .absolute()
        .left(px(x as f32))
        .top(px(y as f32))
        .flex()
        .items_center()
        .gap(px(6.0))
        // Dot
        .child(
            div()
                .size(px(10.0))
                .rounded(px(5.0))
                .bg(rgb(0xf38ba8))
                .border_2()
                .border_color(rgb(0xffffff)),
        )
        // Label
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x9399b2))
                .child(gpui_label.to_string()),
        )
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds {
            origin: point(px(100.0), px(100.0)),
            size: size(px(520.0), px(380.0)),
        };

        eprintln!("╭─────────────────────────────────────────────╮");
        eprintln!("│  Calibration Window                         │");
        eprintln!("├─────────────────────────────────────────────┤");
        eprintln!("│  Window position: (100, 100)                │");
        eprintln!("│  Window size: 520×380                       │");
        eprintln!("│  Title bar: 32px                            │");
        eprintln!("├─────────────────────────────────────────────┤");
        eprintln!("│  Labels show GPUI coords (content_y + 32)   │");
        eprintln!("│  Screen = Window + GPUI                     │");
        eprintln!("│                                             │");
        eprintln!("│  Example: marker \"200,232\"                  │");
        eprintln!("│    Screen = (100+200, 100+232) = (300, 332) │");
        eprintln!("╰─────────────────────────────────────────────╯");

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Calibration".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(12.0), px(9.0))),
                }),
                focus: true,
                show: true,
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| CalibrationWindow::new()),
        )
        .unwrap();

        cx.activate(true);
    });
}
