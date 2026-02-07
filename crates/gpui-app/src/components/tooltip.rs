//! Shared tooltip component for consistent tooltip styling across the app.

use gpui::{div, prelude::*, Rgba, SharedString};

use crate::ui::{sizes, TypographyExt};

/// Simple tooltip content view for displaying hover information.
pub struct TooltipView {
    pub text: SharedString,
    pub bg: Rgba,
    pub border: Rgba,
    pub text_color: Rgba,
}

impl Render for TooltipView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        div()
            .px(sizes::SPACE_2)
            .py(sizes::SPACE_1)
            .bg(self.bg)
            .border_1()
            .border_color(self.border)
            .rounded(sizes::RADIUS_SM)
            .shadow_md()
            .text_ui_body()
            .text_color(self.text_color)
            .child(self.text.clone())
    }
}

/// Helper to create tooltip closure for use with `.tooltip()`.
///
/// # Example
/// ```ignore
/// .tooltip(tooltip_builder("My tooltip", theme.surface, theme.border, theme.text))
/// ```
pub fn tooltip_builder(
    text: impl Into<SharedString>,
    bg: Rgba,
    border: Rgba,
    text_color: Rgba,
) -> impl Fn(&mut gpui::Window, &mut gpui::App) -> gpui::AnyView + 'static {
    let text = text.into();
    move |_window, cx| cx.new(|_| TooltipView { text: text.clone(), bg, border, text_color }).into()
}
