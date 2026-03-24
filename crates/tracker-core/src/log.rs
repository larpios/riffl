use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Logger {
    file: Mutex<Option<std::fs::File>>,
    level: Mutex<LogLevel>,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum LogLevel {
    Error,
    #[default]
    Warn,
    Info,
    Debug,
}

impl LogLevel {
    fn from_env() -> Self {
        match std::env::var("RIFFL_LOG").ok().as_deref() {
            Some("error") => LogLevel::Error,
            Some("warn") => LogLevel::Warn,
            Some("info") => LogLevel::Info,
            Some("debug") => LogLevel::Debug,
            _ => LogLevel::Warn,
        }
    }
}

static LOGGER: Logger = Logger {
    file: Mutex::new(None),
    level: Mutex::new(LogLevel::Warn),
};

impl Logger {
    pub fn init() -> std::io::Result<()> {
        let log_dir = get_log_dir()?;
        fs::create_dir_all(&log_dir)?;

        let log_file = get_log_file_path(&log_dir);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;

        let mut guard = LOGGER.file.lock().unwrap();
        *guard = Some(file);

        drop(guard);

        let mut level_guard = LOGGER.level.lock().unwrap();
        *level_guard = LogLevel::from_env();

        Ok(())
    }

    pub fn log(&self, level: LogLevel, module: &str, message: &str) {
        if !self.should_log(level) {
            return;
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let level_str = match level {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN ",
            LogLevel::Info => "INFO ",
            LogLevel::Debug => "DEBUG",
        };

        let line = format!("[{}] [{}] [{}] {}\n", timestamp, level_str, module, message);

        if let Ok(mut guard) = self.file.lock() {
            if let Some(ref mut file) = *guard {
                let _ = file.write_all(line.as_bytes());
                let _ = file.flush();
            }
        }

        eprintln!("{}", line.trim());
    }

    fn should_log(&self, level: LogLevel) -> bool {
        let guard = self.level.lock().unwrap();
        level as u8 >= *guard as u8
    }
}

fn get_log_dir() -> std::io::Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("riffl")
        .join("logs");
    Ok(config_dir)
}

fn get_log_file_path(log_dir: &Path) -> PathBuf {
    let date = chrono::Local::now().format("%Y%m%d");
    log_dir.join(format!("riffl-{}.log", date))
}

pub fn error(module: &str, message: &str) {
    LOGGER.log(LogLevel::Error, module, message);
}

pub fn warn(module: &str, message: &str) {
    LOGGER.log(LogLevel::Warn, module, message);
}

pub fn info(module: &str, message: &str) {
    LOGGER.log(LogLevel::Info, module, message);
}

pub fn debug(module: &str, message: &str) {
    LOGGER.log(LogLevel::Debug, module, message);
}

#[macro_export]
macro_rules! log {
    ($level:expr, $module:expr, $($arg:tt)*) => {{
        let msg = format!($($arg)*);
        match $level {
            "error" => $crate::log::error($module, &msg),
            "warn" => $crate::log::warn($module, &msg),
            "info" => $crate::log::info($module, &msg),
            "debug" => $crate::log::debug($module, &msg),
            _ => $crate::log::warn($module, &msg),
        }
    }};
}

pub fn init() -> std::io::Result<()> {
    Logger::init()
}
