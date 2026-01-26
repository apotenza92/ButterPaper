//! Theme auto-updater - fetches latest themes from Zed's repository

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Theme sources from Zed's repository
const THEME_SOURCES: &[(&str, &str)] = &[
    ("one", "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/one/one.json"),
    ("ayu", "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/ayu/ayu.json"),
    ("gruvbox", "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/gruvbox/gruvbox.json"),
];

/// How often to check for theme updates (24 hours)
const UPDATE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Get the themes cache directory
pub fn themes_cache_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("butterpaper").join("themes"))
}

/// Get the path to the last update timestamp file
fn last_update_file() -> Option<PathBuf> {
    themes_cache_dir().map(|p| p.join(".last_update"))
}

/// Check if we should update themes (based on time since last update)
pub fn should_update() -> bool {
    let Some(path) = last_update_file() else {
        return true;
    };

    match std::fs::metadata(&path) {
        Ok(meta) => match meta.modified() {
            Ok(modified) => SystemTime::now()
                .duration_since(modified)
                .map(|d| d > UPDATE_INTERVAL)
                .unwrap_or(true),
            Err(_) => true,
        },
        Err(_) => true, // File doesn't exist, should update
    }
}

/// Mark that we've updated themes
fn mark_updated() {
    if let Some(path) = last_update_file() {
        // Touch the file
        let _ = std::fs::write(&path, "");
    }
}

/// Update themes from GitHub (blocking - call from background thread)
pub fn update_themes_blocking() -> Result<(), String> {
    let cache_dir = themes_cache_dir().ok_or("Could not determine cache directory")?;

    // Create cache directory
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache dir: {}", e))?;

    let client = ureq::agent();
    let mut any_updated = false;

    for (name, url) in THEME_SOURCES {
        let target_path = cache_dir.join(format!("{}.json", name));

        match client.get(url).call() {
            Ok(response) => {
                match response.into_string() {
                    Ok(body) => {
                        // Validate it's valid JSON before saving
                        if serde_json::from_str::<serde_json::Value>(&body).is_ok() {
                            if let Err(e) = std::fs::write(&target_path, &body) {
                                eprintln!("Failed to write theme {}: {}", name, e);
                            } else {
                                any_updated = true;
                            }
                        } else {
                            eprintln!("Invalid JSON received for theme {}", name);
                        }
                    }
                    Err(e) => eprintln!("Failed to read response for {}: {}", name, e),
                }
            }
            Err(e) => eprintln!("Failed to fetch theme {}: {}", name, e),
        }
    }

    if any_updated {
        mark_updated();
    }

    Ok(())
}

/// Load a theme from cache if available
pub fn load_cached_theme(name: &str) -> Option<String> {
    let cache_dir = themes_cache_dir()?;
    let path = cache_dir.join(format!("{}.json", name));
    std::fs::read_to_string(path).ok()
}

/// Spawn a background thread to update themes
pub fn spawn_update_check() {
    if !should_update() {
        return;
    }

    std::thread::spawn(|| {
        if let Err(e) = update_themes_blocking() {
            eprintln!("Theme update failed: {}", e);
        }
    });
}
