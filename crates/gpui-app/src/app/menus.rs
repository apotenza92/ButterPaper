//! Application menu bar setup.

use gpui::{App, Menu, MenuItem};

use crate::settings;
use crate::{About, NextPage, Open, PrevPage, Quit, ZoomIn, ZoomOut};

/// Set up the application menu bar.
pub fn set_menus(cx: &mut App) {
    cx.set_menus(vec![
        Menu {
            name: "PDF Editor".into(),
            items: vec![
                MenuItem::action("About PDF Editor", About),
                MenuItem::separator(),
                MenuItem::action("Settings...", settings::OpenSettings),
                MenuItem::separator(),
                MenuItem::action("Quit PDF Editor", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![MenuItem::action("Open...", Open)],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Zoom In", ZoomIn),
                MenuItem::action("Zoom Out", ZoomOut),
                MenuItem::separator(),
                MenuItem::action("Next Page", NextPage),
                MenuItem::action("Previous Page", PrevPage),
            ],
        },
    ]);
}
