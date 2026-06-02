use std::{
    fmt,
    fs::{File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    str::FromStr,
    sync::{Mutex, OnceLock},
};

#[cfg(test)]
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Warn
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => f.write_str("info"),
            Self::Warn => f.write_str("warn"),
            Self::Error => f.write_str("error"),
        }
    }
}

impl FromStr for LogLevel {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            _ => anyhow::bail!("unsupported log level {value:?}; expected info, warn, or error"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LoggerConfig {
    pub level: LogLevel,
    pub file_path: Option<PathBuf>,
}

pub struct Logger {
    level: LogLevel,
    sink: LogSink,
    file: Option<Mutex<File>>,
}

enum LogSink {
    Stderr,
    #[cfg(test)]
    Memory(Arc<Mutex<Vec<u8>>>),
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub fn init(config: LoggerConfig) -> anyhow::Result<()> {
    LOGGER
        .set(Logger::new(config)?)
        .map_err(|_| anyhow::anyhow!("logger has already been initialized"))
}

#[allow(dead_code)]
pub fn global() -> Option<&'static Logger> {
    LOGGER.get()
}

pub fn info(message: impl AsRef<str>) {
    if let Some(logger) = global() {
        logger.info(message);
    }
}

pub fn warn(message: impl AsRef<str>) {
    if let Some(logger) = global() {
        logger.warn(message);
    }
}

pub fn error(message: impl AsRef<str>) {
    if let Some(logger) = global() {
        logger.error(message);
    }
}

impl Logger {
    pub fn new(config: LoggerConfig) -> anyhow::Result<Self> {
        let file = match config.file_path {
            Some(path) => Some(Mutex::new(
                OpenOptions::new().create(true).append(true).open(path)?,
            )),
            None => None,
        };

        Ok(Self {
            level: config.level,
            sink: LogSink::Stderr,
            file,
        })
    }

    #[cfg(test)]
    fn new_for_test(config: LoggerConfig, output: Arc<Mutex<Vec<u8>>>) -> anyhow::Result<Self> {
        let mut logger = Self::new(config)?;
        logger.sink = LogSink::Memory(output);
        Ok(logger)
    }

    pub fn info(&self, message: impl AsRef<str>) {
        self.log(LogLevel::Info, message.as_ref());
    }

    pub fn warn(&self, message: impl AsRef<str>) {
        self.log(LogLevel::Warn, message.as_ref());
    }

    pub fn error(&self, message: impl AsRef<str>) {
        self.log(LogLevel::Error, message.as_ref());
    }

    pub fn log(&self, level: LogLevel, message: &str) {
        if level < self.level {
            return;
        }

        let line = format!("[{level}] {message}\n");
        if let Err(error) = self.write_line(&line) {
            eprintln!("failed to write log message: {error}");
        }
    }

    fn write_line(&self, line: &str) -> io::Result<()> {
        match &self.sink {
            LogSink::Stderr => {
                let mut stderr = io::stderr().lock();
                stderr.write_all(line.as_bytes())?;
                stderr.flush()?;
            }
            #[cfg(test)]
            LogSink::Memory(output) => {
                output
                    .lock()
                    .expect("test logger output lock poisoned")
                    .write_all(line.as_bytes())?;
            }
        }

        if let Some(file) = &self.file {
            let mut file = file.lock().expect("logger file lock poisoned");
            file.write_all(line.as_bytes())?;
            file.flush()?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
