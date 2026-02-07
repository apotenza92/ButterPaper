use gpui::{rems, Rems};

/// The base rem size in pixels.
pub const BASE_REM_SIZE_IN_PX: f32 = 16.0;

/// Convert a px value into rems using a 16px base.
#[inline(always)]
pub fn rems_from_px(px: impl Into<f32>) -> Rems {
    rems(px.into() / BASE_REM_SIZE_IN_PX)
}

#[cfg(test)]
mod tests {
    use super::{rems_from_px, BASE_REM_SIZE_IN_PX};

    #[test]
    fn rems_from_px_uses_base_16() {
        let rems = rems_from_px(16.0);
        assert!((rems.0 - 1.0).abs() < f32::EPSILON);

        let rems = rems_from_px(14.0);
        assert!((rems.0 - (14.0 / BASE_REM_SIZE_IN_PX)).abs() < f32::EPSILON);
    }
}
