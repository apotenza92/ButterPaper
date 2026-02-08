//! Unified settings window.

#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]

use crate::components::{segmented_control, settings_group, settings_row, SegmentOption};
use crate::ui::{sizes, TypographyExt};
use crate::ui_preferences::save_ui_preferences_from_app;
use crate::{current_theme, AppearanceMode, CloseWindow, Theme};
use gpui::{
    actions, div, point, prelude::*, px, size, App, Context, FocusHandle, Focusable, KeyBinding,
    SharedString, Window, WindowBounds, WindowOptions,
};

actions!(settings, [OpenSettings, CloseSettings]);

/// Register settings-related keybindings
pub fn register_bindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("escape", CloseSettings, Some("SettingsView")),
    ]);
}

/// Open the settings window
pub fn open_settings_window(cx: &mut App) {
    let bounds = gpui::Bounds::centered(None, size(px(760.0), px(560.0)), cx);

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("Settings".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(12.0), px(9.0))),
            }),
            focus: true,
            show: true,
            is_movable: true,
            window_min_size: Some(size(px(600.0), px(400.0))),
            ..Default::default()
        },
        |window, cx| {
            let view = cx.new(SettingsView::new);
            view.focus_handle(cx).focus(window);
            view
        },
    )
    .ok();
}

/// The settings view
pub struct SettingsView {
    focus_handle: FocusHandle,
}

impl SettingsView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle() }
    }

    fn handle_close(&mut self, _: &CloseSettings, window: &mut Window, _cx: &mut Context<Self>) {
        window.remove_window();
    }

    fn handle_close_window(
        &mut self,
        _: &CloseWindow,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        window.remove_window();
    }
}

impl Focusable for SettingsView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = current_theme(window, cx);

        div()
            .id("settings-view")
            .key_context("SettingsView")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_close))
            .on_action(cx.listener(Self::handle_close_window))
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.surface)
            .text_color(theme.text)
            .child(crate::ui::title_bar("Settings", theme.text, theme.border))
            .child(self.render_content(&theme, cx))
    }
}

impl SettingsView {
    fn render_appearance_mode_control(
        &self,
        current_mode: AppearanceMode,
        theme: &Theme,
    ) -> impl IntoElement {
        let selected = match current_mode {
            AppearanceMode::System => "System",
            AppearanceMode::Light => "Light",
            AppearanceMode::Dark => "Dark",
        };

        segmented_control(
            "settings.appearance.mode",
            vec![
                SegmentOption::simple("System"),
                SegmentOption::simple("Light"),
                SegmentOption::simple("Dark"),
            ],
            selected,
            theme,
            |label, cx| {
                let mode = match label {
                    "Light" => AppearanceMode::Light,
                    "Dark" => AppearanceMode::Dark,
                    _ => AppearanceMode::System,
                };
                cx.set_global(mode);
                #[cfg(target_os = "macos")]
                crate::macos::set_app_appearance(mode);
                let _ = save_ui_preferences_from_app(cx);
                cx.refresh_windows();
            },
        )
    }

    fn render_content_group(
        &self,
        content: Vec<gpui::AnyElement>,
        theme: &Theme,
    ) -> impl IntoElement {
        settings_group(None::<SharedString>, content, theme)
    }

    fn render_content_row(
        &self,
        title: &'static str,
        description: &'static str,
        control: impl IntoElement,
        theme: &Theme,
    ) -> impl IntoElement {
        settings_row(title, description, control, theme)
    }

    fn render_content(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let current_mode = cx.try_global::<AppearanceMode>().copied().unwrap_or_default();

        div()
            .id("settings-content")
            .flex()
            .flex_col()
            .flex_1()
            .h_full()
            .min_w_0()
            .overflow_hidden()
            .bg(theme.elevated_surface)
            .pt(sizes::SPACE_3)
            .overflow_y_scroll()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .max_w(sizes::SETTINGS_CONTENT_MAX_WIDTH)
                    .px(sizes::SPACE_6)
                    .pb(sizes::SPACE_6)
                    .child(
                        div()
                            .text_ui_title()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .mb(sizes::SPACE_4)
                            .child("Appearance"),
                    )
                    .child(self.render_appearance_content(theme, current_mode).into_any_element()),
            )
    }

    fn render_appearance_content(
        &self,
        theme: &Theme,
        current_mode: AppearanceMode,
    ) -> impl IntoElement {
        self.render_content_group(
            vec![self
                .render_content_row(
                    "Appearance",
                    "Choose whether to use light mode, dark mode, or follow system settings.",
                    self.render_appearance_mode_control(current_mode, theme),
                    theme,
                )
                .into_any_element()],
            theme,
        )
    }
}
