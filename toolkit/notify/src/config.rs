use std::{
    env, fmt, fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, Once, OnceLock},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    atomic_write,
    error::{AppError, Result},
    model::NotificationMode,
};

pub const DEFAULT_APP_BUNDLE: &str = "com.googlecode.iterm2";
pub const DEFAULT_SOUND: &str = "default";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warning,
    Error,
    Critical,
}

impl LogLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
            Self::Critical => "CRITICAL",
        }
    }

    pub const fn tracing_filter(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warn",
            Self::Error | Self::Critical => "error",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_uppercase().as_str() {
            "DEBUG" => Ok(Self::Debug),
            "INFO" => Ok(Self::Info),
            "WARNING" => Ok(Self::Warning),
            "ERROR" => Ok(Self::Error),
            "CRITICAL" => Ok(Self::Critical),
            _ => Err("log level must be one of DEBUG, INFO, WARNING, ERROR, CRITICAL".to_owned()),
        }
    }
}

impl Serialize for LogLevel {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?.parse().map_err(de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct NotificationConfig {
    pub app_bundle: String,
    pub mode: NotificationMode,
    pub sound: String,
    pub threshold_seconds: u64,
    pub exclude_patterns: Vec<String>,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            app_bundle: DEFAULT_APP_BUNDLE.to_owned(),
            mode: NotificationMode::All,
            sound: DEFAULT_SOUND.to_owned(),
            threshold_seconds: 10,
            exclude_patterns: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self { path: config_dir().join("ai-notify.db") }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct CleanupConfig {
    pub auto_cleanup_enabled: bool,
    pub export_before_cleanup: bool,
    pub retention_days: u32,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self { auto_cleanup_enabled: true, export_before_cleanup: true, retention_days: 30 }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub path: PathBuf,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self { level: LogLevel::Info, path: config_dir().join("ai-notify.log") }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct AppConfig {
    pub cleanup: CleanupConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
    pub notification: NotificationConfig,
}

impl AppConfig {
    pub fn validate(&self) -> Result<()> {
        if self.cleanup.retention_days == 0 {
            return Err(AppError::configuration("cleanup.retention_days must be at least 1"));
        }
        Ok(())
    }

    fn expand_paths(&mut self) {
        self.database.path = expand_tilde(&self.database.path);
        self.logging.path = expand_tilde(&self.logging.path);
    }

    fn collapsed_paths(&self) -> Self {
        let mut config = self.clone();
        config.database.path = collapse_home(&config.database.path);
        config.logging.path = collapse_home(&config.logging.path);
        config
    }

    pub fn commented_yaml(&self) -> Result<String> {
        self.validate()?;
        let yaml = serde_yaml_ng::to_string(&self.collapsed_paths())?;
        Ok(add_comments(&yaml))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigSource {
    File,
    Defaults,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigWarning {
    pub path: PathBuf,
    pub message: String,
}

impl fmt::Display for ConfigWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "failed to load config from {}: {}; using defaults", self.path.display(), self.message)
    }
}

#[derive(Clone, Debug)]
pub struct LoadedConfig {
    pub config: Arc<AppConfig>,
    pub source: ConfigSource,
    pub warning: Option<ConfigWarning>,
}

impl LoadedConfig {
    pub fn emit_warning(&self) {
        if let Some(warning) = &self.warning {
            tracing::warn!("{warning}");
        }
    }
}

#[derive(Debug)]
pub struct ConfigLoader {
    path: PathBuf,
    loaded: OnceLock<LoadedConfig>,
    warned: Once,
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new(config_path())
    }
}

impl ConfigLoader {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), loaded: OnceLock::new(), warned: Once::new() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Arc<AppConfig> {
        let loaded = self.load_report();
        self.warned.call_once(|| loaded.emit_warning());
        loaded.config
    }

    pub fn load_report(&self) -> LoadedConfig {
        self.loaded.get_or_init(|| load_path(&self.path)).clone()
    }

    pub fn save(&self, config: &AppConfig) -> Result<()> {
        atomic_write::replace(&self.path, config.commented_yaml()?.as_bytes())
    }

    pub fn reset_to_defaults(&self) -> Result<AppConfig> {
        let config = AppConfig::default();
        self.save(&config)?;
        Ok(config)
    }
}

static RUNTIME_CONFIG: OnceLock<Arc<AppConfig>> = OnceLock::new();

/// Returns the process-wide runtime configuration, loaded at most once.
pub fn runtime_config() -> Arc<AppConfig> {
    Arc::clone(RUNTIME_CONFIG.get_or_init(|| ConfigLoader::default().load()))
}

pub fn xdg_config_home() -> PathBuf {
    xdg_config_home_from(env::var_os("XDG_CONFIG_HOME"), env::var_os("HOME"))
}

pub fn config_dir() -> PathBuf {
    xdg_config_home().join("ai-notify")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.yaml")
}

pub fn export_dir() -> PathBuf {
    config_dir().join("exports")
}

pub fn cleanup_marker_path() -> PathBuf {
    config_dir().join(".last_cleanup")
}

pub fn expand_tilde(path: &Path) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path.to_path_buf();
    };
    let Some(home) = home_dir() else {
        return path.to_path_buf();
    };
    if text == "~" {
        return home;
    }
    text.strip_prefix("~/").map_or_else(|| path.to_path_buf(), |suffix| home.join(suffix))
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").filter(|value| !value.is_empty()).map(PathBuf::from)
}

