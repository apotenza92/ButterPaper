//! ButterPaper main component with tabbed document management.

use gpui::{div, prelude::*, px, App, Context, ExternalPaths, FocusHandle, Focusable, MouseButton, ScrollDelta, ScrollWheelEvent, Window};
use std::path::PathBuf;

use super::document::DocumentTab;
use crate::components::tab_bar::TabId as UiTabId;
use crate::components::{icon, icon_button, text_button_with_shortcut, Icon, IconButtonSize, TextButtonSize};
use crate::sidebar::ThumbnailSidebar;
use crate::viewport::PdfViewport;
use crate::workspace::{load_preferences, TabPreferences};
use crate::{current_theme, ui, Theme};
use crate::{
    CloseTab, CloseWindow, NextPage, NextTab, Open, PrevPage, PrevTab, ZoomIn, ZoomOut,
};

pub struct PdfEditor {
    tabs: Vec<DocumentTab>,
    active_tab_index: usize,
    focus_handle: FocusHandle,
    preferences: TabPreferences,
    /// Horizontal scroll offset for the tab bar (in pixels)
    tab_scroll_offset: f32,
}

impl PdfEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Start with no tabs - shows the full welcome screen
        Self {
            tabs: Vec::new(),
            active_tab_index: 0,
            focus_handle: cx.focus_handle(),
            preferences: load_preferences(),
            tab_scroll_offset: 0.0,
        }
    }

    /// Create a new document tab, optionally with a file path.
    /// If path is None, creates a welcome tab.
    fn create_tab(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) -> usize {
        let viewport = cx.new(PdfViewport::new);
        let sidebar = cx.new(ThumbnailSidebar::new);

        // Set up page change callback from viewport to sidebar
        // Use WeakEntity to avoid dangling references if tab is closed
        let sidebar_weak = sidebar.downgrade();
        viewport.update(cx, |vp, _cx| {
            vp.set_on_page_change(move |page, cx| {
                if let Some(sidebar_handle) = sidebar_weak.upgrade() {
                    sidebar_handle.update(cx, |sb, cx| {
                        sb.set_selected_page(page, cx);
                    });
                }
            });
        });

        // Set up page select callback from sidebar to viewport
        // Use WeakEntity to avoid dangling references if tab is closed
        let viewport_weak = viewport.downgrade();
        sidebar.update(cx, |sb, _cx| {
            sb.set_on_page_select(move |page, cx| {
                if let Some(viewport_handle) = viewport_weak.upgrade() {
                    viewport_handle.update(cx, |vp, cx| {
                        vp.go_to_page(page, cx);
                    });
                }
            });
        });

        let title = path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Welcome".to_string());

        let tab = DocumentTab {
            id: UiTabId::new(),
            path,
            title,
            viewport,
            sidebar,
            is_dirty: false,
        };

        self.tabs.push(tab);
        self.tabs.len() - 1
    }

    /// Create a new welcome tab (no file loaded).
    fn create_welcome_tab(&mut self, cx: &mut Context<Self>) -> usize {
        self.create_tab(None, cx)
    }

    /// Create a new welcome tab and make it active.
    fn new_tab(&mut self, cx: &mut Context<Self>) {
        let idx = self.create_welcome_tab(cx);
        self.active_tab_index = idx;
        cx.notify();
    }

    pub fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // Check if file is already open in a tab
        if let Some(idx) = self.tabs.iter().position(|t| t.path.as_ref() == Some(&path)) {
            self.active_tab_index = idx;
            cx.notify();
            return;
        }

        // If current tab is a welcome tab, reuse it instead of creating a new one
        let tab_index = if self.active_tab().map(|t| t.is_welcome()).unwrap_or(false) {
            // Update the existing welcome tab with the file
            let tab = &mut self.tabs[self.active_tab_index];
            tab.path = Some(path.clone());
            tab.title = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Untitled".to_string());
            self.active_tab_index
        } else {
            // Create new tab
            let idx = self.create_tab(Some(path.clone()), cx);
            self.active_tab_index = idx;
            idx
        };

        // Load the PDF in the tab
        let tab = &self.tabs[tab_index];
        tab.viewport.update(cx, |viewport, cx| {
            if let Err(e) = viewport.load_pdf(path, cx) {
                eprintln!("Error loading PDF: {}", e);
            }
        });

        // Share document with sidebar
        let doc = tab.viewport.read(cx).document();
        tab.sidebar.update(cx, |sidebar, cx| {
            sidebar.set_document(doc, cx);
        });

        cx.notify();
    }

    /// Handle dropped files by opening them as new tabs.
    fn handle_file_drop(&mut self, paths: &ExternalPaths, cx: &mut Context<Self>) {
        for path in paths.paths() {
            // Only handle PDF files (case-insensitive)
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "pdf" {
                    self.open_file(path.clone(), cx);
                }
            }
        }
    }

    /// Handle scroll wheel on tab bar - converts vertical scroll to horizontal.
    fn handle_tab_scroll(&mut self, delta: ScrollDelta, cx: &mut Context<Self>) {
        let scroll_amount: f32 = match delta {
            ScrollDelta::Lines(lines) => {
                // Both vertical and horizontal scroll - use whichever is larger
                // Vertical scroll (lines.y) maps to horizontal movement
                let horizontal = lines.x * 30.0;
                let vertical_as_horizontal = lines.y * 30.0;
                if horizontal.abs() > vertical_as_horizontal.abs() {
                    horizontal
                } else {
                    vertical_as_horizontal
                }
            }
            ScrollDelta::Pixels(pixels) => {
                // Use horizontal if present, otherwise use vertical
                let px_x: f32 = pixels.x.into();
                let px_y: f32 = pixels.y.into();
                if px_x.abs() > px_y.abs() {
                    px_x
                } else {
                    px_y
                }
            }
        };

        self.tab_scroll_offset = (self.tab_scroll_offset - scroll_amount).max(0.0);
        cx.notify();
    }

    fn active_tab(&self) -> Option<&DocumentTab> {
        self.tabs.get(self.active_tab_index)
    }

    fn select_tab(&mut self, tab_id: UiTabId, cx: &mut Context<Self>) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == tab_id) {
            self.active_tab_index = idx;
            cx.notify();
        }
    }

    fn close_tab(&mut self, tab_id: UiTabId, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == tab_id) {
            self.tabs.remove(idx);

            // If no tabs left, show welcome screen (empty tabs vec)
            if self.tabs.is_empty() {
                self.active_tab_index = 0;
                cx.notify();
                return;
            }

            if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len() - 1;
            }

            cx.notify();
        }
    }

    fn next_tab(&mut self, cx: &mut Context<Self>) {
        if !self.tabs.is_empty() {
            self.active_tab_index = (self.active_tab_index + 1) % self.tabs.len();
            cx.notify();
        }
    }

    fn prev_tab(&mut self, cx: &mut Context<Self>) {
        if !self.tabs.is_empty() {
            if self.active_tab_index == 0 {
                self.active_tab_index = self.tabs.len() - 1;
            } else {
                self.active_tab_index -= 1;
            }
            cx.notify();
        }
    }

    fn show_tab_bar(&self) -> bool {
        // Show tab bar if preference is set or if there are multiple tabs
        self.preferences.show_tab_bar || self.tabs.len() > 1
    }

    fn handle_zoom_in(&mut self, _: &ZoomIn, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.zoom_in(cx);
            });
        }
    }

    fn handle_zoom_out(&mut self, _: &ZoomOut, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.zoom_out(cx);
            });
        }
    }

    fn handle_next_page(&mut self, _: &NextPage, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.next_page(cx);
            });
        }
    }

    fn handle_prev_page(&mut self, _: &PrevPage, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.prev_page(cx);
            });
        }
    }

    fn handle_open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        let future = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
        });

        cx.spawn_in(
            window,
            async |this: gpui::WeakEntity<PdfEditor>, cx: &mut gpui::AsyncWindowContext| {
                if let Ok(Ok(Some(paths))) = future.await {
                    if let Some(path) = paths.into_iter().next() {
                        let _ = cx.update(|_window, cx| {
                            this.update(cx, |editor: &mut PdfEditor, cx| {
                                editor.open_file(path, cx);
                            })
                            .ok()
                        });
                    }
                }
            },
        )
        .detach();
    }

    fn handle_close_window(
        &mut self,
        _: &CloseWindow,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        window.remove_window();
    }

    fn handle_next_tab(&mut self, _: &NextTab, _window: &mut Window, cx: &mut Context<Self>) {
        self.next_tab(cx);
    }

    fn handle_prev_tab(&mut self, _: &PrevTab, _window: &mut Window, cx: &mut Context<Self>) {
        self.prev_tab(cx);
    }

    fn handle_close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let tab_id = tab.id;
            self.close_tab(tab_id, window, cx);
        }
    }

    /// Render individual tab items (for use in the tab bar header).
    /// Simple styling - no borders, just background color for active state.
    /// Close button only visible on active tab or hover.
    fn render_tab_items(
        &self,
        theme: &Theme,
        cx: &Context<Self>,
    ) -> Vec<impl IntoElement + use<'_>> {
        let entity = cx.entity().downgrade();

        self.tabs
            .iter()
            .enumerate()
            .map(|(idx, doc_tab)| {
                let is_active = idx == self.active_tab_index;
                let title = doc_tab.title.clone();
                let is_dirty = doc_tab.is_dirty;
                let tab_id = doc_tab.id;

                let entity_for_select = entity.clone();
                let entity_for_close = entity.clone();

                let bg = if is_active { theme.elevated_surface } else { theme.surface };
                let text_color = if is_active { theme.text } else { theme.text_muted };
                let hover_bg = theme.element_hover;
                let hover_text = theme.text;

                div()
                    .id(gpui::SharedString::from(format!("tab-{}", tab_id)))
                    .group("tab")
                    .h_full()
                    .px(ui::sizes::SPACE_3)
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .cursor_pointer()
                    .text_sm()
                    .bg(bg)
                    .text_color(text_color)
                    // Hover effect for inactive tabs
                    .when(!is_active, move |d| {
                        d.hover(move |s| s.text_color(hover_text).bg(hover_bg))
                    })
                    .on_click(move |_, _, cx| {
                        if let Some(editor) = entity_for_select.upgrade() {
                            editor.update(cx, |editor, cx| {
                                editor.select_tab(tab_id, cx);
                            });
                        }
                    })
                    // Tab title
                    .child(div().whitespace_nowrap().child(title))
                    // Dirty indicator
                    .when(is_dirty, {
                        let text_muted = theme.text_muted;
                        move |d| d.child(icon(Icon::Dirty, 12.0, text_muted))
                    })
                    // Close button - visible only on active tab or hover
                    .child(
                        div()
                            .when(!is_active, |d| d.invisible().group_hover("tab", |s| s.visible()))
                            .child(icon_button(
                                format!("tab-close-{}", tab_id),
                                Icon::Close,
                                IconButtonSize::Sm,
                                theme,
                                move |_, window, cx| {
                                    if let Some(editor) = entity_for_close.upgrade() {
                                        editor.update(cx, |editor, cx| {
                                            editor.close_tab(tab_id, window, cx);
                                        });
                                    }
                                },
                            )),
                    )
            })
            .collect()
    }

}

