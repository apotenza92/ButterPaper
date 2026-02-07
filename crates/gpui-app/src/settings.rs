//! Unified settings window matching Zed's settings UI style

#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]

use crate::components::{
    checkbox, nav_item, segmented_control, settings_group, settings_row, Dropdown, DropdownOption,
    SegmentOption,
};
use crate::theme::{theme_registry, ThemeSettings};
use crate::ui::{sizes, TypographyExt};
use crate::ui_preferences::save_ui_preferences_from_app;
use crate::workspace::{load_preferences, save_preferences, TabPreferences};
use crate::{current_theme, AppearanceMode, CloseWindow, Theme};
use gpui::{
    actions, div, point, prelude::*, px, size, App, Context, FocusHandle, Focusable, KeyBinding,
    SharedString, Window, WindowBounds, WindowOptions,
};

actions!(settings, [OpenSettings, CloseSettings]);

/// Navigation pages in settings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SettingsPage {
    #[default]
    Appearance,
    Behavior,
}

impl SettingsPage {
    fn label(&self) -> &'static str {
        match self {
            SettingsPage::Appearance => "Appearance",
            SettingsPage::Behavior => "Behavior",
        }
    }

    fn all() -> &'static [SettingsPage] {
        &[SettingsPage::Appearance, SettingsPage::Behavior]
    }
}

/// Which dropdown is currently open
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenDropdown {
    None,
    LightTheme,
    DarkTheme,
}

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
    current_page: SettingsPage,
    open_dropdown: OpenDropdown,
    tab_preferences: TabPreferences,
}

impl SettingsView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            current_page: SettingsPage::default(),
            open_dropdown: OpenDropdown::None,
            tab_preferences: load_preferences(),
        }
    }

    fn toggle_prefer_tabs(&mut self, cx: &mut Context<Self>) {
        self.tab_preferences.prefer_tabs = !self.tab_preferences.prefer_tabs;
        let _ = save_preferences(&self.tab_preferences);
        cx.notify();
    }

    fn toggle_show_tab_bar(&mut self, cx: &mut Context<Self>) {
        self.tab_preferences.show_tab_bar = !self.tab_preferences.show_tab_bar;
        let _ = save_preferences(&self.tab_preferences);
        cx.notify();
    }

    fn toggle_allow_merge(&mut self, cx: &mut Context<Self>) {
        self.tab_preferences.allow_merge = !self.tab_preferences.allow_merge;
        let _ = save_preferences(&self.tab_preferences);
        cx.notify();
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

    fn select_page(&mut self, page: SettingsPage, cx: &mut Context<Self>) {
        self.current_page = page;
        self.open_dropdown = OpenDropdown::None;
        cx.notify();
    }

    fn toggle_dropdown(&mut self, dropdown: OpenDropdown, cx: &mut Context<Self>) {
        if self.open_dropdown == dropdown {
            self.open_dropdown = OpenDropdown::None;
        } else {
            self.open_dropdown = dropdown;
        }
        cx.notify();
    }

    fn close_dropdown(&mut self, cx: &mut Context<Self>) {
        self.open_dropdown = OpenDropdown::None;
        cx.notify();
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
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.close_dropdown(cx);
            }))
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.surface)
            .text_color(theme.text)
            // Title bar with centered title
            .child(crate::ui::title_bar("Settings", theme.text, theme.border))
            // Main content: sidebar + content area
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .child(self.render_sidebar(&theme, cx))
                    .child(self.render_content(&theme, cx)),
            )
    }
}

impl SettingsView {
    fn render_sidebar(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w(sizes::SIDEBAR_WIDTH)
            .h_full()
            .flex_shrink_0()
            .bg(theme.surface)
            .border_r_1()
            .border_color(theme.border)
            .p(sizes::SPACE_3)
            .gap(sizes::SPACE_1)
            .children(
                SettingsPage::all()
                    .iter()
                    .map(|&page| self.render_nav_item(page, page == self.current_page, theme, cx)),
            )
    }

    fn render_nav_item(
        &self,
        page: SettingsPage,
        selected: bool,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        nav_item(
            page.label(),
            selected,
            theme,
            cx.listener(move |this, _, _window, cx| {
                this.select_page(page, cx);
            }),
        )
    }

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
        let theme_settings = cx.try_global::<ThemeSettings>().cloned().unwrap_or_default();

