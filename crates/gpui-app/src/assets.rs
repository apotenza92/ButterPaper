//! Embedded app assets for GPUI.
//!
//! We keep icon SVGs in-repo and expose them through `AssetSource`, which lets
//! `svg().path(...)` resolve them in both app runtime and tests.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let bytes: Option<&'static [u8]> = match path {
            "icons/arrow_left.svg" => Some(include_bytes!("../assets/icons/arrow_left.svg")),
            "icons/arrow_right.svg" => Some(include_bytes!("../assets/icons/arrow_right.svg")),
            "icons/check.svg" => Some(include_bytes!("../assets/icons/check.svg")),
            "icons/chevron_down.svg" => Some(include_bytes!("../assets/icons/chevron_down.svg")),
            "icons/chevron_left.svg" => Some(include_bytes!("../assets/icons/chevron_left.svg")),
            "icons/chevron_right.svg" => Some(include_bytes!("../assets/icons/chevron_right.svg")),
            "icons/close.svg" => Some(include_bytes!("../assets/icons/close.svg")),
            "icons/dirty.svg" => Some(include_bytes!("../assets/icons/dirty.svg")),
            "icons/fit_page.svg" => Some(include_bytes!("../assets/icons/fit_page.svg")),
            "icons/fit_width.svg" => Some(include_bytes!("../assets/icons/fit_width.svg")),
            "icons/minus.svg" => Some(include_bytes!("../assets/icons/minus.svg")),
            "icons/page_first.svg" => Some(include_bytes!("../assets/icons/page_first.svg")),
            "icons/page_last.svg" => Some(include_bytes!("../assets/icons/page_last.svg")),
            "icons/panel_left.svg" => Some(include_bytes!("../assets/icons/panel_left.svg")),
            "icons/plus.svg" => Some(include_bytes!("../assets/icons/plus.svg")),
            "icons/settings.svg" => Some(include_bytes!("../assets/icons/settings.svg")),
            _ => None,
        };

        Ok(bytes.map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        if path.is_empty() || path == "icons" || path == "icons/" {
            return Ok(vec![
                "icons/arrow_left.svg".into(),
                "icons/arrow_right.svg".into(),
                "icons/check.svg".into(),
                "icons/chevron_down.svg".into(),
                "icons/chevron_left.svg".into(),
                "icons/chevron_right.svg".into(),
                "icons/close.svg".into(),
                "icons/dirty.svg".into(),
                "icons/fit_page.svg".into(),
                "icons/fit_width.svg".into(),
                "icons/minus.svg".into(),
                "icons/page_first.svg".into(),
                "icons/page_last.svg".into(),
                "icons/panel_left.svg".into(),
                "icons/plus.svg".into(),
                "icons/settings.svg".into(),
            ]);
        }

        Ok(vec![])
    }
}
