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

    /// Default playback mode for new sessions
    pub default_playback_mode: riffl_core::transport::PlaybackMode,

    /// Default loop state for new sessions
    pub default_loop_enabled: bool,

    /// Additional sample directories shown in the sample browser.
    /// `~/.config/riffl/samples/` is always included automatically.
    /// Also overridden/extended by RIFFL_SAMPLE_DIR env var or --sample-dir CLI flag.
    #[serde(default)]
    pub sample_dirs: Vec<String>,

    /// Additional module directories shown in the module browser.
    /// `~/.config/riffl/modules/` is always included automatically.
    #[serde(default)]
    pub module_dirs: Vec<String>,

    /// User-bookmarked directories shown at the top of the sample browser roots list.
    /// Populated when the user presses `b` on a directory in the sample browser.
    #[serde(default)]
    pub bookmarked_dirs: Vec<String>,

    /// Status bar configuration - controls what information is displayed in the footer.
    #[serde(default)]
    pub status_bar: StatusBarConfig,

    /// Default follow mode for new sessions: cursor chases playhead during playback
    #[serde(default)]
    pub default_follow_mode: bool,

    /// Default row advance step size for note entry (0 = no advance, 1-8 typical)
    #[serde(default = "default_step_size")]
    pub default_step_size: usize,

    /// Default octave for note entry (0-9, default 4)
    #[serde(default = "default_octave")]
    pub default_octave: u8,

    /// Number of rows per beat for row highlight intervals.
    /// Rows at multiples of this value are highlighted (dimly).
    /// Rows at multiples of 4× this value are highlighted more strongly.
    /// Default is 4 (quarter-note beats at 4/4 time).
    #[serde(default = "default_beat_rows")]
    pub beat_rows: u8,

    /// Autosave interval in seconds. Set to 0 to disable autosave.
    /// When > 0, the project is silently saved every N seconds if there are unsaved changes.
    #[serde(default)]
    pub autosave_interval_secs: u64,

    /// External file picker to use when opening files (Ctrl+F).
    ///
    /// Accepted values:
    ///   "auto"    — try yazi first, fall back to the built-in browser (default)
    ///   "builtin" — always use the built-in overlay browser
    ///   "yazi"    — always use yazi (no fallback)
    ///   "<name>"  — run `<name> --chooser-file <tmpfile> <dir>` (same protocol as yazi)
    #[serde(default = "default_file_picker")]
    pub file_picker: String,
}

fn default_file_picker() -> String {
    "auto".to_string()
}
fn default_step_size() -> usize {
    1
}
fn default_octave() -> u8 {
    4
}
fn default_beat_rows() -> u8 {
    4
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "mocha".to_string(),
            default_bpm: 125.0,
            default_pattern_rows: 16,
            default_channels: 4,
            default_playback_mode: riffl_core::transport::PlaybackMode::Song,
            default_loop_enabled: false,
            sample_dirs: Vec::new(),
            module_dirs: Vec::new(),
            bookmarked_dirs: Vec::new(),
            status_bar: StatusBarConfig::default(),
            default_follow_mode: false,
            default_step_size: default_step_size(),
            default_octave: default_octave(),
            beat_rows: default_beat_rows(),
            autosave_interval_secs: 0,
            file_picker: default_file_picker(),
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

    /// Return the application config directory (platform-aware).
    pub fn config_dir() -> std::path::PathBuf {
        get_config_dir()
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

    /// Return the default modules directory (`~/.config/riffl/modules/`).
    pub fn default_modules_dir() -> std::path::PathBuf {
        Self::config_dir().join("modules")
    }

    /// Resolve all module directories for the browser.
    ///
    /// Order: default modules dir, then config `module_dirs`.
    /// Duplicates are removed. Directories that don't exist are kept.
    pub fn resolve_module_dirs(&self) -> Vec<std::path::PathBuf> {
        let mut dirs: Vec<std::path::PathBuf> = Vec::new();

        // 1. Always include the default modules dir
        dirs.push(Self::default_modules_dir());

        // 2. Dirs from config file
        for p in &self.module_dirs {
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
        Some(get_config_dir().join("config.toml"))
    }
}

/// Configuration for the status bar (footer) display.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusBarConfig {
    /// Show play state (Playing/Stopped/Paused)
    pub show_play_state: bool,
    /// Show current pattern number and row position (CH:ROW)
    pub show_pattern_row: bool,
    /// Show instrument count
    pub show_instrument_count: bool,
    /// Show CPU usage percentage
    pub show_cpu: bool,
    /// Show memory usage percentage
    pub show_memory: bool,
    /// Show selection info (selection start/end)
    pub show_selection: bool,
}

impl Default for StatusBarConfig {
    fn default() -> Self {
        Self {
            show_play_state: true,
            show_pattern_row: true,
            show_instrument_count: true,
            show_cpu: true,
            show_memory: true,
            show_selection: true,
        }
    }
}

/// Application name — the binary name from the [[bin]] section of Cargo.toml.
pub const APP_NAME: &str = env!("CARGO_BIN_NAME");

/// Return the platform-specific config directory for this application.
///
/// - Linux/BSD: `$XDG_CONFIG_HOME/<app>` or `~/.config/<app>`
/// - macOS:     `~/Library/Application Support/<app>`
/// - Windows:   `%APPDATA%\<app>`
pub fn get_config_dir() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(APP_NAME)
    }
    #[cfg(target_os = "macos")]
    {
        home_dir()
            .join("Library")
            .join("Application Support")
            .join(APP_NAME)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            std::path::PathBuf::from(xdg)
        } else {
            home_dir().join(".config")
        }
        .join(APP_NAME)
    }
}

