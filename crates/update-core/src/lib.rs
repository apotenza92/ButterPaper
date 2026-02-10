//! Update discovery and asset selection for ButterPaper.
//!
//! This crate is intentionally UI-free and installer-free:
//! - It selects the correct GitHub Release + asset for a given channel/platform/arch.
//! - Application/install logic (zip extraction, NSIS execution, AppImage swap) lives elsewhere.

use semver::Version;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateChannel {
    Stable,
    Beta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Macos,
    Windows,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X64,
    Arm64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedAsset {
    pub tag_name: String,
    pub version: Version,
    pub channel: UpdateChannel,
    pub asset_name: String,
    pub download_url: String,
}

#[derive(Debug, Clone, Copy)]
pub struct Repo {
    pub owner: &'static str,
    pub name: &'static str,
}

impl Repo {
    pub const fn new(owner: &'static str, name: &'static str) -> Self {
        Self { owner, name }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("network error: {0}")]
    Network(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("no suitable release found for channel={channel:?}")]
    NoRelease { channel: UpdateChannel },
    #[error("missing expected asset '{asset_name}' in tag {tag_name}")]
    MissingAsset { tag_name: String, asset_name: String },
    #[error("unsupported platform/arch")]
    Unsupported,
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    prerelease: bool,
    draft: bool,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

pub fn detect_platform() -> Option<Platform> {
    match std::env::consts::OS {
        "macos" => Some(Platform::Macos),
        "windows" => Some(Platform::Windows),
        "linux" => Some(Platform::Linux),
        _ => None,
    }
}

pub fn detect_arch() -> Option<Arch> {
    match std::env::consts::ARCH {
        "x86_64" => Some(Arch::X64),
        "aarch64" => Some(Arch::Arm64),
        _ => None,
    }
}

pub fn expected_asset_name(
    channel: UpdateChannel,
    platform: Platform,
    arch: Arch,
    version: &Version,
) -> Option<String> {
    let prefix = match channel {
        UpdateChannel::Stable => "ButterPaper",
        UpdateChannel::Beta => "ButterPaper-Beta",
    };
    let arch_str = match arch {
        Arch::X64 => "x64",
        Arch::Arm64 => "arm64",
    };

    let v = version;
    let name = match platform {
        Platform::Macos => format!("{prefix}-v{v}-macos-{arch_str}.zip"),
        Platform::Windows => format!("{prefix}-v{v}-windows-{arch_str}-setup.exe"),
        Platform::Linux => format!("{prefix}-v{v}-linux-{arch_str}.AppImage"),
    };
    Some(name)
}

fn parse_tag(tag: &str) -> Option<(Version, bool, Option<u64>)> {
    // Supported tags:
    // - vX.Y.Z
    // - vX.Y.Z-beta.N
    let tag = tag.strip_prefix('v')?;
    if let Some((core, beta)) = tag.split_once("-beta.") {
        let v = Version::parse(core).ok()?;
        let n = beta.parse::<u64>().ok()?;
        return Some((v, true, Some(n)));
    }
    let v = Version::parse(tag).ok()?;
    Some((v, false, None))
}

fn pick_release_for_channel(releases: &[GhRelease], channel: UpdateChannel) -> Option<(Version, String)> {
    // We rank by core semver first.
    // For equal core:
    // - prefer stable (non-prerelease) over prerelease (beta)
    // - if both prerelease, prefer higher beta.N
    let mut best: Option<(Version, bool, u64, String)> = None; // (core, is_prerelease, beta_n, tag)

    for r in releases {
        if r.draft {
            continue;
        }
        let Some((core, is_beta_tag, beta_n)) = parse_tag(&r.tag_name) else {
            continue;
        };

        // For the stable channel, only consider stable tags.
        if channel == UpdateChannel::Stable && (r.prerelease || is_beta_tag) {
            continue;
        }

        // For the beta channel:
        // - consider both stable and prerelease tags.
        // - treat stable as "preferred" when core matches.
        let prerelease = r.prerelease || is_beta_tag;
        let beta_n = beta_n.unwrap_or(0);

        let candidate = (core.clone(), prerelease, beta_n, r.tag_name.clone());
        best = match best {
            None => Some(candidate),
            Some((best_core, best_pre, best_n, best_tag)) => {
                if candidate.0 > best_core {
                    Some(candidate)
                } else if candidate.0 < best_core {
                    Some((best_core, best_pre, best_n, best_tag))
                } else {
                    // same core
                    match (best_pre, prerelease) {
                        (true, false) => Some(candidate), // stable beats prerelease
                        (false, true) => Some((best_core, best_pre, best_n, best_tag)),
                        (false, false) => Some((best_core, best_pre, best_n, best_tag)),
                        (true, true) => {
                            if beta_n > best_n {
                                Some(candidate)
                            } else {
                                Some((best_core, best_pre, best_n, best_tag))
                            }
                        }
                    }
                }
            }
        };
    }

    best.map(|(v, _pre, _n, tag)| (v, tag))
}

fn select_update_asset_from_releases(
    _repo: Repo,
    channel: UpdateChannel,
    platform: Platform,
    arch: Arch,
    current_version: &Version,
    releases: &[GhRelease],
) -> Result<Option<SelectedAsset>, UpdateError> {
    let Some((target_version, tag_name)) = pick_release_for_channel(releases, channel) else {
        return Err(UpdateError::NoRelease { channel });
    };

    if target_version <= *current_version {
        return Ok(None);
    }

    let asset_name =
        expected_asset_name(channel, platform, arch, &target_version).ok_or(UpdateError::Unsupported)?;

    let release = releases
        .iter()
        .find(|r| r.tag_name == tag_name)
        .ok_or_else(|| UpdateError::InvalidResponse("picked tag not found".into()))?;

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| UpdateError::MissingAsset {
            tag_name: release.tag_name.clone(),
            asset_name: asset_name.clone(),
        })?;

    Ok(Some(SelectedAsset {
        tag_name: release.tag_name.clone(),
        version: target_version,
        channel,
        asset_name: asset.name.clone(),
        download_url: asset.browser_download_url.clone(),
    }))
}

fn fetch_releases(repo: Repo) -> Result<Vec<GhRelease>, UpdateError> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases?per_page=20",
        repo.owner, repo.name
    );

    let agent = ureq::agent();
    let resp = agent
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", "ButterPaper-Updater")
        .call()
        .map_err(|e| UpdateError::Network(e.to_string()))?;

    let body = resp
        .into_string()
        .map_err(|e| UpdateError::InvalidResponse(e.to_string()))?;

    serde_json::from_str::<Vec<GhRelease>>(&body)
        .map_err(|e| UpdateError::InvalidResponse(e.to_string()))
}

