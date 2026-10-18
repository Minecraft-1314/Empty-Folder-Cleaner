use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::error_categories;
use super::localization;
use super::models::{
    AppSettings, SettingsLoadResult, SettingsResetOutcome, SettingsSaveOutcome, SettingsSource,
};
use super::path_guard;

pub const MAX_SETTINGS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_RECENT_DIRECTORIES: usize = 8;
pub const MAX_IGNORED_PATHS: usize = 5000;
pub const MAX_IGNORE_PATTERNS: usize = 200;
const MAX_PATTERN_LENGTH: usize = 260;

pub struct SettingsStore {
    settings_path: PathBuf,
}

impl Default for SettingsStore {
    fn default() -> Self {
        Self::new(None)
    }
}

impl SettingsStore {
    pub fn new(settings_path: Option<PathBuf>) -> Self {
        Self {
            settings_path: settings_path.unwrap_or_else(default_settings_path),
        }
    }

    pub fn settings_path(&self) -> PathBuf {
        self.settings_path.clone()
    }

    pub fn exists(&self) -> bool {
        self.settings_path.is_file()
    }

    pub fn load(&self) -> SettingsLoadResult {
        if !self.settings_path.is_file() {
            return SettingsLoadResult {
                settings: AppSettings::default(),
                source: SettingsSource::None,
                warning_code: None,
            };
        }

        let Some(json) = read_text_within_limit(&self.settings_path) else {
            return self.recover_from_backup();
        };

        let Ok(parsed) = parse_settings(&json) else {
            return self.recover_from_backup();
        };

        SettingsLoadResult {
            settings: normalize(&parsed),
            source: SettingsSource::Json,
            warning_code: None,
        }
    }

    pub fn save(&self, settings: &AppSettings) -> SettingsSaveOutcome {
        let normalized = normalize(settings);
        let Some(directory) = self.settings_path.parent() else {
            return SettingsSaveOutcome {
                saved: false,
                warning_code: Some("settings_path_invalid".to_string()),
                settings_path: self.settings_path.to_string_lossy().to_string(),
            };
        };

        let mut temporary = PathBuf::new();
        let result = (|| -> std::io::Result<()> {
            fs::create_dir_all(directory)?;
            temporary = directory.join(format!("settings-{}.tmp", new_identifier()));
            write_settings(&temporary, &normalized)?;
            self.archive_existing_settings();
            if self.settings_path.is_file() {
                fs::remove_file(&self.settings_path)?;
            }
            fs::rename(&temporary, &self.settings_path)?;
            Ok(())
        })();

        match result {
            Ok(()) => SettingsSaveOutcome {
                saved: true,
                warning_code: None,
                settings_path: self.settings_path.to_string_lossy().to_string(),
            },
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                SettingsSaveOutcome {
                    saved: false,
                    warning_code: Some(error_categories::from_io(&error)),
                    settings_path: self.settings_path.to_string_lossy().to_string(),
                }
            }
        }
    }

    pub fn reset(&self) -> SettingsResetOutcome {
        let cleared_json = try_delete(&self.settings_path);
        try_delete(&self.backup_path());
        SettingsResetOutcome {
            cleared_json,
            warning_code: if cleared_json {
                None
            } else {
                Some("settings_reset_incomplete".to_string())
            },
        }
    }

    fn recover_from_backup(&self) -> SettingsLoadResult {
        match self.try_read_backup() {
            Some(settings) => SettingsLoadResult {
                settings: normalize(&settings),
                source: SettingsSource::Recovered,
                warning_code: Some("settings_recovered_from_backup".to_string()),
            },
            None => SettingsLoadResult {
                settings: AppSettings::default(),
                source: SettingsSource::None,
                warning_code: Some("settings_corrupt".to_string()),
            },
        }
    }

    fn try_read_backup(&self) -> Option<AppSettings> {
        let path = self.backup_path();
        if !path.is_file() {
            return None;
        }

        let json = read_text_within_limit(&path)?;
        parse_settings(&json).ok()
    }

    fn backup_path(&self) -> PathBuf {
        let mut value = self.settings_path.to_string_lossy().to_string();
        value.push_str(".bak");
        PathBuf::from(value)
    }

    fn archive_existing_settings(&self) {
        if !self.settings_path.is_file() {
            return;
        }

        let _ = fs::copy(&self.settings_path, self.backup_path());
    }
}

pub fn default_settings_path() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base)
        .join("EmptyFolderCleaner")
        .join("settings.json")
}

