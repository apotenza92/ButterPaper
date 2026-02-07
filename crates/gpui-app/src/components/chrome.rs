//! Shared editor chrome primitives for compact toolbar surfaces.

use gpui::{div, prelude::*, ClickEvent, InteractiveElement, Pixels, SharedString, Window};

use crate::components::button_like::disabled_text;
use crate::components::{icon, tooltip_builder, Icon};
use crate::ui::color;
use crate::ui::sizes;
use crate::Theme;

/// Shared size token for toolbar controls (buttons, combo shells, dropdown triggers).
pub fn chrome_control_size() -> Pixels {
    sizes::TOOLBAR_CONTROL_SIZE
}

/// Shared shell used by toolbar controls so all chrome has identical outer metrics.
pub fn chrome_control_shell(
    id: impl Into<SharedString>,
    enabled: bool,
    selected: bool,
    min_width: Option<Pixels>,
    theme: &Theme,
    child: impl IntoElement,
) -> impl IntoElement {
    let control_size = chrome_control_size();
    let base_background = if selected {
        color::with_alpha(theme.element_selected, 0.88)
    } else {
        theme.elevated_surface
    };
    let border = color::subtle_border(theme.border);
    let hover = if selected { base_background } else { theme.element_hover };
    let active = theme.element_selected;

    div()
        .id(id.into())
        .h(control_size)
        .w(control_size)
        .when_some(min_width, |d, width| d.min_w(width))
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .bg(base_background)
        .border_1()
        .border_color(border)
        .rounded(sizes::RADIUS_SM)
        .when(enabled, move |d| {
            d.cursor_pointer()
                .hover({
                    let hover = hover;
                    move |s| s.bg(hover)
                })
                .active({
                    let active = active;
                    move |s| s.bg(active)
                })
        })
        .child(child)
}

/// Compact icon button used by editor chrome surfaces (tool rail and canvas toolbar).
pub fn chrome_icon_button<F>(
    id: impl Into<SharedString>,
    icon_type: Icon,
    tooltip: impl Into<SharedString>,
    enabled: bool,
    selected: bool,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let tooltip = tooltip.into();
    let icon_size = 16.0;
    let text_color = if enabled {
        if selected {
            theme.text
        } else {
            theme.text_muted
        }
    } else {
        disabled_text(theme)
    };

    let button_size = chrome_control_size();
    let base_background = if selected {
        color::with_alpha(theme.element_selected, 0.88)
    } else {
        theme.elevated_surface
    };
    let border = color::subtle_border(theme.border);
    let hover = if selected { base_background } else { theme.element_hover };
    let active = theme.element_selected;

    div()
        .id(id.into())
        .w(button_size)
        .h(button_size)
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .bg(base_background)
        .border_1()
        .border_color(border)
        .rounded(sizes::RADIUS_SM)
        .tooltip(tooltip_builder(tooltip, theme.surface, theme.border, theme.text))
        .when(enabled, move |d| {
            d.cursor_pointer()
                .hover({
                    let hover = hover;
                    move |s| s.bg(hover)
                })
                .active({
                    let active = active;
                    move |s| s.bg(active)
                })
                .on_click(on_click)
        })
        .child(icon(icon_type, icon_size, text_color))
}

#[cfg(test)]
mod tests {
    use super::chrome_control_size;
    use crate::ui::sizes;

    #[test]
    fn chrome_control_size_matches_toolbar_contract() {
        let shell: f32 = chrome_control_size().into();
        let toolbar: f32 = sizes::TOOLBAR_CONTROL_SIZE.into();
        assert_eq!(shell, toolbar);
    }
}
