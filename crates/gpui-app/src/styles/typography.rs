#![allow(dead_code)]

use gpui::{px, Pixels, Rems};

use crate::styles::units::rems_from_px;

/// Shared text sizes for UI surfaces.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextSize {
    Large,
    #[default]
    Default,
    Small,
    XSmall,
}

impl TextSize {
    pub fn pixels(self) -> Pixels {
        match self {
            TextSize::Large => px(16.0),
            TextSize::Default => px(14.0),
            TextSize::Small => px(12.0),
            TextSize::XSmall => px(10.0),
        }
    }

    pub fn rems(self) -> Rems {
        let px_val: f32 = self.pixels().into();
        rems_from_px(px_val)
    }
}
