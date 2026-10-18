use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::core::cluster_size;
use crate::core::deleter;
use crate::core::error_categories;
use crate::core::localization;
use crate::core::models::{
    AppSettings, DeleteProgress, DeleteRequest, FolderEntry, ScanOptions, ScanStats,
    SettingsResetOutcome,
};
use crate::core::path_guard;
use crate::core::scanner;
use crate::core::settings_store::{self, SettingsStore};

use super::contracts::event;
use super::json as bridge_json;
use super::platform;

const MAX_PARALLEL_SCAN: usize = 8;
const MAX_TRACKED_IGNORED_PATHS: usize = 5000;

pub trait EventSink: Send + Sync {
    fn post(&self, payload: Value);
}

pub struct OperationStart {
    pub accepted: bool,
    pub error_code: Option<String>,
}

struct ActiveOperation {
    id: u64,
    cancel: Arc<AtomicBool>,
}

struct Inner {
    settings: AppSettings,
    load_warning: Option<String>,
    operation: Option<ActiveOperation>,
    next_operation_id: u64,
}

pub struct AppRuntime {
    events: Arc<dyn EventSink>,
    settings_store: SettingsStore,
    inner: Mutex<Inner>,
    save_gate: Mutex<()>,
}

impl AppRuntime {
    pub fn new(events: Arc<dyn EventSink>, settings_store: SettingsStore) -> Self {
        let loaded = settings_store.load();
        Self {
            events,
            settings_store,
            inner: Mutex::new(Inner {
                settings: loaded.settings,
                load_warning: loaded.warning_code,
                operation: None,
                next_operation_id: 1,
            }),
            save_gate: Mutex::new(()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }

    fn raise(&self, kind: &str, payload: Value) {
        self.events.post(event(kind, payload));
    }

    pub fn describe_state(&self) -> Value {
        let (settings, load_warning) = {
            let inner = self.lock();
            (inner.settings.clone(), inner.load_warning.clone())
        };
        let language = {
            let stored = localization::normalize(Some(&settings.language));
            if stored.is_empty() {
                localization::detect()
            } else {
                stored
            }
        };
        let theme = if settings.theme_mode == "light" || settings.theme_mode == "dark" {
            settings.theme_mode.clone()
        } else {
            "system".to_string()
        };

        json!({
            "settings": {
                "language": settings.language,
                "themeMode": settings.theme_mode,
                "useRecycleBin": settings.use_recycle_bin,
                "scanDirectory": settings.scan_directory,
                "recentDirectories": settings.recent_directories,
                "ignoredPaths": settings.ignored_paths,
                "ignorePatterns": settings.ignore_patterns,
            },
            "effectiveLanguage": language,
            "effectiveTheme": theme,
            "isDarkSystemTheme": platform::is_dark_system_theme(),
            "isRecycleBinAvailable": true,
            "loadWarning": load_warning,
            "configExists": self.settings_store.exists(),
            "settingsPath": self.settings_store.settings_path().to_string_lossy().to_string(),
            "version": env!("CARGO_PKG_VERSION"),
            "runtimeVersion": env!("APP_RUST_VERSION"),
            "installPath": install_path(),
        })
    }

    pub fn current_language(&self) -> String {
        let stored = {
            let inner = self.lock();
            localization::normalize(Some(&inner.settings.language))
        };
        if stored.is_empty() {
            localization::detect()
        } else {
            stored
        }
    }

    pub fn save_settings(&self, input: &Value) -> bool {
        let _guard = self
            .save_gate
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        let previous = {
            let mut inner = self.lock();
            let previous = inner.settings.clone();
            let mut updated = inner.settings.clone();
            updated.language = bridge_json::text(input, "language", &updated.language);
            updated.theme_mode = bridge_json::text(input, "themeMode", &updated.theme_mode);
            updated.use_recycle_bin =
                bridge_json::boolean(input, "useRecycleBin", updated.use_recycle_bin);
            updated.scan_directory =
                bridge_json::text(input, "scanDirectory", &updated.scan_directory);
            if let Some(values) = bridge_json::string_list(input, "recentDirectories") {
                updated.recent_directories = values;
            }
            if let Some(values) = bridge_json::string_list(input, "ignoredPaths") {
                updated.ignored_paths = values;
            }
            if let Some(values) = bridge_json::string_list(input, "ignorePatterns") {
                updated.ignore_patterns = values;
            }
            let normalized = settings_store::normalize(&updated);
            inner.settings = normalized.clone();
            (previous, normalized)
        };

        let outcome = self.settings_store.save(&previous.1);
        if !outcome.saved {
            let mut inner = self.lock();
            inner.settings = previous.0;
            drop(inner);
            self.raise(
                "settingsSaveFailed",
                json!({ "warningCode": outcome.warning_code }),
            );
            return false;
        }

        {
            let mut inner = self.lock();
            inner.load_warning = None;
        }
        self.raise(
            "settingsSaved",
            json!({ "settingsPath": outcome.settings_path }),
        );
        true
    }

    pub fn reset_settings(&self) -> SettingsResetOutcome {
        let outcome = self.settings_store.reset();
        {
            let mut inner = self.lock();
            inner.settings = AppSettings::default();
            inner.load_warning = outcome.warning_code.clone();
        }
        self.raise(
            "settingsReset",
            json!({ "warningCode": outcome.warning_code }),
        );
        outcome
    }

    pub fn start_scan(
        self: &Arc<Self>,
        root_directory: Option<&str>,
        ignored_paths: Option<&[String]>,
        ignore_patterns: Option<&[String]>,
    ) -> OperationStart {
        let Some(root) = path_guard::try_normalize(root_directory) else {
            return OperationStart {
                accepted: false,
                error_code: Some(error_categories::INVALID_INPUT.to_string()),
            };
        };
        if !Path::new(&root).is_dir() {
            return OperationStart {
                accepted: false,
                error_code: Some(error_categories::NOT_FOUND.to_string()),
            };
        }

        let ignored = settings_store::normalize_path_set(ignored_paths.unwrap_or(&[]));
        let patterns = settings_store::normalize_patterns(ignore_patterns.unwrap_or(&[]));
        let runtime = self.clone();
        self.start_operation(move |cancel| runtime.run_scan(root, ignored, patterns, cancel))
    }

    pub fn start_delete(
        self: &Arc<Self>,
        paths: Option<&[String]>,
        use_recycle_bin: bool,
    ) -> OperationStart {
        let Some(paths) = paths else {
            return OperationStart {
                accepted: false,
                error_code: Some("no_paths".to_string()),
            };
        };
        if paths.is_empty() {
            return OperationStart {
                accepted: false,
                error_code: Some("no_paths".to_string()),
            };
        }
        if paths.len() > MAX_TRACKED_IGNORED_PATHS {
            return OperationStart {
                accepted: false,
                error_code: Some(error_categories::INVALID_INPUT.to_string()),
            };
        }

        let (root, ignored, patterns) = {
            let inner = self.lock();
            (
                inner.settings.scan_directory.clone(),
                settings_store::normalize_path_set(&inner.settings.ignored_paths),
                settings_store::normalize_patterns(&inner.settings.ignore_patterns),
            )
        };

        let Some(normalized_root) = path_guard::try_normalize(Some(&root)) else {
            return OperationStart {
                accepted: false,
                error_code: Some(error_categories::NOT_FOUND.to_string()),
            };
        };
        if !Path::new(&normalized_root).is_dir() {
            return OperationStart {
                accepted: false,
                error_code: Some(error_categories::NOT_FOUND.to_string()),
            };
        }

        let request = DeleteRequest {
            root_directory: normalized_root,
            paths: paths.to_vec(),
            use_recycle_bin,
            ignored_paths: ignored,
            ignore_patterns: patterns,
        };
        let runtime = self.clone();
        self.start_operation(move |cancel| runtime.run_delete(request, cancel))
    }

    pub fn is_busy(&self) -> bool {
        let inner = self.lock();
        inner.operation.is_some()
    }

    pub fn stop_current_operation(&self) {
        let cancel = {
            let inner = self.lock();
            inner
                .operation
                .as_ref()
                .map(|operation| operation.cancel.clone())
        };
        if let Some(flag) = cancel {
            flag.store(true, Ordering::SeqCst);
        }
    }

    pub fn shutdown(&self, timeout: Duration) {
        let cancel = {
            let inner = self.lock();
            inner
                .operation
                .as_ref()
                .map(|operation| operation.cancel.clone())
        };
        if let Some(flag) = cancel {
            flag.store(true, Ordering::SeqCst);
        }

        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            let running = {
                let inner = self.lock();
                inner.operation.is_some()
            };
            if !running {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        let _guard = self
            .save_gate
            .lock()
            .unwrap_or_else(|error| error.into_inner());
    }

    fn start_operation<F>(self: &Arc<Self>, body: F) -> OperationStart
    where
        F: FnOnce(Arc<AtomicBool>) + Send + 'static,
    {
        let (id, cancel) = {
            let mut inner = self.lock();
            if inner.operation.is_some() {
                return OperationStart {
                    accepted: false,
                    error_code: Some(error_categories::BUSY.to_string()),
                };
            }
            let id = inner.next_operation_id;
            inner.next_operation_id += 1;
            let cancel = Arc::new(AtomicBool::new(false));
            inner.operation = Some(ActiveOperation {
                id,
                cancel: cancel.clone(),
            });
            (id, cancel)
        };

        let runtime = self.clone();
        std::thread::spawn(move || {
            body(cancel);
            runtime.end_operation(id);
        });

        OperationStart {
            accepted: true,
            error_code: None,
        }
    }

    fn end_operation(&self, id: u64) {
        let mut inner = self.lock();
        if let Some(operation) = inner.operation.as_ref() {
            if operation.id == id {
                inner.operation = None;
            }
        }
    }

    fn run_scan(
        self: Arc<Self>,
        root: String,
        ignored: Vec<String>,
        patterns: Vec<String>,
        cancel: Arc<AtomicBool>,
    ) {
        let mut was_cancelled = false;
        self.raise(
            "scanStarted",
            json!({ "root": root, "clusterSize": cluster_size::get_cluster_size(Some(&root)) }),
        );

        let options = ScanOptions {
            root_directory: root,
            ignored_paths: ignored,
            ignore_patterns: patterns,
            max_degree_of_parallelism: MAX_PARALLEL_SCAN,
        };
        let events = self.events.clone();
        let progress_events = self.events.clone();
        let flag = cancel.clone();

        let result = scanner::scan(
            &options,
            cancel,
            move |batch: &[FolderEntry]| {
                events.post(event("scanBatch", json!({ "entries": batch })));
            },
            move |stats: &ScanStats| {
                progress_events.post(event("scanProgress", json!(stats)));
            },
        );

        match result {
            Ok(report) => {
                was_cancelled = report.was_cancelled;
                self.raise(
                    "scanCompleted",
                    json!({ "stats": report.stats, "wasCancelled": report.was_cancelled }),
                );
            }
            Err(error) => {
                if flag.load(Ordering::SeqCst) {
                    was_cancelled = true;
                } else {
                    self.raise("operationFailed", json!({ "code": error.message() }));
                }
            }
        }

        if was_cancelled {
            self.raise("scanStopped", json!({}));
        } else {
            self.raise("scanFinished", json!({}));
        }
    }

    fn run_delete(self: Arc<Self>, request: DeleteRequest, cancel: Arc<AtomicBool>) {
        self.raise("deleteStarted", json!({ "total": request.paths.len() }));
        let events = self.events.clone();
        let progress_events = self.events.clone();

        let result = deleter::delete(
            &request,
            cancel.clone(),
            move |item| {
                events.post(event("deleteEvent", json!(item)));
            },
            move |progress: &DeleteProgress| {
                progress_events.post(event("deleteProgress", json!(progress)));
            },
        );

        match result {
            Ok(report) => {
                self.remember_failed_paths(&report.failed_paths);
                self.raise(
                    "deleteCompleted",
                    json!({
                        "deletedPaths": report.deleted_paths,
                        "failedPaths": report.failed_paths,
                        "remainingPaths": report.remaining_paths,
                        "wasCancelled": report.was_cancelled,
                    }),
                );
            }
            Err(error) => {
                if cancel.load(Ordering::SeqCst) {
                    self.raise(
                        "deleteCompleted",
                        json!({
                            "deletedPaths": Vec::<String>::new(),
                            "failedPaths": Vec::<String>::new(),
                            "remainingPaths": request.paths,
                            "wasCancelled": true,
                        }),
                    );
                } else {
                    self.raise("operationFailed", json!({ "code": error.message() }));
                }
            }
        }
    }

    fn remember_failed_paths(&self, paths: &[String]) {
        let mut inner = self.lock();
        let mut merged: Vec<String> = inner.settings.ignored_paths.clone();
        let mut seen: std::collections::HashSet<String> =
            merged.iter().map(|value| value.to_lowercase()).collect();
        for path in paths {
            if let Some(normalized) = path_guard::try_normalize(Some(path)) {
                if seen.insert(normalized.to_lowercase()) {
                    merged.push(normalized);
                }
            }
        }
        merged.truncate(MAX_TRACKED_IGNORED_PATHS);
        inner.settings.ignored_paths = merged;
    }
}

fn install_path() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.parent()
                .map(|value| value.to_string_lossy().to_string())
        })
        .unwrap_or_default()
}