fn xdg_config_home_from(xdg: Option<std::ffi::OsString>, home: Option<std::ffi::OsString>) -> PathBuf {
    xdg.filter(|value| !value.is_empty()).map(PathBuf::from).unwrap_or_else(|| {
        home.filter(|value| !value.is_empty()).map(PathBuf::from).unwrap_or_default().join(".config")
    })
}

fn collapse_home(path: &Path) -> PathBuf {
    let Some(home) = home_dir() else {
        return path.to_path_buf();
    };
    match path.strip_prefix(&home) {
        Ok(relative) if relative.as_os_str().is_empty() => PathBuf::from("~"),
        Ok(relative) => PathBuf::from("~").join(relative),
        Err(_) => path.to_path_buf(),
    }
}

fn load_path(path: &Path) -> LoadedConfig {
    let result = fs::read_to_string(path)
        .map_err(|error| error.to_string())
        .and_then(|text| {
            serde_yaml_ng::from_str::<Option<AppConfig>>(&text)
                .map(Option::unwrap_or_default)
                .map_err(|error| error.to_string())
        })
        .and_then(|mut config| {
            config.expand_paths();
            config.validate().map(|()| config).map_err(|error| error.to_string())
        });

    match result {
        Ok(config) => LoadedConfig { config: Arc::new(config), source: ConfigSource::File, warning: None },
        Err(message) => LoadedConfig {
            config: Arc::new(AppConfig::default()),
            source: ConfigSource::Defaults,
            warning: Some(ConfigWarning { path: path.to_path_buf(), message }),
        },
    }
}

fn add_comments(yaml: &str) -> String {
    let mut output = String::with_capacity(yaml.len() + 768);
    let mut section = "";
    for line in yaml.lines() {
        if !line.starts_with(' ') {
            section = line.strip_suffix(':').unwrap_or("");
        }
        output.push_str(line);
        if let Some(comment) = field_comment(section, line) {
            output.push_str("  # ");
            output.push_str(comment);
        }
        output.push('\n');
    }
    output
}

