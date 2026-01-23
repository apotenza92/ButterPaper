//! Unified settings window matching Zed's settings UI style

#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]

use crate::components::toggle_switch;
use crate::theme::{theme_registry, ThemeSettings};
use crate::ui::sizes;
use crate::workspace::{load_preferences, save_preferences, TabPreferences};
use crate::{current_theme, AppearanceMode, CloseWindow, Theme};
use gpui::{
    actions, deferred, div, point, prelude::*, px, size, App, Context, FocusHandle, Focusable,
    KeyBinding, SharedString, Window, WindowBounds, WindowOptions,
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
    AppearanceMode,
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

    /// Create with a specific dropdown already open (for screenshot testing)
    pub fn new_with_open_dropdown(dropdown: &str, cx: &mut Context<Self>) -> Self {
        let open_dropdown = match dropdown {
            "appearance" => OpenDropdown::AppearanceMode,
            "light" => OpenDropdown::LightTheme,
            "dark" => OpenDropdown::DarkTheme,
            _ => OpenDropdown::None,
        };
        Self {
            focus_handle: cx.focus_handle(),
            current_page: SettingsPage::default(),
            open_dropdown,
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
            .p(sizes::PADDING_LG)
            .gap(px(2.0))
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
        div()
            .id(page.label())
            .h(sizes::CONTROL_HEIGHT)
            .px(sizes::PADDING_MD)
            .flex()
            .items_center()
            .rounded(sizes::RADIUS_SM)
            .cursor_pointer()
            .text_sm()
            .when(selected, |d| d.bg(theme.element_selected))
            .when(!selected, |d| d.hover(|s| s.bg(theme.element_hover)))
            .on_click(cx.listener(move |this, _, _window, cx| {
                this.select_page(page, cx);
            }))
            .child(page.label())
    }

    fn render_content(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let current_mode = cx
            .try_global::<AppearanceMode>()
            .copied()
            .unwrap_or_default();
        let theme_settings = cx
            .try_global::<ThemeSettings>()
            .cloned()
            .unwrap_or_default();

        div()
            .id("settings-content")
            .flex()
            .flex_col()
            .flex_1()
            .h_full()
            .min_w_0()
            .overflow_hidden() // Prevent any content from escaping
            .bg(theme.elevated_surface)
            .pt(sizes::PADDING_LG)
            .overflow_y_scroll()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .max_w(sizes::SETTINGS_CONTENT_MAX_WIDTH)
                    .px(sizes::PADDING_3XL)
                    .pb(sizes::PADDING_3XL)
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .mb(sizes::PADDING_2XL)
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
        let mode_label = match current_mode {
            AppearanceMode::System => "System",
            AppearanceMode::Light => "Light",
            AppearanceMode::Dark => "Dark",
        };

        let registry = theme_registry();
        let light_themes: Vec<String> = registry
            .light_themes()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let dark_themes: Vec<String> = registry
            .dark_themes()
            .iter()
            .map(|s| s.to_string())
            .collect();

        div()
            .flex()
            .flex_col()
            // Appearance mode
            .child(self.render_setting_row(
                "Appearance",
                "Choose whether to use light or dark theme, or follow system settings.",
                self.render_dropdown(
                    "settings.appearance",
                    "Appearance Mode",
                    mode_label,
                    self.open_dropdown == OpenDropdown::AppearanceMode,
                    vec![
                        "System".to_string(),
                        "Light".to_string(),
                        "Dark".to_string(),
                    ],
                    mode_label,
                    OpenDropdown::AppearanceMode,
                    |label, cx| {
                        let mode = match label {
                            "Light" => AppearanceMode::Light,
                            "Dark" => AppearanceMode::Dark,
                            _ => AppearanceMode::System,
                        };
                        cx.set_global(mode);
                        #[cfg(target_os = "macos")]
                        crate::macos::set_app_appearance(mode);
                        cx.refresh_windows();
                    },
                    theme,
                    cx,
                ),
                theme,
            ))
            // Light theme
            .child(self.render_setting_row(
                "Light Theme",
                "Theme used when in light mode.",
                self.render_dropdown(
                    "settings.light_theme",
                    "Light Theme",
                    &theme_settings.light_theme,
                    self.open_dropdown == OpenDropdown::LightTheme,
                    light_themes,
                    &theme_settings.light_theme,
                    OpenDropdown::LightTheme,
                    |name, cx| {
                        let mut settings = cx
                            .try_global::<ThemeSettings>()
                            .cloned()
                            .unwrap_or_default();
                        settings.light_theme = name.to_string();
                        cx.set_global(settings);
                        cx.refresh_windows();
                    },
                    theme,
                    cx,
                ),
                theme,
            ))
            // Dark theme
            .child(self.render_setting_row(
                "Dark Theme",
                "Theme used when in dark mode.",
                self.render_dropdown(
                    "settings.dark_theme",
                    "Dark Theme",
                    &theme_settings.dark_theme,
                    self.open_dropdown == OpenDropdown::DarkTheme,
                    dark_themes,
                    &theme_settings.dark_theme,
                    OpenDropdown::DarkTheme,
                    |name, cx| {
                        let mut settings = cx
                            .try_global::<ThemeSettings>()
                            .cloned()
                            .unwrap_or_default();
                        settings.dark_theme = name.to_string();
                        cx.set_global(settings);
                        cx.refresh_windows();
                    },
                    theme,
                    cx,
                ),
                theme,
            ))
    }

    fn render_behavior_content(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let prefer_tabs = self.tab_preferences.prefer_tabs;
        let show_tab_bar = self.tab_preferences.show_tab_bar;
        let allow_merge = self.tab_preferences.allow_merge;

        div()
            .flex()
            .flex_col()
            // Open PDFs in tabs
            .child(self.render_setting_row(
                "Open PDFs in tabs",
                "New PDFs open as tabs in the current window instead of new windows.",
                toggle_switch(
                    "prefer-tabs",
                    prefer_tabs,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_prefer_tabs(cx);
                    }),
                ),
                theme,
            ))
            // Show tab bar
            .child(self.render_setting_row(
                "Show tab bar",
                "Always show the tab bar, even when only one document is open.",
                toggle_switch(
                    "show-tab-bar",
                    show_tab_bar,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_show_tab_bar(cx);
                    }),
                ),
                theme,
            ))
            // Allow window merging
            .child(self.render_setting_row(
                "Allow window merging",
                "Drag tabs between windows to merge them together.",
                toggle_switch(
                    "allow-merge",
                    allow_merge,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_allow_merge(cx);
                    }),
                ),
                theme,
            ))
    }

    fn render_setting_row(
        &self,
        title: &'static str,
        description: &'static str,
        control: impl IntoElement,
        theme: &Theme,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .w_full()
            .items_center()
            .gap(sizes::GAP_LG)
            .py(sizes::PADDING_XL)
            .border_b_1()
            .border_color(theme.border)
            // Label column - takes remaining space
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0() // Allow text to shrink/wrap
                    .gap(sizes::GAP_SM)
                    .child(div().text_sm().child(title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child(description),
                    ),
            )
            // Control column - fixed width, right-aligned
            .child(
                div()
                    .flex_shrink_0()
                    .w(sizes::DROPDOWN_WIDTH)
                    .child(control),
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
        selected: &str,
        dropdown_id: OpenDropdown,
        on_select: F,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        F: Fn(&str, &mut App) + Clone + 'static,
    {
        let surface = theme.surface;
        let border = theme.border;
        let hover = theme.element_hover;
        let text_muted = theme.text_muted;
        let accent = theme.accent;
        let selected = selected.to_string();
        let current = current_value.to_string();
        let max_height = px(240.0);

        // Silence unused warnings - element_id/element_name reserved for future dev mode
        let _ = (element_id, element_name);

        // The button itself - dropdown menu is a child so it anchors correctly
        div()
            .relative() // Enable absolute positioning for children
            .w_full()
            .h(sizes::CONTROL_HEIGHT)
            // Button visual
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .pl(sizes::PADDING_LG)
            .pr(px(10.0))
            .bg(surface)
            .border_1()
            .border_color(border)
            .rounded(sizes::RADIUS_SM)
            .cursor_pointer()
            .hover(|s| s.bg(hover))
            .id(SharedString::from(format!("dropdown-{:?}", dropdown_id)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.toggle_dropdown(dropdown_id, cx);
            }))
            .child(
                div()
                    .text_sm()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(current.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(text_muted)
                    .ml(sizes::GAP_SM)
                    .child("▼"),
            )
            // Dropdown menu - use absolute positioning below button
            .when(is_open, |d| {
                d.child(
                    div()
                        .absolute()
                        .left_0()
                        .top(sizes::CONTROL_HEIGHT + px(4.0)) // Position below button with 4px gap
                        .child(
                            deferred(
                                div()
                                    .occlude()
                                    .min_w(sizes::DROPDOWN_WIDTH)
                                    .max_h(max_height)
                                    .overflow_hidden()
                                    .bg(surface)
                                    .border_1()
                                    .border_color(border)
                                    .rounded(sizes::RADIUS_MD)
                                    .shadow_lg()
                                    .py(sizes::PADDING_SM)
                                    .children(options.iter().map(|label| {
                                        let is_selected = label == &selected;
                                        let on_select = on_select.clone();
                                        let label_owned = label.clone();

                                        div()
                                            .id(SharedString::from(format!("opt-{}", label)))
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .justify_between()
                                            .h(sizes::CONTROL_HEIGHT)
                                            .px(sizes::PADDING_LG)
                                            .mx(sizes::PADDING_SM)
                                            .rounded(sizes::RADIUS_SM)
                                            .cursor_pointer()
                                            .text_sm()
                                            .hover(|s| s.bg(hover))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                on_select(&label_owned, cx);
                                                this.close_dropdown(cx);
                                            }))
                                            .child(label.clone())
                                            .when(is_selected, |d| {
                                                d.child(
                                                    div().text_sm().text_color(accent).child("✓"),
                                                )
                                            })
                                    })),
                            )
                            .with_priority(1),
                        ),
                )
            })
    }
}
