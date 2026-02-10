//! App auto-updater (discovery + orchestration).
//!
//! This module intentionally avoids doing installation itself. Instead it:
//! - checks GitHub Releases for a newer version
//! - offers an in-app "Update" action that spawns `butterpaper-updater apply ...`

use std::path::PathBuf;
use std::sync::Once;
use std::time::{Duration, SystemTime};

use butterpaper_update_core::{Repo, SelectedAsset, UpdateChannel};
use gpui::{prelude::*, Context};
use semver::Version;

use crate::components::{text_button, ButtonSize};
use crate::ui::TypographyExt;
use crate::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UpdateCheckFrequency {
    Never,
    OnStartup,
    EveryHour,
    Every6Hours,
    Every12Hours,
    Daily,
    Weekly,
}

impl Default for UpdateCheckFrequency {
    fn default() -> Self {
        Self::Weekly
    }
}

impl gpui::Global for UpdateCheckFrequency {}

impl UpdateCheckFrequency {
    pub fn label(self) -> &'static str {
        match self {
            Self::Never => "Never",
            Self::OnStartup => "On Startup",
            Self::EveryHour => "Every Hour",
            Self::Every6Hours => "Every 6 Hours",
            Self::Every12Hours => "Every 12 Hours",
            Self::Daily => "Daily",
            Self::Weekly => "Weekly",
        }
    }

    fn interval(self) -> Option<Duration> {
        match self {
            Self::Never => None,
            Self::OnStartup => Some(Duration::ZERO),
            Self::EveryHour => Some(Duration::from_secs(60 * 60)),
            Self::Every6Hours => Some(Duration::from_secs(6 * 60 * 60)),
            Self::Every12Hours => Some(Duration::from_secs(12 * 60 * 60)),
            Self::Daily => Some(Duration::from_secs(24 * 60 * 60)),
            Self::Weekly => Some(Duration::from_secs(7 * 24 * 60 * 60)),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UpdateAvailable {
    pub version: String,
    pub tag: String,
    pub asset: String,
    pub url: String,
    pub channel: UpdateChannel,
}

#[derive(Debug, Clone)]
pub enum UpdateCheckBanner {
    Checking { channel: UpdateChannel },
    UpToDate { channel: UpdateChannel, current_version: String },
    Error { message: String },
}

static UPDATE_CHECK_ONCE: Once = Once::new();

fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("butterpaper"))
}

fn last_update_file() -> Option<PathBuf> {
    config_dir().map(|p| p.join(".last_app_update_check"))
}

pub fn should_check(frequency: UpdateCheckFrequency) -> bool {
    if frequency == UpdateCheckFrequency::Never {
        return false;
    }
    if frequency == UpdateCheckFrequency::OnStartup {
        return true;
    }

    let Some(interval) = frequency.interval() else {
        return false;
    };

    let Some(path) = last_update_file() else {
        return true;
    };
    match std::fs::metadata(&path) {
        Ok(meta) => match meta.modified() {
            Ok(modified) => {
                SystemTime::now().duration_since(modified).map(|d| d > interval).unwrap_or(true)
            }
            Err(_) => true,
        },
        Err(_) => true,
    }
}

pub fn mark_checked() {
    if let Some(path) = last_update_file() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, "");
    }
}

pub fn default_channel() -> UpdateChannel {
    #[cfg(feature = "beta")]
    {
        UpdateChannel::Beta
    }
    #[cfg(not(feature = "beta"))]
    {
        UpdateChannel::Stable
    }
}

pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("valid CARGO_PKG_VERSION")
}

pub fn check_for_update_blocking(
    channel: UpdateChannel,
) -> Result<Option<UpdateAvailable>, String> {
    let repo = Repo::new("apotenza92", "ButterPaper");

    let platform = butterpaper_update_core::detect_platform()
        .ok_or_else(|| "unsupported platform".to_string())?;
    let arch =
        butterpaper_update_core::detect_arch().ok_or_else(|| "unsupported arch".to_string())?;

    let current = current_version();
    let selected =
        butterpaper_update_core::check_for_update(repo, channel, platform, arch, &current)
            .map_err(|e| e.to_string())?;
    Ok(selected.map(|s| UpdateAvailable::from_selected(s, channel)))
}

impl UpdateAvailable {
    fn from_selected(sel: SelectedAsset, channel: UpdateChannel) -> Self {
        Self {
            version: sel.version.to_string(),
            tag: sel.tag_name,
            asset: sel.asset_name,
            url: sel.download_url,
            channel,
        }
    }
}

pub fn updater_exe_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let name = if cfg!(windows) { "butterpaper-updater.exe" } else { "butterpaper-updater" };
    Some(dir.join(name))
}

