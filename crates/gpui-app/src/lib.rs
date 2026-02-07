//! ButterPaper GPUI - shared library for UI components and utilities

use gpui::actions;

pub mod styles;
pub mod ui;

actions!(
    butterpaper,
    [
        Quit,
        Open,
        About,
        ZoomIn,
        ZoomOut,
        ResetZoom,
        FitWidth,
        FitPage,
        FirstPage,
        LastPage,
        NextPage,
        PrevPage,
        CloseWindow,
        NextTab,
        PrevTab,
        CloseTab
    ]
);
