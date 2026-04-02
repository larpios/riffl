use std::path::PathBuf;

/// The name of the application.
pub const APP_NAME: &str = "riffl";

/// Return the platform-specific config directory for this application.
///
/// - Unix/macOS: `$XDG_CONFIG_HOME/riffl` or `~/.config/riffl`
/// - Windows:    `%APPDATA%\riffl`
pub fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .ok()
            .map(PathBuf::from)
            .map(|p| p.join(APP_NAME))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let base = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg)
        } else {
            std::env::home_dir()?.join(".config")
        };
        Some(base.join(APP_NAME))
    }
}

/// Return the platform-specific data directory for this application.
///
/// - Unix/macOS: `$XDG_DATA_HOME/riffl` or `~/.local/share/riffl`
/// - Windows:    `%APPDATA%\riffl`
pub fn data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .ok()
            .map(PathBuf::from)
            .map(|p| p.join(APP_NAME))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let base = if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            PathBuf::from(xdg)
        } else {
            std::env::home_dir()?.join(".local").join("share")
        };
        Some(base.join(APP_NAME))
    }
}

/// Return the platform-specific log directory for this application.
///
/// This is set to the `logs` subdirectory of the data directory.
pub fn log_dir() -> Option<PathBuf> {
    data_dir().map(|p| p.join("logs"))
}

/// Search upward from the current directory for the workspace root.
pub fn find_project_root() -> Option<PathBuf> {
    let mut current_dir = std::env::current_dir().ok()?;
    loop {
        if current_dir.join("Cargo.toml").exists() {
            // Check if it's the workspace root by looking for [workspace] in Cargo.toml
            if let Ok(content) = std::fs::read_to_string(current_dir.join("Cargo.toml")) {
                if content.contains("[workspace]") {
                    return Some(current_dir);
                }
            }
        }
        if !current_dir.pop() {
            break;
        }
    }
    None
}
