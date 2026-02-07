#![allow(dead_code)]

use gpui::{px, Pixels, Rems};

use crate::styles::units::rems_from_px;

/// Fixed spacing tokens for layout helpers.
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
    fn value(self) -> f32 {
        match self {
            DynamicSpacing::Base00 => 0.0,
            DynamicSpacing::Base01 => 1.0,
            DynamicSpacing::Base02 => 2.0,
            DynamicSpacing::Base03 => 3.0,
            DynamicSpacing::Base04 => 4.0,
            DynamicSpacing::Base06 => 6.0,
            DynamicSpacing::Base08 => 8.0,
            DynamicSpacing::Base10 => 12.0,
            DynamicSpacing::Base12 => 16.0,
            DynamicSpacing::Base16 => 20.0,
            DynamicSpacing::Base24 => 24.0,
            DynamicSpacing::Base32 => 32.0,
            DynamicSpacing::Base40 => 40.0,
            DynamicSpacing::Base48 => 48.0,
        }
    }

    pub fn px(self) -> Pixels {
        px(self.value())
    }

    pub fn rems(self) -> Rems {
        rems_from_px(self.value())
    }
}

#[cfg(test)]
mod tests {
    use super::DynamicSpacing;

    #[test]
    fn spacing_values_are_fixed() {
        let medium = DynamicSpacing::Base24.px();
        let medium: f32 = medium.into();
        assert_eq!(medium, 24.0);
    }
}
