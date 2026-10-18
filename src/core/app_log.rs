use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use regex::Regex;

use super::timestamp;

const MAX_LOG_BYTES: u64 = 512 * 1024;
const MAX_MESSAGE_CHARACTERS: usize = 12_000;

fn writer() -> &'static Mutex<Option<std::fs::File>> {
    static WRITER: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();
    WRITER.get_or_init(|| Mutex::new(None))
}

pub fn file_path() -> PathBuf {
    log_directory().join("app.log")
}

pub fn log_directory() -> PathBuf {
    match std::env::var("LOCALAPPDATA") {
        Ok(value) if !value.trim().is_empty() => PathBuf::from(value).join("EmptyFolderCleaner"),
        _ => std::env::temp_dir(),
    }
}

pub fn bind() {
    let mut guard = writer().lock().unwrap_or_else(|error| error.into_inner());
    if guard.is_some() {
        return;
    }

    let directory = log_directory();
    if fs::create_dir_all(&directory).is_err() {
        return;
    }

    let path = directory.join("app.log");
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(path) {
        *guard = Some(file);
    }
}

pub fn error(message: &str) {
    write("ERROR", message, None);
}

pub fn error_with_detail(message: &str, detail: &str) {
    write("ERROR", message, Some(detail));
}

fn write(level: &str, message: &str, detail: Option<&str>) {
    let mut text = format!("{} [{level}] {message}", timestamp::format_local_datetime());
    if let Some(value) = detail {
        if !value.is_empty() {
            text.push('\r');
            text.push('\n');
            text.push_str(value);
        }
    }

    let line = redact(&text);
    let mut guard = writer().lock().unwrap_or_else(|error| error.into_inner());
    if guard.is_none() {
        return;
    }

    let path = file_path();
    if fs::metadata(&path)
        .map(|metadata| metadata.len() > MAX_LOG_BYTES)
        .unwrap_or(false)
    {
        let _ = fs::remove_file(&path);
    }

    if let Some(file) = guard.as_mut() {
        if writeln!(file, "{line}").is_err() {
            let fallback = std::env::temp_dir().join("empty-folder-cleaner.log");
            if path != fallback {
                if let Ok(file) = OpenOptions::new().create(true).append(true).open(&fallback) {
                    *guard = Some(file);
                    if let Some(file) = guard.as_mut() {
                        let _ = writeln!(file, "{line}");
                    }
                }
            }
        }
    }
}

pub fn redact(value: &str) -> String {
    static BEARER: OnceLock<Regex> = OnceLock::new();
    static NAMED: OnceLock<Regex> = OnceLock::new();
    static QUERY: OnceLock<Regex> = OnceLock::new();

    let bearer = BEARER.get_or_init(|| {
        Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]+").expect("valid bearer pattern")
    });
    let named = NAMED.get_or_init(|| {
        Regex::new(
            r#"(?i)\b(authorization|cookie|set-cookie|password|passwd|pwd|access[_-]?token|refresh[_-]?token|token|secret|api[_-]?key)("?\s*[:=]\s*"?)[^\s";,}]+"#,
        )
        .expect("valid named pattern")
    });
    let query = QUERY.get_or_init(|| {
        Regex::new(
            r"(?i)([?&](?:authorization|cookie|password|passwd|pwd|access[_-]?token|refresh[_-]?token|token|secret|api[_-]?key|code|state)=)[^&\s]+",
        )
        .expect("valid query pattern")
    });

    let result = bearer.replace_all(value, "Bearer [REDACTED]").to_string();
    let result = query.replace_all(&result, "$1[REDACTED]").to_string();
    let result = named.replace_all(&result, "$1$2[REDACTED]").to_string();
    if result.chars().count() > MAX_MESSAGE_CHARACTERS {
        let head: String = result.chars().take(MAX_MESSAGE_CHARACTERS).collect();
        format!("{head}...")
    } else {
        result
    }
}