pub fn check_for_update(
    repo: Repo,
    channel: UpdateChannel,
    platform: Platform,
    arch: Arch,
    current_version: &Version,
) -> Result<Option<SelectedAsset>, UpdateError> {
    let releases = fetch_releases(repo)?;
    select_update_asset_from_releases(repo, channel, platform, arch, current_version, &releases)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_release(tag: &str, prerelease: bool, assets: &[&str]) -> GhRelease {
        GhRelease {
            tag_name: tag.to_string(),
            prerelease,
            draft: false,
            assets: assets
                .iter()
                .map(|name| GhAsset {
                    name: name.to_string(),
                    browser_download_url: format!("https://example.invalid/{name}"),
                })
                .collect(),
        }
    }

    #[test]
    fn stable_channel_ignores_prereleases() {
        let repo = Repo::new("o", "r");
        let current = Version::parse("0.0.1").unwrap();
        let v002 = Version::parse("0.0.2").unwrap();
        let v003 = Version::parse("0.0.3").unwrap();

        let stable_asset_002 =
            expected_asset_name(UpdateChannel::Stable, Platform::Macos, Arch::X64, &v002).unwrap();
        let stable_asset_003 =
            expected_asset_name(UpdateChannel::Stable, Platform::Macos, Arch::X64, &v003).unwrap();

        let releases = vec![
            mk_release("v0.0.3-beta.1", true, &[&stable_asset_003]),
            mk_release("v0.0.2", false, &[&stable_asset_002]),
        ];

        let sel = select_update_asset_from_releases(
            repo,
            UpdateChannel::Stable,
            Platform::Macos,
            Arch::X64,
            &current,
            &releases,
        )
        .unwrap()
        .unwrap();

        assert_eq!(sel.tag_name, "v0.0.2");
    }

    #[test]
    fn beta_channel_prefers_higher_core_even_if_only_stable() {
        let repo = Repo::new("o", "r");
        let current = Version::parse("0.0.1").unwrap();
        let v002 = Version::parse("0.0.2").unwrap();

        let beta_asset_002 =
            expected_asset_name(UpdateChannel::Beta, Platform::Windows, Arch::Arm64, &v002).unwrap();

        let releases = vec![mk_release("v0.0.2", false, &[&beta_asset_002])];

        let sel = select_update_asset_from_releases(
            repo,
            UpdateChannel::Beta,
            Platform::Windows,
            Arch::Arm64,
            &current,
            &releases,
        )
        .unwrap()
        .unwrap();

        assert_eq!(sel.tag_name, "v0.0.2");
        assert_eq!(sel.asset_name, beta_asset_002);
    }

    #[test]
    fn beta_channel_prefers_stable_over_beta_for_same_core() {
        let repo = Repo::new("o", "r");
        let current = Version::parse("0.0.1").unwrap();
        let v002 = Version::parse("0.0.2").unwrap();

        let beta_asset_002 =
            expected_asset_name(UpdateChannel::Beta, Platform::Linux, Arch::X64, &v002).unwrap();

        let releases = vec![
            mk_release("v0.0.2-beta.3", true, &[&beta_asset_002]),
            mk_release("v0.0.2", false, &[&beta_asset_002]),
        ];

        let sel = select_update_asset_from_releases(
            repo,
            UpdateChannel::Beta,
            Platform::Linux,
            Arch::X64,
            &current,
            &releases,
        )
        .unwrap()
        .unwrap();

        assert_eq!(sel.tag_name, "v0.0.2");
    }
}
