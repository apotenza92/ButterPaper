//! Application menu bar setup.

use gpui::{App, Menu, MenuItem};

use crate::app_update::UpdateCheckFrequency;
use crate::settings;
use crate::{
    About, CheckForUpdates, Open, Quit, SetUpdateCheckFrequencyDaily,
    SetUpdateCheckFrequencyEvery12Hours, SetUpdateCheckFrequencyEvery6Hours,
    SetUpdateCheckFrequencyEveryHour, SetUpdateCheckFrequencyNever,
    SetUpdateCheckFrequencyOnStartup, SetUpdateCheckFrequencyWeekly,
};

/// Set up the application menu bar.
pub fn set_menus(cx: &mut App) {
    let frequency = cx
        .try_global::<UpdateCheckFrequency>()
        .copied()
        .unwrap_or_default();

    fn mark(label: &'static str, selected: bool) -> gpui::SharedString {
        if selected {
            format!("✓ {label}").into()
        } else {
            label.into()
        }
    }

    cx.set_menus(vec![
        Menu {
            name: "ButterPaper".into(),
            items: vec![
                MenuItem::action("About ButterPaper", About),
                MenuItem::separator(),
                MenuItem::action("Check for Updates...", CheckForUpdates),
                MenuItem::separator(),
                MenuItem::submenu(Menu {
                    name: "Update Check Frequency".into(),
                    items: vec![
                        MenuItem::action(
                            mark(
                                UpdateCheckFrequency::Never.label(),
                                frequency == UpdateCheckFrequency::Never,
                            ),
                            SetUpdateCheckFrequencyNever,
                        ),
                        MenuItem::action(
                            mark(
                                UpdateCheckFrequency::OnStartup.label(),
                                frequency == UpdateCheckFrequency::OnStartup,
                            ),
                            SetUpdateCheckFrequencyOnStartup,
                        ),
                        MenuItem::action(
                            mark(
                                UpdateCheckFrequency::EveryHour.label(),
                                frequency == UpdateCheckFrequency::EveryHour,
                            ),
                            SetUpdateCheckFrequencyEveryHour,
                        ),
                        MenuItem::action(
                            mark(
                                UpdateCheckFrequency::Every6Hours.label(),
                                frequency == UpdateCheckFrequency::Every6Hours,
                            ),
                            SetUpdateCheckFrequencyEvery6Hours,
                        ),
                        MenuItem::action(
                            mark(
                                UpdateCheckFrequency::Every12Hours.label(),
                                frequency == UpdateCheckFrequency::Every12Hours,
                            ),
                            SetUpdateCheckFrequencyEvery12Hours,
                        ),
                        MenuItem::action(
                            mark(
                                UpdateCheckFrequency::Daily.label(),
                                frequency == UpdateCheckFrequency::Daily,
                            ),
                            SetUpdateCheckFrequencyDaily,
                        ),
                        MenuItem::action(
                            mark(
                                UpdateCheckFrequency::Weekly.label(),
                                frequency == UpdateCheckFrequency::Weekly,
                            ),
                            SetUpdateCheckFrequencyWeekly,
                        ),
                    ],
                }),
                MenuItem::separator(),
                MenuItem::action("Settings...", settings::OpenSettings),
                MenuItem::separator(),
                MenuItem::action("Quit ButterPaper", Quit),
            ],
        },
        Menu { name: "File".into(), items: vec![MenuItem::action("Open...", Open)] },
    ]);
}
