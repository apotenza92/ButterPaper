//! App icon names and asset paths.
//!
//! Mirrors Zed's pattern: semantic icon names that map to vendored SVG assets.

/// Common icons used throughout the app.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconName {
    Close,
    ChevronLeft,
    ChevronRight,
    ArrowLeft,
    ArrowRight,
    PageFirst,
    PageLast,
    PanelLeft,
    PageThumbnails,
    FitWidth,
    FitPage,
    Dirty,
    Settings,
    Check,
    ChevronDown,
    Plus,
    Minus,
    ZoomIn,
    ZoomOut,
    ViewContinuous,
    ViewSinglePage,
}

impl IconName {
    /// Returns the embedded asset path for this icon.
    pub const fn path(self) -> &'static str {
        match self {
            IconName::Close => "icons/close.svg",
            IconName::ChevronLeft => "icons/chevron_left.svg",
            IconName::ChevronRight => "icons/chevron_right.svg",
            IconName::ArrowLeft => "icons/arrow_left.svg",
            IconName::ArrowRight => "icons/arrow_right.svg",
            IconName::PageFirst => "icons/page_first.svg",
            IconName::PageLast => "icons/page_last.svg",
            IconName::PanelLeft => "icons/panel_left.svg",
            IconName::PageThumbnails => "icons/page_thumbnails.svg",
            IconName::FitWidth => "icons/fit_width.svg",
            IconName::FitPage => "icons/fit_page.svg",
            IconName::Dirty => "icons/dirty.svg",
            IconName::Settings => "icons/settings.svg",
            IconName::Check => "icons/check.svg",
            IconName::ChevronDown => "icons/chevron_down.svg",
            IconName::Plus => "icons/plus.svg",
            IconName::Minus => "icons/minus.svg",
            IconName::ZoomIn => "icons/zoom_in.svg",
            IconName::ZoomOut => "icons/zoom_out.svg",
            IconName::ViewContinuous => "icons/view_continuous.svg",
            IconName::ViewSinglePage => "icons/view_single_page.svg",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::IconName;

    #[test]
    fn page_thumbnails_icon_path_is_stable() {
        assert_eq!(IconName::PageThumbnails.path(), "icons/page_thumbnails.svg");
    }
}
