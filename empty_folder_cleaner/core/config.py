"""Application constants and system locale resolution."""

import locale
import os
import re
import sys

APP_ORG = "EmptyFolderCleaner"
APP_NAME = "Settings"
DEFAULT_CLUSTER_SIZE = 4096
MAX_RECENT_DIRS = 5
MAX_DELETE_PREVIEW = 20
LOG_MAX_BLOCKS = 500
THEME_POLL_INTERVAL_MS = 2000
THREAD_QUIT_TIMEOUT_MS = 5000
TABLE_UPDATE_BATCH = 100
TABLE_FLUSH_INTERVAL_MS = 120
CLOSE_RETRY_INTERVAL_MS = 250
CLOSE_MAX_RETRIES = 40
STATUS_UPDATE_INTERVAL = 0.25
SCAN_MAX_WORKERS = 8
MIN_WINDOW_WIDTH = 1000
MIN_WINDOW_HEIGHT = 680
FONT_FAMILY = "Segoe UI"
LANG_COMBO_WIDTH = 110
TABLE_TIME_WIDTH = 150
TABLE_STATUS_WIDTH = 120
TABLE_SELECT_WIDTH = 80
IGNORE_DIALOG_MIN_WIDTH = 400
LOG_ERROR_COLOR = "#e53935"
CHINESE_LOCALE_PATTERN = re.compile(r"^(zh|cmn|zho)", re.IGNORECASE)


def get_system_language():
    """Detect a Chinese UI locale without mutating process-wide locale state."""
    candidates = []
    for getter in (
        lambda: locale.getlocale(locale.LC_MESSAGES)[0],
        lambda: locale.getlocale(locale.LC_CTYPE)[0],
    ):
        try:
            candidates.append(getter())
        except Exception:
            candidates.append(None)
    candidates.extend(
        os.environ.get(name)
        for name in ("LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG")
    )
    if sys.platform == "win32":
        candidates.append(_windows_ui_language())
    return "zh" if any(CHINESE_LOCALE_PATTERN.match(c or "") for c in candidates) else "en"


def _windows_ui_language():
    try:
        import ctypes

        lang_id = ctypes.windll.kernel32.GetUserDefaultUILanguage()
        return locale.windows_locale.get(lang_id, "")
    except Exception:
        return ""


def as_str_list(value):
    """Normalize a QSettings value into a list of non-empty strings.

    QSettings returns a bare str instead of a list when a single-element list is
    round-tripped through some backends, which would otherwise make callers
    iterate a string character by character.
    """
    if value is None:
        return []
    if isinstance(value, str):
        return [value] if value else []
    if isinstance(value, (list, tuple, set)):
        return [str(item) for item in value if item]
    return []
