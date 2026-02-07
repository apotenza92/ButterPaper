//! Step slider primitive (button-adjusted) for consistent numeric controls.

use gpui::{div, prelude::*, px, ClickEvent, SharedString, Window};

use crate::components::button_like::subtle_border;
use crate::components::{icon, Icon};
use crate::ui::sizes;
use crate::Theme;

fn clamp_step(value: f32, min: f32, max: f32, step: f32) -> f32 {
    let clamped = value.clamp(min, max);
    if step <= 0.0 {
        return clamped;
    }
    let snapped = ((clamped - min) / step).round() * step + min;
    snapped.clamp(min, max)
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::clamp_step;

    #[test]
    fn clamps_to_min_and_max() {
        assert_eq!(clamp_step(-10.0, 0.0, 100.0, 5.0), 0.0);
        assert_eq!(clamp_step(120.0, 0.0, 100.0, 5.0), 100.0);
    }

    #[test]
    fn snaps_to_nearest_step() {
        assert_eq!(clamp_step(12.2, 0.0, 100.0, 5.0), 10.0);
        assert_eq!(clamp_step(12.8, 0.0, 100.0, 5.0), 15.0);
    }

    #[test]
    fn non_positive_step_disables_snapping() {
        assert_eq!(clamp_step(12.8, 0.0, 100.0, 0.0), 12.8);
        assert_eq!(clamp_step(12.8, 0.0, 100.0, -5.0), 12.8);
    }
}

pub fn slider<F>(
    id: impl Into<SharedString>,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    theme: &Theme,
    on_change: F,
) -> impl IntoElement
where
    F: Fn(f32, &mut gpui::App) + Clone + 'static,
{
    let id = id.into();
    let dec_id: SharedString = format!("{id}-dec").into();
    let inc_id: SharedString = format!("{id}-inc").into();
    let value = clamp_step(value, min, max, step);
    let ratio = if max > min { ((value - min) / (max - min)).clamp(0.0, 1.0) } else { 0.0 };
    let minus_enabled = value > min;
    let plus_enabled = value < max;
    let border = subtle_border(theme);

    let dec_value = clamp_step(value - step, min, max, step);
    let inc_value = clamp_step(value + step, min, max, step);

    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .gap(sizes::SPACE_2)
        .child(
            div()
                .id(dec_id)
                .w(sizes::SLIDER_BUTTON_SIZE)
                .h(sizes::SLIDER_BUTTON_SIZE)
                .flex()
                .items_center()
                .justify_center()
                .rounded(sizes::RADIUS_SM)
                .bg(theme.elevated_surface)
                .border_1()
                .border_color(border)
                .text_color(if minus_enabled { theme.text } else { theme.text_muted })
                .when(minus_enabled, |d| {
                    d.cursor_pointer().hover({
                        let hover = theme.element_hover;
                        move |s| s.bg(hover)
                    })
                })
                .on_click({
                    let on_change = on_change.clone();
                    move |_: &ClickEvent, _: &mut Window, cx: &mut gpui::App| {
                        if minus_enabled {
                            on_change(dec_value, cx);
                        }
                    }
                })
                .child(icon(Icon::Minus, sizes::SLIDER_ICON_SIZE, theme.text)),
        )
        .child(
            div()
                .w(sizes::SLIDER_TRACK_WIDTH)
                .h(sizes::SLIDER_TRACK_HEIGHT)
                .rounded_full()
                .bg(theme.surface)
                .border_1()
                .border_color(border)
                .overflow_hidden()
                .child(div().h_full().w(sizes::SLIDER_TRACK_WIDTH * ratio).bg(theme.accent)),
        )
        .child(
            div()
                .id(inc_id)
                .w(sizes::SLIDER_BUTTON_SIZE)
                .h(sizes::SLIDER_BUTTON_SIZE)
                .flex()
                .items_center()
                .justify_center()
                .rounded(sizes::RADIUS_SM)
                .bg(theme.elevated_surface)
                .border_1()
                .border_color(border)
                .text_color(if plus_enabled { theme.text } else { theme.text_muted })
                .when(plus_enabled, |d| {
                    d.cursor_pointer().hover({
                        let hover = theme.element_hover;
                        move |s| s.bg(hover)
                    })
                })
                .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut gpui::App| {
                    if plus_enabled {
                        on_change(inc_value, cx);
                    }
                })
                .child(icon(Icon::Plus, sizes::SLIDER_ICON_SIZE, theme.text)),
        )
        .child(
            div()
                .min_w(sizes::SLIDER_VALUE_MIN_WIDTH)
                .text_sm()
                .text_color(theme.text_muted)
                .child(format!("{value:.0}")),
        )
}