impl Focusable for PdfEditor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PdfEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = current_theme(window, cx);

        let show_tab_bar = self.show_tab_bar();

        // Layout: Titlebar (32px) -> Content (sidebar | main)
        // CRITICAL: Titlebar MUST remain empty except for traffic lights
        div()
            .id("butterpaper")
            .key_context("PdfEditor")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_zoom_in))
            .on_action(cx.listener(Self::handle_zoom_out))
            .on_action(cx.listener(Self::handle_next_page))
            .on_action(cx.listener(Self::handle_prev_page))
            .on_action(cx.listener(Self::handle_open))
            .on_action(cx.listener(Self::handle_close_window))
            .on_action(cx.listener(Self::handle_next_tab))
            .on_action(cx.listener(Self::handle_prev_tab))
            .on_action(cx.listener(Self::handle_close_tab))
            // Handle dropped files (PDFs only)
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _window, cx| {
                this.handle_file_drop(paths, cx);
            }))
            .flex()
            .flex_col() // Vertical layout: titlebar -> content
            .bg(theme.surface)
            .text_color(theme.text)
            .size_full()
            // Title bar (32px) - EMPTY, only traffic lights
            .child(ui::title_bar("ButterPaper", theme.text, theme.border))
            // Main content below titlebar
            // When no tabs: full welcome screen
            // When tabs: sidebar | right column
            .when(self.tabs.is_empty(), {
                // Full welcome screen - no tabs, no sidebar, just welcome content
                let entity = cx.entity().downgrade();
                let theme_clone = theme.clone();
                move |d| {
                    d.child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .bg(theme.elevated_surface)
                            .child(text_button_with_shortcut(
                                "welcome-open-file",
                                "Open File",
                                "⌘O",
                                TextButtonSize::Md,
                                &theme_clone,
                                move |_, window, cx| {
                                    if let Some(editor) = entity.upgrade() {
                                        editor.update(cx, |editor, cx| {
                                            editor.handle_open(&Open, window, cx);
                                        });
                                    }
                                },
                            )),
                    )
                }
            })
            .when(!self.tabs.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_1()
                        .overflow_hidden()
                        // Left: Sidebar (always show when tabs exist)
                        .when_some(self.active_tab(), |d, tab| d.child(tab.sidebar.clone()))
                        // Right: Content column (tab bar + viewport + status bar)
                        // Left border separates from sidebar (always present when tabs exist)
                        .child({
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .overflow_hidden()
                                .border_l_1()
                                .border_color(theme.border)
                                // Tab bar - simple layout with single bottom border
                                .child(
                                    div()
                                        .h(ui::sizes::TAB_BAR_HEIGHT)
                                        .w_full()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .bg(theme.surface)
                                        .border_b_1()
                                        .border_color(theme.border)
                                        // Scrollable tabs container
                                        .when(show_tab_bar, |d| {
                                            let scroll_offset = self.tab_scroll_offset;
                                            let tab_items = self.render_tab_items(&theme, cx);
                                            d.child(
                                                div()
                                                    .id("tabs-scroll")
                                                    .flex()
                                                    .flex_row()
                                                    .h_full()
                                                    .min_w_0()
                                                    .overflow_hidden()
                                                    .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _window, cx| {
                                                        this.handle_tab_scroll(event.delta, cx);
                                                    }))
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .flex_row()
                                                            .h_full()
                                                            .ml(px(-scroll_offset))
                                                            .children(tab_items),
                                                    ),
                                            )
                                        })
                                        .when(!show_tab_bar, {
                                            // Show document title when single tab
                                            let title = self
                                                .active_tab()
                                                .map(|t| t.title.clone())
                                                .unwrap_or_default();
                                            move |d| {
                                                d.child(
                                                    div()
                                                        .h_full()
                                                        .flex()
                                                        .items_center()
                                                        .text_sm()
                                                        .text_color(theme.text)
                                                        .child(title),
                                                )
                                            }
                                        })
                                        // Empty space - fills remaining width, double-click to create new tab
                                        .child({
                                            let entity = cx.entity().downgrade();
                                            div()
                                                .id("tab-bar-empty")
                                                .flex_1()
                                                .h_full()
                                                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                                    if event.click_count == 2 {
                                                        if let Some(editor) = entity.upgrade() {
                                                            editor.update(cx, |editor, cx| {
                                                                editor.new_tab(cx);
                                                            });
                                                        }
                                                    }
                                                })
                                        })
                                        // "+" button on far right
                                        .when(show_tab_bar, |d| {
                                            let entity = cx.entity().downgrade();
                                            d.child(
                                                div()
                                                    .flex_shrink_0()
                                                    .h_full()
                                                    .flex()
                                                    .items_center()
                                                    .px(ui::sizes::SPACE_1)
                                                    .child(icon_button(
                                                        "new-tab",
                                                        Icon::Plus,
                                                        IconButtonSize::Md,
                                                        &theme,
                                                        move |_, _, cx| {
                                                            if let Some(editor) = entity.upgrade() {
                                                                editor.update(cx, |editor, cx| {
                                                                    editor.new_tab(cx);
                                                                });
                                                            }
                                                        },
                                                    )),
                                            )
                                        }),
                                )
                                // Viewport / Welcome tab content
                                .child({
                                    let is_welcome = self.active_tab().map(|t| t.is_welcome()).unwrap_or(false);
                                    let viewport = self.active_tab().map(|t| t.viewport.clone());
                                    let entity = cx.entity().downgrade();
                                    let theme_clone = theme.clone();

                                    div()
                                        .flex_1()
                                        .overflow_hidden()
                                        .bg(theme.elevated_surface)
                                        // Show welcome content for welcome tabs
                                        .when(is_welcome, |d| {
                                            d.child(
                                                div()
                                                    .size_full()
                                                    .flex()
                                                    .flex_col()
                                                    .items_center()
                                                    .justify_center()
                                                    .child(text_button_with_shortcut(
                                                        "tab-welcome-open-file",
                                                        "Open File",
                                                        "⌘O",
                                                        TextButtonSize::Md,
                                                        &theme_clone,
                                                        move |_, window, cx| {
                                                            if let Some(editor) = entity.upgrade() {
                                                                editor.update(cx, |editor, cx| {
                                                                    editor.handle_open(&Open, window, cx);
                                                                });
                                                            }
                                                        },
                                                    )),
                                            )
                                        })
                                        // Show viewport for document tabs
                                        .when_some(viewport.filter(|_| !is_welcome), |d, vp| {
                                            d.child(vp)
                                        })
                                })
                        })
                )
            })
    }
}
