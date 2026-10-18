use std::sync::Arc;

use serde_json::{json, Value};

use crate::core::error::AppError;
use crate::core::error_categories;

use super::contracts::{BridgeException, BridgeRequest};
use super::json as bridge_json;
use super::platform;
use super::runtime::AppRuntime;

pub const ACTION_GET_STATE: &str = "app.getState";
pub const ACTION_GET_SYSTEM_THEME: &str = "app.getSystemTheme";
pub const ACTION_CHOOSE_DIRECTORY: &str = "app.chooseDirectory";
pub const ACTION_START_SCAN: &str = "scan.start";
pub const ACTION_STOP_OPERATION: &str = "operation.stop";
pub const ACTION_DELETE_FOLDERS: &str = "delete.start";
pub const ACTION_SAVE_SETTINGS: &str = "settings.save";
pub const ACTION_RESET_SETTINGS: &str = "settings.reset";
pub const ACTION_OPEN_FOLDER: &str = "shell.openFolder";
pub const ACTION_COPY_PATH: &str = "shell.copyPath";

pub struct CommandRouter {
    runtime: Arc<AppRuntime>,
}

impl CommandRouter {
    pub fn new(runtime: Arc<AppRuntime>) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &Arc<AppRuntime> {
        &self.runtime
    }

    pub fn execute(&self, action: &str, payload: &Value) -> Result<Value, BridgeException> {
        match action {
            ACTION_GET_STATE => Ok(self.runtime.describe_state()),
            ACTION_GET_SYSTEM_THEME => Ok(json!({ "isDark": platform::is_dark_system_theme() })),
            ACTION_CHOOSE_DIRECTORY => {
                Ok(platform::choose_directory(&self.runtime.current_language()))
            }
            ACTION_START_SCAN => self.start_scan(payload),
            ACTION_STOP_OPERATION => {
                self.runtime.stop_current_operation();
                Ok(json!({ "accepted": true }))
            }
            ACTION_DELETE_FOLDERS => self.start_delete(payload),
            ACTION_SAVE_SETTINGS => Ok(self.save_settings(payload)),
            ACTION_RESET_SETTINGS => Ok(describe_reset(&self.runtime.reset_settings())),
            ACTION_OPEN_FOLDER => self.open_folder(payload),
            ACTION_COPY_PATH => self.copy_path(payload),
            _ => Err(BridgeException::new("unknown_action", &["action"])),
        }
    }

    fn start_scan(&self, payload: &Value) -> Result<Value, BridgeException> {
        let root = bridge_json::text(payload, "root", "");
        let ignored = bridge_json::string_list(payload, "ignoredPaths");
        let patterns = bridge_json::string_list(payload, "ignorePatterns");
        let started = self
            .runtime
            .start_scan(Some(&root), ignored.as_deref(), patterns.as_deref());
        if !started.accepted {
            return Err(BridgeException::new(
                started
                    .error_code
                    .unwrap_or_else(|| error_categories::BUSY.to_string()),
                &["root"],
            ));
        }

        Ok(json!({ "accepted": true }))
    }

    fn start_delete(&self, payload: &Value) -> Result<Value, BridgeException> {
        let paths = bridge_json::string_list(payload, "paths");
        let use_recycle_bin = bridge_json::boolean(payload, "useRecycleBin", true);
        let started = self.runtime.start_delete(paths.as_deref(), use_recycle_bin);
        if !started.accepted {
            return Err(BridgeException::new(
                started
                    .error_code
                    .unwrap_or_else(|| error_categories::BUSY.to_string()),
                &["paths"],
            ));
        }

        Ok(json!({ "accepted": true }))
    }

    fn save_settings(&self, payload: &Value) -> Value {
        let saved = self.runtime.save_settings(payload);
        json!({
            "saved": saved,
            "warning": if saved { Value::Null } else { Value::String("settings_save_failed".to_string()) },
        })
    }

    fn open_folder(&self, payload: &Value) -> Result<Value, BridgeException> {
        let path = bridge_json::raw_text(payload);
        platform::open_folder(path.as_deref()).map_err(|error| bridge_error(&error, &["path"]))
    }

    fn copy_path(&self, payload: &Value) -> Result<Value, BridgeException> {
        let path = bridge_json::raw_text(payload);
        platform::copy_path(path.as_deref()).map_err(|error| bridge_error(&error, &["path"]))
    }
}

fn bridge_error(error: &AppError, fields: &[&str]) -> BridgeException {
    BridgeException::new(error.message(), fields)
}

fn describe_reset(outcome: &crate::core::models::SettingsResetOutcome) -> Value {
    json!({
        "cleared": outcome.cleared_json,
        "warningCode": outcome.warning_code,
        "settings": {
            "language": "",
            "themeMode": "",
            "useRecycleBin": true,
            "scanDirectory": "",
            "recentDirectories": Vec::<String>::new(),
            "ignoredPaths": Vec::<String>::new(),
            "ignorePatterns": Vec::<String>::new(),
        },
    })
}

pub fn parse_request(body: &str, max_characters: usize) -> Result<BridgeRequest, BridgeException> {
    if body.chars().count() > max_characters {
        return Err(BridgeException::new("payload_too_large", &["request"]));
    }

    let value: Value = serde_json::from_str(body)
        .map_err(|_| BridgeException::new("invalid_request", &["request"]))?;
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    let action = value
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if action.trim().is_empty() {
        return Err(BridgeException::new("invalid_request", &["request"]));
    }

    let payload = value.get("payload").cloned().unwrap_or(Value::Null);
    Ok(BridgeRequest {
        id,
        action,
        payload,
    })
}
