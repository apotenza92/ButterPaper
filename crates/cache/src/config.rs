//! Cache configuration system for user-configurable cache sizes and locations.
//!
//! This module provides a centralized configuration system for all cache types
//! (RAM, GPU, Disk). Configuration can be loaded from a file, environment variables,
//! or created programmatically.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Configuration for the cache system.
///
/// Provides user-configurable settings for RAM, GPU (VRAM), and disk cache limits,
/// as well as the disk cache location.
#[derive(Debug, Clone, PartialEq)]
pub struct CacheConfig {
    /// RAM cache size limit in bytes
    pub ram_cache_size: usize,
    /// GPU (VRAM) cache size limit in bytes
    pub gpu_cache_size: usize,
    /// Disk cache size limit in bytes
    pub disk_cache_size: usize,
    /// Directory path for disk cache storage
    pub disk_cache_dir: PathBuf,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ram_cache_size: 256 * 1024 * 1024,   // 256 MB
            gpu_cache_size: 512 * 1024 * 1024,   // 512 MB
            disk_cache_size: 1024 * 1024 * 1024, // 1 GB
            disk_cache_dir: Self::default_cache_dir(),
        }
    }
}

impl CacheConfig {
    /// Creates a new cache configuration with custom values in megabytes.
    ///
    /// # Arguments
    /// * `ram_mb` - RAM cache size in megabytes
    /// * `gpu_mb` - GPU cache size in megabytes
    /// * `disk_mb` - Disk cache size in megabytes
    /// * `disk_dir` - Directory path for disk cache storage
    pub fn new(ram_mb: usize, gpu_mb: usize, disk_mb: usize, disk_dir: PathBuf) -> Self {
        Self {
            ram_cache_size: ram_mb * 1024 * 1024,
            gpu_cache_size: gpu_mb * 1024 * 1024,
            disk_cache_size: disk_mb * 1024 * 1024,
            disk_cache_dir: disk_dir,
        }
    }

    /// Sets the RAM cache size in megabytes.
    pub fn with_ram_mb(mut self, mb: usize) -> Self {
        self.ram_cache_size = mb * 1024 * 1024;
        self
    }

    /// Sets the GPU cache size in megabytes.
    pub fn with_gpu_mb(mut self, mb: usize) -> Self {
        self.gpu_cache_size = mb * 1024 * 1024;
        self
    }

    /// Sets the disk cache size in megabytes.
    pub fn with_disk_mb(mut self, mb: usize) -> Self {
        self.disk_cache_size = mb * 1024 * 1024;
        self
    }

