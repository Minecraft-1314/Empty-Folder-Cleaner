use windows_sys::Win32::Globalization::GetSystemDefaultUILanguage;

pub const CHINESE: &str = "zh-CN";
pub const ENGLISH: &str = "en-US";
pub const SYSTEM: &str = "system";

pub const KEY_TITLE: &str = "title";
pub const KEY_SUBTITLE: &str = "subtitle";
pub const KEY_STARTING: &str = "starting";
pub const KEY_RETRY: &str = "retry";
pub const KEY_STOPPING: &str = "stopping";
pub const KEY_WAITING_FOR_TASK: &str = "waiting_for_task";
pub const KEY_FALLBACK_MISSING_FILES: &str = "fallback_missing_files";
pub const KEY_FALLBACK_MISSING_RUNTIME: &str = "fallback_missing_runtime";
pub const KEY_FALLBACK_START_FAILED: &str = "fallback_start_failed";
pub const KEY_SELECT_FOLDER: &str = "select_folder";
pub const KEY_FOLDER_MISSING: &str = "folder_missing";
pub const KEY_PATH_EMPTY: &str = "path_empty";
pub const KEY_UNEXPECTED_ERROR: &str = "unexpected_error";
pub const KEY_UNEXPECTED_ERROR_DETAIL: &str = "unexpected_error_detail";
pub const KEY_CANCEL: &str = "cancel";
pub const KEY_CONFIRM: &str = "confirm";

const KEYS: [&str; 16] = [
    KEY_TITLE,
    KEY_SUBTITLE,
    KEY_STARTING,
    KEY_RETRY,
    KEY_STOPPING,
    KEY_WAITING_FOR_TASK,
    KEY_FALLBACK_MISSING_FILES,
    KEY_FALLBACK_MISSING_RUNTIME,
    KEY_FALLBACK_START_FAILED,
    KEY_SELECT_FOLDER,
    KEY_FOLDER_MISSING,
    KEY_PATH_EMPTY,
    KEY_UNEXPECTED_ERROR,
    KEY_UNEXPECTED_ERROR_DETAIL,
    KEY_CANCEL,
    KEY_CONFIRM,
];

const CHINESE_TEXTS: [&str; 16] = [
    "空文件夹清理器",
    "查找并删除电脑中的空文件夹",
    "正在启动本地界面...",
    "重试",
    "正在停止",
    "正在等待当前任务结束...",
    "本地界面文件缺失。请重新安装程序或恢复 Ui/Web 目录后重试。",
    "未检测到 Microsoft Edge WebView2 运行时。请安装 Evergreen WebView2 Runtime 后重试。",
    "本地界面启动失败。请检查 WebView2 运行时后重试，期间未修改任何文件。",
    "选择文件夹",
    "该文件夹已不存在。",
    "路径为空。",
    "程序遇到意外问题，已恢复到安全状态，未修改任何文件。",
    "详细信息已写入日志。",
    "取消",
    "确定",
];

const ENGLISH_TEXTS: [&str; 16] = [
    "Empty Folder Cleaner",
    "Find and delete empty folders on your computer",
    "Starting the local interface...",
    "Retry",
    "Stopping",
    "Waiting for the current task to stop...",
    "The local interface files are missing. Reinstall the application or restore the Ui/Web folder, then retry.",
    "Microsoft Edge WebView2 Runtime was not found. Install the Evergreen WebView2 Runtime, then retry.",
    "The local interface could not start. Check the WebView2 Runtime and retry. No files were changed.",
    "Select a folder",
    "This folder no longer exists.",
    "The path is empty.",
    "Empty Folder Cleaner hit an unexpected problem and recovered to a safe state. No files were changed.",
    "Details were written to the local log.",
    "Cancel",
    "OK",
];

pub fn resolve(language: &str, key: &str) -> String {
    let Some(index) = KEYS.iter().position(|candidate| *candidate == key) else {
        return key.to_string();
    };

    let table = if normalize(Some(language)) == CHINESE {
        CHINESE_TEXTS
    } else {
        ENGLISH_TEXTS
    };
    table[index].to_string()
}

pub fn is_supported(language: Option<&str>) -> bool {
    match language {
        Some(value) => value.trim().eq_ignore_ascii_case(SYSTEM) || find(Some(value)).is_some(),
        None => false,
    }
}

pub fn normalize(language: Option<&str>) -> String {
    find(language).unwrap_or_else(detect)
}

pub fn detect() -> String {
    let primary = unsafe { GetSystemDefaultUILanguage() };
    let name = language_tag(primary);
    if is_chinese_culture(&name) {
        CHINESE.to_string()
    } else {
        ENGLISH.to_string()
    }
}

pub fn language_tag(language_identifier: u16) -> String {
    const TAGS: [(u16, &str); 26] = [
        (0x0404, "zh-TW"),
        (0x0409, "en-US"),
        (0x0804, "zh-CN"),
        (0x0C04, "zh-HK"),
        (0x1004, "zh-SG"),
        (0x0407, "de-DE"),
        (0x040C, "fr-FR"),
        (0x0410, "it-IT"),
        (0x0411, "ja-JP"),
        (0x0412, "ko-KR"),
        (0x0416, "pt-BR"),
        (0x0419, "ru-RU"),
        (0x040A, "es-ES"),
        (0x0413, "nl-NL"),
        (0x041D, "sv-SE"),
        (0x0414, "nb-NO"),
        (0x0406, "da-DK"),
        (0x040B, "fi-FI"),
        (0x0405, "cs-CZ"),
        (0x0415, "pl-PL"),
        (0x041F, "tr-TR"),
        (0x040E, "hu-HU"),
        (0x0418, "ro-RO"),
        (0x0408, "el-GR"),
        (0x040D, "he-IL"),
        (0x0422, "uk-UA"),
    ];
    for (identifier, tag) in TAGS {
        if identifier == language_identifier {
            return tag.to_string();
        }
    }

    String::new()
}

fn find(language: Option<&str>) -> Option<String> {
    let value = language?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    for candidate in [CHINESE, ENGLISH] {
        if candidate.eq_ignore_ascii_case(trimmed) {
            return Some(candidate.to_string());
        }
    }

    let primary = trimmed.split(['-', '+']).next().unwrap_or_default();
    for candidate in [CHINESE, ENGLISH] {
        let candidate_primary = candidate.split(['-', '+']).next().unwrap_or_default();
        if candidate_primary.eq_ignore_ascii_case(primary) {
            return Some(candidate.to_string());
        }
    }

    None
}

fn is_chinese_culture(name: &str) -> bool {
    name.starts_with("zh") || name.eq_ignore_ascii_case("cmn")
}
