use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderEntry {
    pub path: String,
    pub modified_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStats {
    pub visited_directories: i64,
    pub empty_folders: i64,
    pub elapsed: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanReport {
    pub stats: ScanStats,
    pub was_cancelled: bool,
}

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub root_directory: String,
    pub ignored_paths: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub max_degree_of_parallelism: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeleteOutcome {
    Deleted,
    MovedToRecycleBin,
    NotEmpty,
    AlreadyGone,
    OutsideRoot,
    PermissionDenied,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeleteStage {
    Started,
    Finished,
    CascadeCandidate,
    BatchStopped,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteEvent {
    pub stage: DeleteStage,
    pub outcome: DeleteOutcome,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteProgress {
    pub completed: i64,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct DeleteRequest {
    pub root_directory: String,
    pub paths: Vec<String>,
    pub use_recycle_bin: bool,
    pub ignored_paths: Vec<String>,
    pub ignore_patterns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteReport {
    pub deleted_paths: Vec<String>,
    pub failed_paths: Vec<String>,
    pub remaining_paths: Vec<String>,
    pub was_cancelled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub language: String,
    pub theme_mode: String,
    pub use_recycle_bin: bool,
    pub scan_directory: String,
    pub recent_directories: Vec<String>,
    pub ignored_paths: Vec<String>,
    pub ignore_patterns: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: String::new(),
            theme_mode: String::new(),
            use_recycle_bin: true,
            scan_directory: String::new(),
            recent_directories: Vec::new(),
            ignored_paths: Vec::new(),
            ignore_patterns: Vec::new(),
        }
    }
}

impl AppSettings {
    pub fn has_stored_values(&self) -> bool {
        !self.language.is_empty()
            || !self.theme_mode.is_empty()
            || !self.scan_directory.is_empty()
            || !self.recent_directories.is_empty()
            || !self.ignored_paths.is_empty()
            || !self.ignore_patterns.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSource {
    None,
    Json,
    Recovered,
}

#[derive(Debug, Clone)]
pub struct SettingsLoadResult {
    pub settings: AppSettings,
    pub source: SettingsSource,
    pub warning_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSaveOutcome {
    pub saved: bool,
    pub warning_code: Option<String>,
    pub settings_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsResetOutcome {
    pub cleared_json: bool,
    pub warning_code: Option<String>,
}