    /// Sets the disk cache directory.
    pub fn with_disk_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.disk_cache_dir = path.as_ref().to_path_buf();
        self
    }

    /// Returns the default cache directory for the current platform.
    ///
    /// - macOS: ~/Library/Caches/pdf-editor/tiles
    /// - Linux: ~/.cache/pdf-editor/tiles
    /// - Windows: %LOCALAPPDATA%\pdf-editor\tiles
    pub fn default_cache_dir() -> PathBuf {
        if let Some(cache_dir) = dirs::cache_dir() {
            cache_dir.join("pdf-editor").join("tiles")
        } else {
            // Fallback to current directory if cache dir unavailable
            PathBuf::from("cache/tiles")
        }
    }

    /// Loads configuration from environment variables.
    ///
    /// Environment variables:
    /// - `PDF_EDITOR_RAM_CACHE_MB`: RAM cache size in MB (default: 256)
    /// - `PDF_EDITOR_GPU_CACHE_MB`: GPU cache size in MB (default: 512)
    /// - `PDF_EDITOR_DISK_CACHE_MB`: Disk cache size in MB (default: 1024)
    /// - `PDF_EDITOR_CACHE_DIR`: Disk cache directory path
    ///
    /// # Errors
    /// Returns an error if any environment variable contains an invalid value.
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();

        if let Ok(val) = std::env::var("PDF_EDITOR_RAM_CACHE_MB") {
            config.ram_cache_size = val
                .parse::<usize>()
                .map_err(|_| ConfigError::InvalidValue("PDF_EDITOR_RAM_CACHE_MB".to_string()))?
                * 1024
                * 1024;
        }

        if let Ok(val) = std::env::var("PDF_EDITOR_GPU_CACHE_MB") {
            config.gpu_cache_size = val
                .parse::<usize>()
                .map_err(|_| ConfigError::InvalidValue("PDF_EDITOR_GPU_CACHE_MB".to_string()))?
                * 1024
                * 1024;
        }

        if let Ok(val) = std::env::var("PDF_EDITOR_DISK_CACHE_MB") {
            config.disk_cache_size = val
                .parse::<usize>()
                .map_err(|_| ConfigError::InvalidValue("PDF_EDITOR_DISK_CACHE_MB".to_string()))?
                * 1024
                * 1024;
        }

        if let Ok(val) = std::env::var("PDF_EDITOR_CACHE_DIR") {
            config.disk_cache_dir = PathBuf::from(val);
        }

        Ok(config)
    }

    /// Loads configuration from a TOML file.
    ///
    /// Expected file format:
    /// ```toml
    /// ram_cache_mb = 256
    /// gpu_cache_mb = 512
    /// disk_cache_mb = 1024
    /// disk_cache_dir = "/path/to/cache"
    /// ```
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path.as_ref()).map_err(ConfigError::IoError)?;

        Self::from_toml(&contents)
    }

    /// Parses configuration from a TOML string.
    fn from_toml(toml_str: &str) -> Result<Self, ConfigError> {
        let mut config = Self::default();

        for line in toml_str.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');

                match key {
                    "ram_cache_mb" => {
                        config.ram_cache_size = value
                            .parse::<usize>()
                            .map_err(|_| ConfigError::InvalidValue(key.to_string()))?
                            * 1024
                            * 1024;
                    }
                    "gpu_cache_mb" => {
                        config.gpu_cache_size = value
                            .parse::<usize>()
                            .map_err(|_| ConfigError::InvalidValue(key.to_string()))?
                            * 1024
                            * 1024;
                    }
                    "disk_cache_mb" => {
                        config.disk_cache_size = value
                            .parse::<usize>()
                            .map_err(|_| ConfigError::InvalidValue(key.to_string()))?
                            * 1024
                            * 1024;
                    }
                    "disk_cache_dir" => {
                        config.disk_cache_dir = PathBuf::from(value);
                    }
                    _ => {} // Ignore unknown keys
                }
            }
        }

        Ok(config)
    }

    /// Saves configuration to a TOML file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be written.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let toml = self.to_toml();
        fs::write(path.as_ref(), toml).map_err(ConfigError::IoError)
    }

    /// Converts configuration to TOML format.
    fn to_toml(&self) -> String {
        format!(
            "# PDF Editor Cache Configuration\n\
             ram_cache_mb = {}\n\
             gpu_cache_mb = {}\n\
             disk_cache_mb = {}\n\
             disk_cache_dir = \"{}\"\n",
            self.ram_cache_size / (1024 * 1024),
            self.gpu_cache_size / (1024 * 1024),
            self.disk_cache_size / (1024 * 1024),
            self.disk_cache_dir.display()
        )
    }

    /// Returns the RAM cache size in megabytes.
    pub fn ram_cache_mb(&self) -> usize {
        self.ram_cache_size / (1024 * 1024)
    }

    /// Returns the GPU cache size in megabytes.
    pub fn gpu_cache_mb(&self) -> usize {
        self.gpu_cache_size / (1024 * 1024)
    }

    /// Returns the disk cache size in megabytes.
    pub fn disk_cache_mb(&self) -> usize {
        self.disk_cache_size / (1024 * 1024)
    }
}

/// Errors that can occur during configuration operations.
#[derive(Debug)]
pub enum ConfigError {
    /// Invalid value for a configuration parameter
    InvalidValue(String),
    /// I/O error reading or writing configuration file
    IoError(io::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidValue(key) => {
                write!(f, "Invalid value for configuration key: {}", key)
            }
            ConfigError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = CacheConfig::default();
        assert_eq!(config.ram_cache_size, 256 * 1024 * 1024);
        assert_eq!(config.gpu_cache_size, 512 * 1024 * 1024);
        assert_eq!(config.disk_cache_size, 1024 * 1024 * 1024);
    }

    #[test]
    fn test_new_config() {
        let config = CacheConfig::new(128, 256, 512, PathBuf::from("/tmp/cache"));
        assert_eq!(config.ram_cache_size, 128 * 1024 * 1024);
        assert_eq!(config.gpu_cache_size, 256 * 1024 * 1024);
        assert_eq!(config.disk_cache_size, 512 * 1024 * 1024);
        assert_eq!(config.disk_cache_dir, PathBuf::from("/tmp/cache"));
    }

    #[test]
    fn test_builder_methods() {
        let config = CacheConfig::default()
            .with_ram_mb(512)
            .with_gpu_mb(1024)
            .with_disk_mb(2048)
            .with_disk_dir("/custom/path");

        assert_eq!(config.ram_cache_size, 512 * 1024 * 1024);
        assert_eq!(config.gpu_cache_size, 1024 * 1024 * 1024);
        assert_eq!(config.disk_cache_size, 2048 * 1024 * 1024);
        assert_eq!(config.disk_cache_dir, PathBuf::from("/custom/path"));
    }

    #[test]
    fn test_mb_getters() {
        let config = CacheConfig::default();
        assert_eq!(config.ram_cache_mb(), 256);
        assert_eq!(config.gpu_cache_mb(), 512);
        assert_eq!(config.disk_cache_mb(), 1024);
    }