fn field_comment(section: &str, line: &str) -> Option<&'static str> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    let key = line.trim_start().split_once(':')?.0;
    match (section, indent, key) {
        ("cleanup", 2, "auto_cleanup_enabled") => Some("Enable automatic cleanup of old data"),
        ("cleanup", 2, "export_before_cleanup") => Some("Export data before cleanup"),
        ("cleanup", 2, "retention_days") => {
            Some("Number of days to retain session data (older data will be auto-cleaned)")
        }
        ("database", 2, "path") => Some("Path to SQLite database file"),
        ("logging", 2, "level") => Some("Log level (DEBUG, INFO, WARNING, ERROR, CRITICAL)"),
        ("logging", 2, "path") => Some("Path to log file"),
        ("notification", 2, "app_bundle") => Some("Application bundle ID to focus on notification click"),
        ("notification", 2, "mode") => Some("Notification mode: 'all' (default), 'permission_only', or 'disabled'"),
        ("notification", 2, "sound") => Some("Notification sound (see /System/Library/Sounds for options)"),
        ("notification", 2, "threshold_seconds") => {
            Some("Minimum job duration in seconds to trigger notification (0 = notify all)")
        }
        ("notification", 2, "exclude_patterns") => {
            Some("List of prompt prefixes to exclude from notifications (case-sensitive)")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn missing_config_reports_warning_and_uses_defaults() {
        let directory = tempdir().unwrap();
        let loader = ConfigLoader::new(directory.path().join("missing.yaml"));

        let loaded = loader.load_report();

        assert_eq!(loaded.source, ConfigSource::Defaults);
        assert!(loaded.warning.unwrap().to_string().contains("using defaults"));
        assert_eq!(loaded.config.notification.threshold_seconds, 10);
    }

    #[test]
    fn xdg_config_home_prefers_nonempty_xdg_and_falls_back_to_home() {
        assert_eq!(xdg_config_home_from(Some("/tmp/xdg".into()), Some("/tmp/home".into())), PathBuf::from("/tmp/xdg"));
        assert_eq!(xdg_config_home_from(Some("".into()), Some("/tmp/home".into())), PathBuf::from("/tmp/home/.config"));
    }

    #[test]
    fn empty_yaml_uses_defaults_without_a_warning() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.yaml");
        fs::write(&path, "").unwrap();

        let loaded = ConfigLoader::new(path).load_report();

        assert_eq!(loaded.source, ConfigSource::File);
        assert!(loaded.warning.is_none());
        assert_eq!(loaded.config.notification.threshold_seconds, 10);
    }

    #[test]
    fn partial_yaml_uses_defaults_and_expands_paths() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.yaml");
        fs::write(
            &path,
            "notification:\n  threshold_seconds: 15\nlogging:\n  level: warning\n  path: ~/logs/ai-notify.log\ndatabase:\n  path: ~/state/ai-notify.db\n",
        )
        .unwrap();

        let config = ConfigLoader::new(path).load();

        assert_eq!(config.notification.threshold_seconds, 15);
        assert_eq!(config.notification.mode, NotificationMode::All);
        assert_eq!(config.logging.level, LogLevel::Warning);
        if let Some(home) = home_dir() {
            assert_eq!(config.logging.path, home.join("logs/ai-notify.log"));
            assert_eq!(config.database.path, home.join("state/ai-notify.db"));
        }
    }

    #[test]
    fn malformed_and_invalid_yaml_fall_back_as_a_whole() {
        let directory = tempdir().unwrap();
        let malformed = directory.path().join("malformed.yaml");
        let invalid = directory.path().join("invalid.yaml");
        fs::write(&malformed, "cleanup: [").unwrap();
        fs::write(&invalid, "cleanup:\n  retention_days: 0\nnotification:\n  threshold_seconds: 99\n").unwrap();

        for path in [malformed, invalid] {
            let loaded = ConfigLoader::new(path).load_report();
            assert_eq!(loaded.source, ConfigSource::Defaults);
            assert!(loaded.warning.is_some());
            assert_eq!(loaded.config.notification.threshold_seconds, 10);
        }
    }

    #[test]
    fn save_writes_canonical_commented_yaml_and_round_trips() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("nested/config.yaml");
        let loader = ConfigLoader::new(&path);
        let mut config = AppConfig::default();
        config.database.path = PathBuf::from("/tmp/custom.db");
        config.logging.path = PathBuf::from("/tmp/custom.log");
        config.logging.level = LogLevel::Debug;
        config.notification.exclude_patterns = vec!["/skip".to_owned()];

        loader.save(&config).unwrap();
        let text = fs::read_to_string(&path).unwrap();

        assert!(text.contains("retention_days: 30  # Number of days"));
        assert!(text.contains("level: DEBUG  # Log level"));
        assert!(text.contains("path: /tmp/custom.db  # Path to SQLite database file"));
        assert!(text.contains("path: /tmp/custom.log  # Path to log file"));
        assert_eq!(&*ConfigLoader::new(path).load(), &config);
    }

    #[test]
    fn invalid_log_level_falls_back_to_defaults() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.yaml");
        fs::write(&path, "logging:\n  level: verbose\n").unwrap();

        let loaded = ConfigLoader::new(path).load_report();

        assert_eq!(loaded.source, ConfigSource::Defaults);
        assert_eq!(loaded.config.logging.level, LogLevel::Info);
    }
}
