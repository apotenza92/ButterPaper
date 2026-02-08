//! Navigation item component for sidebars.

use gpui::{div, prelude::*, ClickEvent, Rgba, SharedString, Window};

use crate::components::ButtonSize;
use crate::ui::{sizes, TypographyExt};
use crate::Theme;

fn nav_item_text_colors(selected: bool, theme: &Theme) -> (Rgba, Rgba) {
    if selected {
        (theme.text, theme.text)
    } else {
        (theme.text_muted, theme.text)
    }
}

/// Navigation item for sidebar menus.
///
/// # Example
/// ```ignore
/// nav_item("Settings", true, theme, cx.listener(|this, _, _, cx| {
///     this.navigate_to_settings(cx);
/// }))
/// ```
pub fn nav_item<F>(
    label: impl Into<SharedString>,
    selected: bool,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let label: SharedString = label.into();
    let element_selected = theme.element_selected;
    let element_hover = theme.element_hover;
    let (text, hover_text) = nav_item_text_colors(selected, theme);

    div()
        .id(SharedString::from(format!("nav-{}", label.clone())))
        .h(ButtonSize::Medium.height_px())
        .px(sizes::PADDING_MD)
        .flex()
        .items_center()
        .rounded(sizes::RADIUS_SM)
        .cursor_pointer()
        .text_ui_body()
        .text_color(text)
        .when(selected, move |d| d.bg(element_selected))
        .when(!selected, move |d| d.hover(move |s| s.bg(element_hover).text_color(hover_text)))
        .on_click(on_click)
        .child(label)
}

/// Navigation item with icon.
pub fn nav_item_with_icon<F>(
    icon: impl Into<SharedString>,
    label: impl Into<SharedString>,
    selected: bool,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let label: SharedString = label.into();
    let element_selected = theme.element_selected;
    let element_hover = theme.element_hover;
    let (text, hover_text) = nav_item_text_colors(selected, theme);

    div()
        .id(SharedString::from(format!("nav-{}", label.clone())))
        .h(ButtonSize::Medium.height_px())
        .px(sizes::PADDING_MD)
        .flex()
        .items_center()
        .gap(sizes::GAP_MD)
        .rounded(sizes::RADIUS_SM)
        .cursor_pointer()
        .text_ui_body()
        .text_color(text)
        .when(selected, move |d| d.bg(element_selected))
        .when(!selected, move |d| d.hover(move |s| s.bg(element_hover).text_color(hover_text)))
        .on_click(on_click)
        .child(div().text_ui_icon().child(icon.into()))
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::nav_item_text_colors;
    use crate::theme::ThemeColors;

    #[test]
    fn selected_nav_item_keeps_primary_text_color() {
        let theme = ThemeColors::fallback_dark();
        let (text, hover_text) = nav_item_text_colors(true, &theme);
        assert_eq!(text, theme.text);
        assert_eq!(hover_text, theme.text);
    }

    #[test]
    fn unselected_nav_item_promotes_muted_text_on_hover() {
        let theme = ThemeColors::fallback_light();
        let (text, hover_text) = nav_item_text_colors(false, &theme);
        assert_eq!(text, theme.text_muted);
        assert_eq!(hover_text, theme.text);
    }
}
