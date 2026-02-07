#![allow(dead_code)]

use gpui::{px, App, Global, Pixels, Rems};
use serde::{Deserialize, Serialize};

use crate::styles::units::rems_from_px;

/// UI density drives spacing expansion/compression across the app.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiDensity {
    Compact,
    #[default]
    Default,
    Comfortable,
}

impl Global for UiDensity {}

/// Resolve the current UI density.
pub fn ui_density(cx: &App) -> UiDensity {
    cx.try_global::<UiDensity>().copied().unwrap_or_default()
}

/// Spacing tokens that scale with UI density.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DynamicSpacing {
    Base00,
    Base01,
    Base02,
    Base03,
    Base04,
    Base06,
    Base08,
    Base10,
    Base12,
    Base16,
    Base24,
    Base32,
    Base40,
    Base48,
}

impl DynamicSpacing {
    fn density_triplet(self) -> (f32, f32, f32) {
        match self {
            DynamicSpacing::Base00 => (0.0, 0.0, 0.0),
            DynamicSpacing::Base01 => (1.0, 1.0, 2.0),
            DynamicSpacing::Base02 => (1.0, 2.0, 4.0),
            DynamicSpacing::Base03 => (2.0, 3.0, 4.0),
            DynamicSpacing::Base04 => (2.0, 4.0, 6.0),
            DynamicSpacing::Base06 => (3.0, 6.0, 8.0),
            DynamicSpacing::Base08 => (4.0, 8.0, 10.0),
            DynamicSpacing::Base10 => (10.0, 12.0, 14.0),
            DynamicSpacing::Base12 => (14.0, 16.0, 18.0),
            DynamicSpacing::Base16 => (18.0, 20.0, 22.0),
            DynamicSpacing::Base24 => (20.0, 24.0, 28.0),
            DynamicSpacing::Base32 => (28.0, 32.0, 36.0),
            DynamicSpacing::Base40 => (36.0, 40.0, 44.0),
            DynamicSpacing::Base48 => (44.0, 48.0, 52.0),
        }
    }

    fn value_for_density(self, density: UiDensity) -> f32 {
        let (compact, default, comfortable) = self.density_triplet();
        match density {
            UiDensity::Compact => compact,
            UiDensity::Default => default,
            UiDensity::Comfortable => comfortable,
        }
    }

    pub fn px(self, cx: &App) -> Pixels {
        px(self.value_for_density(ui_density(cx)))
    }

    pub fn rems(self, cx: &App) -> Rems {
        let value: f32 = self.px(cx).into();
        rems_from_px(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{DynamicSpacing, UiDensity};

    #[test]
    fn spacing_default_density_uses_middle_value() {
        assert_eq!(DynamicSpacing::Base24.value_for_density(UiDensity::Default), 24.0);
    }

    #[test]
    fn spacing_compact_and_comfortable_shift_values() {
        let compact = DynamicSpacing::Base32.value_for_density(UiDensity::Compact);
        assert_eq!(compact, 28.0);

        let comfortable = DynamicSpacing::Base32.value_for_density(UiDensity::Comfortable);
        assert_eq!(comfortable, 36.0);
    }
}