/// Return the platform-specific data directory for this application.
///
/// - Linux/BSD: `$XDG_DATA_HOME/<app>` or `~/.local/share/<app>`
/// - macOS:     `~/Library/Application Support/<app>`
/// - Windows:   `%APPDATA%\<app>`
pub fn get_data_dir() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(APP_NAME)
    }
    #[cfg(target_os = "macos")]
    {
        home_dir()
            .join("Library")
            .join("Application Support")
            .join(APP_NAME)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            std::path::PathBuf::from(xdg)
        } else {
            home_dir().join(".local").join("share")
        }
        .join(APP_NAME)
    }
}

/// Return the user's home directory.
fn home_dir() -> std::path::PathBuf {
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
        c.theme = "mocha".to_string();
        assert_eq!(c.theme_kind(), ThemeKind::CatppuccinMocha);
        c.theme = "nord".to_string();
        assert_eq!(c.theme_kind(), ThemeKind::Nord);
        c.theme = "unknown".to_string();
        assert_eq!(c.theme_kind(), ThemeKind::Dark); // default fallback
    }

    #[test]
    fn test_bookmarked_dirs_toml_roundtrip() {
        let cfg = Config {
            bookmarked_dirs: vec!["/tmp/fav1".to_string(), "/tmp/fav2".to_string()],
            ..Config::default()
        };
        let s = toml::to_string_pretty(&cfg).unwrap();
        let restored: Config = toml::from_str(&s).unwrap();
        assert_eq!(
            restored.bookmarked_dirs,
            vec!["/tmp/fav1".to_string(), "/tmp/fav2".to_string()]
        );
    }

    #[test]
    fn test_bookmarked_dirs_default_empty() {
        let cfg = Config::default();
        assert!(cfg.bookmarked_dirs.is_empty());
    }

    #[test]
    fn test_config_new_fields_defaults() {
        let c = Config::default();
        assert!(!c.default_follow_mode);
        assert_eq!(c.default_step_size, 1);
        assert_eq!(c.default_octave, 4);
        assert_eq!(c.beat_rows, 4);
        assert_eq!(c.autosave_interval_secs, 0);
    }

    #[test]
    fn test_config_roundtrip_toml() {
        let cfg = Config {
            theme: "nord".to_string(),
            default_bpm: 140.0,
            default_pattern_rows: 32,
            default_channels: 8,
            default_playback_mode: riffl_core::transport::PlaybackMode::Song,
            default_loop_enabled: false,
            sample_dirs: vec!["/tmp/samples".to_string()],
            module_dirs: vec!["/tmp/modules".to_string()],
            bookmarked_dirs: vec![],
            status_bar: StatusBarConfig::default(),
            default_follow_mode: false,
            default_step_size: 1,
            default_octave: 4,
            beat_rows: 4,
            autosave_interval_secs: 0,
            file_picker: "builtin".to_string(),
        };
        let s = toml::to_string_pretty(&cfg).unwrap();
        let restored: Config = toml::from_str(&s).unwrap();
        assert_eq!(restored.theme, "nord");
        assert_eq!(restored.default_bpm, 140.0);
        assert_eq!(restored.default_pattern_rows, 32);
        assert_eq!(restored.default_channels, 8);
        assert_eq!(
            restored.default_playback_mode,
            riffl_core::transport::PlaybackMode::Song
        );
        assert_eq!(restored.default_loop_enabled, false);
    }

    #[test]
    fn test_status_bar_config_default() {
        let sb = StatusBarConfig::default();
        assert!(sb.show_play_state);
        assert!(sb.show_pattern_row);
        assert!(sb.show_instrument_count);
        assert!(sb.show_cpu);
        assert!(sb.show_memory);
        assert!(sb.show_selection);
    }

    #[test]
    fn test_status_bar_config_toml_roundtrip() {
        let sb = StatusBarConfig {
            show_play_state: false,
            show_pattern_row: true,
            show_instrument_count: false,
            show_cpu: true,
            show_memory: false,
            show_selection: true,
        };
        let s = toml::to_string_pretty(&sb).unwrap();
        let restored: StatusBarConfig = toml::from_str(&s).unwrap();
        assert_eq!(restored.show_play_state, false);
        assert_eq!(restored.show_pattern_row, true);
        assert_eq!(restored.show_instrument_count, false);
        assert_eq!(restored.show_cpu, true);
        assert_eq!(restored.show_memory, false);
        assert_eq!(restored.show_selection, true);
    }
}