pub fn spawn_apply_update(update: &UpdateAvailable) -> Result<(), String> {
    let updater = updater_exe_path().ok_or_else(|| "updater executable not found".to_string())?;
    if !updater.is_file() {
        return Err(format!("updater executable not found at {}", updater.display()));
    }

    let channel = match update.channel {
        UpdateChannel::Stable => "stable",
        UpdateChannel::Beta => "beta",
    };

    let current = env!("CARGO_PKG_VERSION");
    let parent_pid = std::process::id().to_string();

    // Spawn updater and quit; updater will wait for this PID to exit before applying.
    std::process::Command::new(updater)
        .arg("apply")
        .arg("--channel")
        .arg(channel)
        .arg("--current")
        .arg(current)
        .arg("--parent-pid")
        .arg(parent_pid)
        .spawn()
        .map_err(|e| format!("failed to spawn updater: {e}"))?;

    Ok(())
}

pub fn spawn_update_check_once(cx: &mut Context<crate::app::PdfEditor>) {
    let frequency = cx.try_global::<UpdateCheckFrequency>().copied().unwrap_or_default();
    if !should_check(frequency) {
        return;
    }

    UPDATE_CHECK_ONCE.call_once(|| {
        let channel = default_channel();
        cx.spawn(move |this: gpui::WeakEntity<crate::app::PdfEditor>, cx: &mut gpui::AsyncApp| {
            let mut async_cx = cx.clone();
            async move {
                let result = async_cx
                    .background_executor()
                    .spawn(async move { check_for_update_blocking(channel) })
                    .await;

                mark_checked();

                match result {
                    Ok(Some(update)) => {
                        let _ = this.update(&mut async_cx, move |editor, cx| {
                            editor.update_available = Some(update);
                            cx.notify();
                        });
                    }
                    Ok(None) => {}
                    Err(err) => eprintln!("update check failed: {err}"),
                }
            }
        })
        .detach();
    });
}

#[allow(dead_code)]
pub fn render_update_banner(
    update: &UpdateAvailable,
    theme: &Theme,
    editor: gpui::WeakEntity<crate::app::PdfEditor>,
) -> impl gpui::IntoElement {
    let update = update.clone();

    gpui::div()
        .id("update-banner")
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .px(crate::ui::sizes::SPACE_4)
        .py(crate::ui::sizes::SPACE_2)
        .bg(theme.element_selected)
        .border_b_1()
        .border_color(theme.border)
        .child(gpui::div().text_ui_body().text_color(theme.text).child(format!(
            "Update available: {} ({}, {})",
            update.tag,
            update.version,
            update.channel_name()
        )))
        .child(text_button(
            "update-banner-apply",
            "Update and restart",
            ButtonSize::Medium,
            theme,
            move |_, _window, app| {
                if let Some(editor) = editor.upgrade() {
                    editor.update(app, |editor, cx| {
                        editor.update_available = None;
                        cx.notify();
                    });
                }

                if let Err(err) = spawn_apply_update(&update) {
                    eprintln!("update apply failed: {err}");
                    return;
                }
                app.quit();
            },
        ))
}

#[allow(dead_code)]
pub fn render_update_check_banner(
    banner: &UpdateCheckBanner,
    theme: &Theme,
    editor: gpui::WeakEntity<crate::app::PdfEditor>,
) -> impl gpui::IntoElement {
    let (bg, border) = match banner {
        UpdateCheckBanner::Error { .. } => (theme.danger_bg, theme.danger_border),
        _ => (theme.element_selected, theme.border),
    };

    let (message, show_dismiss) = match banner {
        UpdateCheckBanner::Checking { channel } => {
            (format!("Checking for updates ({})…", channel_name(*channel)), false)
        }
        UpdateCheckBanner::UpToDate { channel, current_version } => {
            (format!("You are up to date (v{}, {})", current_version, channel_name(*channel)), true)
        }
        UpdateCheckBanner::Error { message } => (format!("Update check failed: {message}"), true),
    };

    gpui::div()
        .id("update-check-banner")
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .px(crate::ui::sizes::SPACE_4)
        .py(crate::ui::sizes::SPACE_2)
        .bg(bg)
        .border_b_1()
        .border_color(border)
        .child(gpui::div().text_ui_body().text_color(theme.text).child(message))
        .when(show_dismiss, move |d| {
            d.child(text_button(
                "update-check-dismiss",
                "OK",
                ButtonSize::Medium,
                theme,
                move |_, _window, app| {
                    if let Some(editor) = editor.upgrade() {
                        editor.update(app, |editor, cx| {
                            editor.update_check_banner = None;
                            cx.notify();
                        });
                    }
                },
            ))
        })
}

#[allow(dead_code)]
impl UpdateAvailable {
    fn channel_name(&self) -> &'static str {
        match self.channel {
            UpdateChannel::Stable => "stable",
            UpdateChannel::Beta => "beta",
        }
    }
}

#[allow(dead_code)]
fn channel_name(channel: UpdateChannel) -> &'static str {
    match channel {
        UpdateChannel::Stable => "stable",
        UpdateChannel::Beta => "beta",
    }
}
