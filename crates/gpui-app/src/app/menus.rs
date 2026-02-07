//! Application menu bar setup.

use gpui::{App, Menu, MenuItem};

use crate::settings;
use crate::{About, Open, Quit};

/// Set up the application menu bar.
pub fn set_menus(cx: &mut App) {
    cx.set_menus(vec![
        Menu {
            name: "ButterPaper".into(),
            items: vec![
                MenuItem::action("About ButterPaper", About),
                MenuItem::separator(),
                MenuItem::action("Settings...", settings::OpenSettings),
                MenuItem::separator(),
                MenuItem::action("Quit ButterPaper", Quit),
            ],
        },
        Menu { name: "File".into(), items: vec![MenuItem::action("Open...", Open)] },
    ]);
}