    #[test]
    #[serial]
    fn test_from_env() {
        // Save and restore env vars to avoid test pollution
        let _guard = EnvGuard::new(&[
            "PDF_EDITOR_RAM_CACHE_MB",
            "PDF_EDITOR_GPU_CACHE_MB",
            "PDF_EDITOR_DISK_CACHE_MB",
            "PDF_EDITOR_CACHE_DIR",
        ]);

        env::set_var("PDF_EDITOR_RAM_CACHE_MB", "128");
        env::set_var("PDF_EDITOR_GPU_CACHE_MB", "256");
        env::set_var("PDF_EDITOR_DISK_CACHE_MB", "512");
        env::set_var("PDF_EDITOR_CACHE_DIR", "/tmp/test-cache");

        let config = CacheConfig::from_env().unwrap();
        assert_eq!(config.ram_cache_size, 128 * 1024 * 1024);
        assert_eq!(config.gpu_cache_size, 256 * 1024 * 1024);
        assert_eq!(config.disk_cache_size, 512 * 1024 * 1024);
        assert_eq!(config.disk_cache_dir, PathBuf::from("/tmp/test-cache"));
    }

    #[test]
    #[serial]
    fn test_from_env_partial() {
        // Save and restore env vars to avoid test pollution
        let _guard = EnvGuard::new(&[
            "PDF_EDITOR_RAM_CACHE_MB",
            "PDF_EDITOR_GPU_CACHE_MB",
            "PDF_EDITOR_DISK_CACHE_MB",
            "PDF_EDITOR_CACHE_DIR",
        ]);

        // Clear all env vars first, then set only RAM
        env::remove_var("PDF_EDITOR_GPU_CACHE_MB");
        env::remove_var("PDF_EDITOR_DISK_CACHE_MB");
        env::remove_var("PDF_EDITOR_CACHE_DIR");
        env::set_var("PDF_EDITOR_RAM_CACHE_MB", "128");

        let config = CacheConfig::from_env().unwrap();
        assert_eq!(config.ram_cache_size, 128 * 1024 * 1024);
        assert_eq!(config.gpu_cache_size, 512 * 1024 * 1024); // default
        assert_eq!(config.disk_cache_size, 1024 * 1024 * 1024); // default
    }

    #[test]
    #[serial]
    fn test_from_env_invalid() {
        let _guard = EnvGuard::new(&["PDF_EDITOR_RAM_CACHE_MB"]);

        env::set_var("PDF_EDITOR_RAM_CACHE_MB", "not_a_number");
        let result = CacheConfig::from_env();
        assert!(result.is_err());
    }

    // Helper to save and restore environment variables
    struct EnvGuard {
        vars: Vec<(String, Option<String>)>,
    }

    impl EnvGuard {
        fn new(var_names: &[&str]) -> Self {
            let vars = var_names
                .iter()
                .map(|name| (name.to_string(), env::var(name).ok()))
                .collect();
            Self { vars }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (name, value) in &self.vars {
                match value {
                    Some(v) => env::set_var(name, v),
                    None => env::remove_var(name),
                }
            }
        }
    }

    #[test]
    fn test_toml_roundtrip() {
        let config = CacheConfig::new(128, 256, 512, PathBuf::from("/tmp/cache"));
        let toml = config.to_toml();
        let parsed = CacheConfig::from_toml(&toml).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn test_from_toml() {
        let toml = r#"
            # Test configuration
            ram_cache_mb = 128
            gpu_cache_mb = 256
            disk_cache_mb = 512
            disk_cache_dir = "/tmp/test"
        "#;

        let config = CacheConfig::from_toml(toml).unwrap();
        assert_eq!(config.ram_cache_size, 128 * 1024 * 1024);
        assert_eq!(config.gpu_cache_size, 256 * 1024 * 1024);
        assert_eq!(config.disk_cache_size, 512 * 1024 * 1024);
        assert_eq!(config.disk_cache_dir, PathBuf::from("/tmp/test"));
    }

    #[test]
    fn test_from_toml_partial() {
        let toml = r#"
            ram_cache_mb = 128
        "#;

        let config = CacheConfig::from_toml(toml).unwrap();
        assert_eq!(config.ram_cache_size, 128 * 1024 * 1024);
        assert_eq!(config.gpu_cache_size, 512 * 1024 * 1024); // default
    }

    #[test]
    fn test_file_save_and_load() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_cache_config.toml");

        let config = CacheConfig::new(128, 256, 512, PathBuf::from("/tmp/cache"));
        config.save_to_file(&config_path).unwrap();

        let loaded = CacheConfig::from_file(&config_path).unwrap();
        assert_eq!(config, loaded);

        // Cleanup
        let _ = fs::remove_file(config_path);
    }
}
