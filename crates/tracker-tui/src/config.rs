/// Application configuration loaded from a TOML file.
///
/// Config is searched for in (in order):
///   1. `$TRACKER_RS_CONFIG` env var
///   2. `$XDG_CONFIG_HOME/riffl/config.toml`
///   3. `~/.config/riffl/config.toml`
use serde::{Deserialize, Serialize};

use crate::ui::theme::ThemeKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Color theme name: "dark", "mocha" / "catppuccin-mocha", "nord"
    pub theme: String,

    /// Default BPM for new projects
    pub default_bpm: f64,

    /// Default number of rows for new patterns
    pub default_pattern_rows: usize,

    /// Default number of channels (tracks) for new patterns
    pub default_channels: usize,

    /// Additional sample directories shown in the sample browser.
    /// `~/.config/riffl/samples/` is always included automatically.
    /// Also overridden/extended by RIFFL_SAMPLE_DIR env var or --sample-dir CLI flag.
    #[serde(default)]
    pub sample_dirs: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "mocha".to_string(),
            default_bpm: 125.0,
            default_pattern_rows: 16,
            default_channels: 4,
            sample_dirs: Vec::new(),
        }
    }
}

impl Config {
    /// Load configuration from the first available config file path.
    /// Falls back to defaults if no config file is found or if parsing fails.
    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(content) => match toml::from_str::<Config>(&content) {
                        Ok(cfg) => return cfg,
                        Err(e) => {
                            eprintln!("riffl: config parse error in {}: {e}", path.display());
                        }
                    },
                    Err(e) => {
                        eprintln!("riffl: could not read config {}: {e}", path.display());
                    }
                }
            }
        }
        Config::default()
    }

    /// Save the current configuration to the config file.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or_else(|| "Cannot determine config path".to_string())?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| format!("serialize config: {e}"))?;
        std::fs::write(&path, content).map_err(|e| format!("write config: {e}"))?;
        Ok(())
    }

    /// Resolve the ThemeKind for this config's theme string.
    pub fn theme_kind(&self) -> ThemeKind {
        ThemeKind::from_str(&self.theme).unwrap_or_default()
    }

    /// Return the riffl config directory (`$XDG_CONFIG_HOME/riffl` or `~/.config/riffl`).
    pub fn config_dir() -> std::path::PathBuf {
        let base = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            std::path::PathBuf::from(xdg)
        } else {
            dirs_next().join(".config")
        };
        base.join("riffl")
    }

    /// Return the default samples directory (`~/.config/riffl/samples/`).
    pub fn default_samples_dir() -> std::path::PathBuf {
        Self::config_dir().join("samples")
    }

    /// Resolve all sample directories for the browser.
    ///
    /// Order: default samples dir, then config `sample_dirs`, then
    /// RIFFL_SAMPLE_DIR env var (if set), then --sample-dir CLI flag (if set).
    /// Duplicates are removed. Directories that don't exist are kept
    /// (the browser shows them as empty rather than hiding them).
    pub fn resolve_sample_dirs(&self, cli_override: Option<&str>) -> Vec<std::path::PathBuf> {
        let mut dirs: Vec<std::path::PathBuf> = Vec::new();

        // 1. Always include the default samples dir
        dirs.push(Self::default_samples_dir());

        // 2. Dirs from config file
        for p in &self.sample_dirs {
            let path = std::path::PathBuf::from(p);
            if !dirs.contains(&path) {
                dirs.push(path);
            }
        }

        // 3. RIFFL_SAMPLE_DIR env var
        if let Ok(p) = std::env::var("RIFFL_SAMPLE_DIR") {
            let path = std::path::PathBuf::from(p);
            if !dirs.contains(&path) {
                dirs.push(path);
            }
        }

        // 4. --sample-dir CLI flag
        if let Some(p) = cli_override {
            let path = std::path::PathBuf::from(p);
            if !dirs.contains(&path) {
                dirs.push(path);
            }
        }

        dirs
    }

    /// Return the config file path (does not check whether it exists).
    pub fn config_path() -> Option<std::path::PathBuf> {
        // 1. Explicit env override
        if let Ok(p) = std::env::var("TRACKER_RS_CONFIG") {
            return Some(std::path::PathBuf::from(p));
        }
        // 2. XDG_CONFIG_HOME / ~/.config
        let base = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            std::path::PathBuf::from(xdg)
        } else {
            dirs_next().join(".config")
        };
        Some(base.join("riffl").join("config.toml"))
    }
}

/// Return the user's home directory.
fn dirs_next() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let c = Config::default();
        assert_eq!(c.theme, "mocha");
        assert_eq!(c.default_bpm, 125.0);
        assert_eq!(c.default_pattern_rows, 16);
        assert_eq!(c.default_channels, 4);
    }

    #[test]
    fn test_theme_kind_resolution() {
        let mut c = Config::default();
        assert_eq!(c.theme_kind(), ThemeKind::CatppuccinMocha);
        c.theme = "latte".to_string();
        assert_eq!(c.theme_kind(), ThemeKind::CatppuccinLatte);
        c.theme = "nord".to_string();
        assert_eq!(c.theme_kind(), ThemeKind::Nord);
        c.theme = "unknown".to_string();
        assert_eq!(c.theme_kind(), ThemeKind::Dark); // default fallback
    }

    #[test]
    fn test_config_roundtrip_toml() {
        let cfg = Config {
            theme: "nord".to_string(),
            default_bpm: 140.0,
            default_pattern_rows: 32,
            default_channels: 8,
            sample_dirs: vec!["/tmp/samples".to_string()],
        };
        let s = toml::to_string_pretty(&cfg).unwrap();
        let restored: Config = toml::from_str(&s).unwrap();
        assert_eq!(restored.theme, "nord");
        assert_eq!(restored.default_bpm, 140.0);
        assert_eq!(restored.default_pattern_rows, 32);
        assert_eq!(restored.default_channels, 8);
    }
}
