use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
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

pub struct Logger {
    file: Mutex<Option<std::fs::File>>,
    level: Mutex<LogLevel>,
}

static LOGGER: Logger = Logger {
    file: Mutex::new(None),
    level: Mutex::new(LogLevel::Warn),
};

pub fn init() -> std::io::Result<()> {
    Logger::init()
}

impl Logger {
    pub fn init() -> std::io::Result<()> {
        let log_dir = crate::metadata::log_dir().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine log directory",
            )
        })?;
        std::fs::create_dir_all(&log_dir)?;
        let log_file_path = log_dir.join("log.txt");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)?;

        if let Ok(mut guard) = LOGGER.file.lock() {
            *guard = Some(file);
        }

        if let Ok(mut level_guard) = LOGGER.level.lock() {
            *level_guard = LogLevel::from_env();
        }

        Ok(())
    }

    pub fn instance() -> &'static Logger {
        &LOGGER
    }

    pub fn log(&self, level: LogLevel, module: &str, message: &str) {
        if !self.should_log(level) {
            return;
        }

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
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
    }

    pub fn should_log(&self, level: LogLevel) -> bool {
        if let Ok(logger_level) = self.level.lock() {
            level >= *logger_level
        } else {
            false
        }
    }
}

#[macro_export]
macro_rules! log_error {
    ($module:expr, $($arg:tt)*) => {
        $crate::log::Logger::instance().log($crate::log::LogLevel::Error, $module, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_warn {
    ($module:expr, $($arg:tt)*) => {
        $crate::log::Logger::instance().log($crate::log::LogLevel::Warn, $module, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_info {
    ($module:expr, $($arg:tt)*) => {
        $crate::log::Logger::instance().log($crate::log::LogLevel::Info, $module, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_debug {
    ($module:expr, $($arg:tt)*) => {
        $crate::log::Logger::instance().log($crate::log::LogLevel::Debug, $module, &format!($($arg)*));
    };
}