pub fn normalize(settings: &AppSettings) -> AppSettings {
    AppSettings {
        language: normalize_language_setting(&settings.language),
        theme_mode: normalize_theme(&settings.theme_mode),
        use_recycle_bin: settings.use_recycle_bin,
        scan_directory: normalize_directory(Some(&settings.scan_directory)),
        recent_directories: distinct_directories(
            &settings.recent_directories,
            MAX_RECENT_DIRECTORIES,
        ),
        ignored_paths: distinct_directories(&settings.ignored_paths, MAX_IGNORED_PATHS),
        ignore_patterns: normalize_patterns(&settings.ignore_patterns),
    }
}

fn normalize_language_setting(value: &str) -> String {
    if value.trim().eq_ignore_ascii_case(localization::SYSTEM) {
        localization::SYSTEM.to_string()
    } else {
        localization::normalize(Some(value))
    }
}

pub fn normalize_patterns(patterns: &[String]) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for pattern in patterns {
        let trimmed = pattern.trim();
        let length = trimmed.chars().count();
        if length == 0 || length > MAX_PATTERN_LENGTH || trimmed.chars().any(char::is_control) {
            continue;
        }
        if !seen.insert(trimmed.to_lowercase()) {
            continue;
        }
        result.push(trimmed.to_string());
        if result.len() >= MAX_IGNORE_PATTERNS {
            break;
        }
    }
    result
}

pub fn normalize_path_set(paths: &[String]) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for path in paths {
        if let Some(normalized) = path_guard::try_normalize(Some(path)) {
            if seen.insert(normalized.to_lowercase()) {
                result.push(normalized);
            }
        }
    }
    result
}

pub fn distinct_directories(paths: &[String], limit: usize) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for path in paths {
        if result.len() >= limit {
            break;
        }
        if let Some(normalized) = path_guard::try_normalize(Some(path)) {
            if seen.insert(normalized.to_lowercase()) {
                result.push(normalized);
            }
        }
    }
    result
}

fn normalize_directory(value: Option<&str>) -> String {
    path_guard::try_normalize(value).unwrap_or_default()
}

fn normalize_theme(value: &str) -> String {
    match value {
        "light" | "dark" | "system" => value.to_string(),
        _ => String::new(),
    }
}

fn try_delete(path: &Path) -> bool {
    if !path.exists() {
        return true;
    }

    fs::remove_file(path).is_ok()
}

fn read_text_within_limit(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > MAX_SETTINGS_FILE_BYTES {
        return None;
    }

    fs::read_to_string(path).ok()
}

fn write_settings(path: &Path, settings: &AppSettings) -> std::io::Result<()> {
    let value = serde_json::to_string_pretty(settings)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))?;
    fs::write(path, value)
}

pub fn new_identifier() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::SeqCst)
    };
    format!("{nanos:x}{counter:x}")
}

fn parse_settings(json: &str) -> Result<AppSettings, serde_json::Error> {
    let value: Value = serde_json::from_str(json)?;
    let Value::Object(map) = value else {
        return Err(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "settings_root_not_object",
        )));
    };

    let mut settings = AppSettings::default();
    if let Some(value) = text_field(&map, "language") {
        settings.language = value;
    }
    if let Some(value) = text_field(&map, "themeMode") {
        settings.theme_mode = value;
    }
    if let Some(value) = boolean_field(&map, "useRecycleBin") {
        settings.use_recycle_bin = value;
    }
    if let Some(value) = text_field(&map, "scanDirectory") {
        settings.scan_directory = value;
    }
    if let Some(value) = list_field(&map, "recentDirectories") {
        settings.recent_directories = value;
    }
    if let Some(value) = list_field(&map, "ignoredPaths") {
        settings.ignored_paths = value;
    }
    if let Some(value) = list_field(&map, "ignorePatterns") {
        settings.ignore_patterns = value;
    }

    Ok(settings)
}

fn text_field(map: &Map<String, Value>, name: &str) -> Option<String> {
    match map.get(name) {
        Some(Value::String(value)) => Some(value.clone()),
        _ => None,
    }
}

fn boolean_field(map: &Map<String, Value>, name: &str) -> Option<bool> {
    match map.get(name) {
        Some(Value::Bool(value)) => Some(*value),
        _ => None,
    }
}

fn list_field(map: &Map<String, Value>, name: &str) -> Option<Vec<String>> {
    match map.get(name) {
        Some(Value::Array(items)) => Some(
            items
                .iter()
                .filter_map(|item| item.as_str().map(|value| value.to_string()))
                .collect(),
        ),
        _ => None,
    }
}

pub fn describe_error(error: &std::io::Error) -> String {
    error_categories::from_io(error)
}
