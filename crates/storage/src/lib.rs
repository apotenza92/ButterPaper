use directories::ProjectDirs;
use doc_model::Preferences;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const PREFS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("unable to resolve local data directory")]
    NoDataDirectory,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct Storage {
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PreferencesEnvelope {
    version: u32,
    preferences: Preferences,
}

impl Storage {
    pub fn from_default_project() -> Result<Self, StorageError> {
        let dirs = ProjectDirs::from("dev", "ButterPaper", "ButterPaper")
            .ok_or(StorageError::NoDataDirectory)?;

        Ok(Self { root: dirs.data_local_dir().to_path_buf() })
    }

    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn load_preferences(&self) -> Result<Preferences, StorageError> {
        let path = self.preferences_path();
        if !path.exists() {
            return Ok(Preferences::default());
        }

        let bytes = fs::read(path)?;
        let envelope: PreferencesEnvelope = serde_json::from_slice(&bytes)?;

        Ok(envelope.preferences)
    }

    pub fn save_preferences(&self, preferences: &Preferences) -> Result<(), StorageError> {
        fs::create_dir_all(&self.root)?;

        let envelope =
            PreferencesEnvelope { version: PREFS_SCHEMA_VERSION, preferences: preferences.clone() };

        let bytes = serde_json::to_vec_pretty(&envelope)?;
        fs::write(self.preferences_path(), bytes)?;
        Ok(())
    }

    fn preferences_path(&self) -> PathBuf {
        self.root.join("preferences.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_round_trip() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let store = Storage::with_root(temp.path());

        let prefs =
            Preferences { prefer_tabs: false, show_tab_bar: false, allow_window_merge: false };

        store.save_preferences(&prefs).expect("save should succeed");
        let loaded = store.load_preferences().expect("load should succeed");

        assert_eq!(loaded, prefs);
    }

    #[test]
    fn load_defaults_when_file_absent() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let store = Storage::with_root(temp.path());

        let loaded = store.load_preferences().expect("load should succeed");
        assert_eq!(loaded, Preferences::default());
    }
}
