use std::fs::{self, OpenOptions};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt::time::ChronoLocal, prelude::*};

use crate::{
    config::{AppConfig, LoadedConfig},
    error::{AppError, Result},
};

#[derive(Debug)]
pub struct LoggingGuard(Option<WorkerGuard>);

impl LoggingGuard {
    pub const fn disabled() -> Self {
        Self(None)
    }

    pub const fn is_enabled(&self) -> bool {
        self.0.is_some()
    }
}

/// Installs process logging at the YAML-configured path and level.
///
/// Keep the returned guard alive until process exit so short-lived hook invocations flush
/// their final records. `AI_NOTIFY_LOG=0|false|no|off` disables logging.
pub fn init(config: &AppConfig) -> Result<LoggingGuard> {
    if logging_disabled() {
        return Ok(LoggingGuard::disabled());
    }

    let path = &config.logging.path;
    let parent =
        path.parent().filter(|value| !value.as_os_str().is_empty()).unwrap_or_else(|| std::path::Path::new("."));
    fs::create_dir_all(parent)?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| AppError::operational(format!("cannot open log file {}: {error}", path.display())))?;
    let (writer, guard) = tracing_appender::non_blocking(file);
    let filter = EnvFilter::new(config.logging.level.tracing_filter());
    let subscriber = tracing_subscriber::registry().with(filter).with(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_target(true)
            .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S".to_owned()))
            .with_writer(writer),
    );
    subscriber.try_init().map_err(|error| AppError::operational(format!("cannot initialize logging: {error}")))?;
    Ok(LoggingGuard(Some(guard)))
}

/// Initializes logging and records any fallback diagnostic produced while loading YAML.
pub fn init_loaded(loaded: &LoadedConfig) -> Result<LoggingGuard> {
    let guard = init(&loaded.config)?;
    loaded.emit_warning();
    Ok(guard)
}

fn logging_disabled() -> bool {
    std::env::var("AI_NOTIFY_LOG")
        .ok()
        .is_some_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "0" | "false" | "no" | "off"))
}