        div()
            .id("settings-content")
            .flex()
            .flex_col()
            .flex_1()
            .h_full()
            .min_w_0()
            .overflow_hidden() // Prevent any content from escaping
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
                            .child(self.current_page.label()),
                    )
                    .child(match self.current_page {
                        SettingsPage::Appearance => self
                            .render_appearance_content(theme, current_mode, theme_settings, cx)
                            .into_any_element(),
                        SettingsPage::Behavior => {
                            self.render_behavior_content(theme, cx).into_any_element()
                        }
                    }),
            )
    }

    fn render_appearance_content(
        &self,
        theme: &Theme,
        current_mode: AppearanceMode,
        theme_settings: ThemeSettings,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let registry = theme_registry();
        let light_themes: Vec<String> =
            registry.light_themes().iter().map(|s| s.to_string()).collect();
        let dark_themes: Vec<String> =
            registry.dark_themes().iter().map(|s| s.to_string()).collect();

        self.render_content_group(
            vec![
                self.render_content_row(
                    "Appearance",
                    "Choose whether to use light or dark theme, or follow system settings.",
                    self.render_appearance_mode_control(current_mode, theme),
                    theme,
                )
                .into_any_element(),
                self.render_content_row(
                    "Light Theme",
                    "Theme used when in light mode.",
                    self.render_dropdown(
                        "settings.light_theme",
                        "Light Theme",
                        &theme_settings.light_theme,
                        self.open_dropdown == OpenDropdown::LightTheme,
                        light_themes,
                        OpenDropdown::LightTheme,
                        |name, cx| {
                            let mut settings =
                                cx.try_global::<ThemeSettings>().cloned().unwrap_or_default();
                            settings.light_theme = name.to_string();
                            cx.set_global(settings);
                            let _ = save_ui_preferences_from_app(cx);
                            cx.refresh_windows();
                        },
                        theme,
                        cx,
                    ),
                    theme,
                )
                .into_any_element(),
                self.render_content_row(
                    "Dark Theme",
                    "Theme used when in dark mode.",
                    self.render_dropdown(
                        "settings.dark_theme",
                        "Dark Theme",
                        &theme_settings.dark_theme,
                        self.open_dropdown == OpenDropdown::DarkTheme,
                        dark_themes,
                        OpenDropdown::DarkTheme,
                        |name, cx| {
                            let mut settings =
                                cx.try_global::<ThemeSettings>().cloned().unwrap_or_default();
                            settings.dark_theme = name.to_string();
                            cx.set_global(settings);
                            let _ = save_ui_preferences_from_app(cx);
                            cx.refresh_windows();
                        },
                        theme,
                        cx,
                    ),
                    theme,
                )
                .into_any_element(),
            ],
            theme,
        )
    }

    fn render_behavior_content(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let prefer_tabs = self.tab_preferences.prefer_tabs;
        let show_tab_bar = self.tab_preferences.show_tab_bar;
        let allow_merge = self.tab_preferences.allow_merge;

        self.render_content_group(
            vec![
                self.render_content_row(
                    "Open PDFs in tabs",
                    "New PDFs open as tabs in the current window instead of new windows.",
                    div().flex().justify_end().w_full().child(checkbox(
                        "prefer-tabs",
                        prefer_tabs,
                        theme,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_prefer_tabs(cx);
                        }),
                    )),
                    theme,
                )
                .into_any_element(),
                self.render_content_row(
                    "Show tab bar",
                    "Always show the tab bar, even when only one document is open.",
                    div().flex().justify_end().w_full().child(checkbox(
                        "show-tab-bar",
                        show_tab_bar,
                        theme,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_show_tab_bar(cx);
                        }),
                    )),
                    theme,
                )
                .into_any_element(),
                self.render_content_row(
                    "Allow window merging",
                    "Drag tabs between windows to merge them together.",
                    div().flex().justify_end().w_full().child(checkbox(
                        "allow-merge",
                        allow_merge,
                        theme,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_allow_merge(cx);
                        }),
                    )),
                    theme,
                )
                .into_any_element(),
            ],
            theme,
        )
    }

    /// Render a dropdown menu that stays within window bounds
    fn render_dropdown<F>(
        &self,
        element_id: &str,
        element_name: &str,
        current_value: &str,
        is_open: bool,
        options: Vec<String>,
        dropdown_id: OpenDropdown,
        on_select: F,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        F: Fn(&str, &mut App) + Clone + 'static,
    {
        // Reserved for dev tooling hooks
        let _ = (element_id, element_name);

        let options = options
            .into_iter()
            .map(|label| DropdownOption::new(label.clone(), label))
            .collect::<Vec<_>>();
        let owner = cx.entity().downgrade();

        Dropdown::new(
            SharedString::from(format!("dropdown-{:?}", dropdown_id)),
            move |value, cx| {
                on_select(value, cx);
                if let Some(settings) = owner.upgrade() {
                    settings.update(cx, |this, cx| {
                        this.close_dropdown(cx);
                    });
                }
            },
        )
        .options(options)
        .selected(current_value.to_string())
        .on_toggle({
            let owner = cx.entity().downgrade();
            move |cx| {
                if let Some(settings) = owner.upgrade() {
                    settings.update(cx, |this, cx| {
                        this.toggle_dropdown(dropdown_id, cx);
                    });
                }
            }
        })
        .render(is_open, theme)
    }
}
