import os
import sys
import time
import locale
import threading
import fnmatch

from PyQt6.QtCore import (
    Qt, QThread, pyqtSignal, QStandardPaths, QSettings, QSortFilterProxyModel, QTimer
)
from PyQt6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QLabel, QLineEdit, QPushButton, QProgressBar, QTableWidget,
    QTableWidgetItem, QHeaderView, QPlainTextEdit, QComboBox,
    QFileDialog, QCheckBox, QAbstractItemView, QMenu, QDialog,
    QDialogButtonBox, QListWidget, QMessageBox, QStyleFactory,
    QStatusBar, QFrame, QSplitter
)
from PyQt6.QtGui import QFont, QColor, QPalette, QAction, QKeySequence, QShortcut

try:
    import darkdetect
    _DARKDETECT_AVAILABLE = True
except ImportError:
    _DARKDETECT_AVAILABLE = False

try:
    import send2trash
    _SEND2TRASH_AVAILABLE = True
except ImportError:
    _SEND2TRASH_AVAILABLE = False


class I18n:
    translations = {
        "en": {
            "title": "Empty Folder Cleaner",
            "subtitle": "Find and delete empty folders on your computer",
            "scan_dir": "Scan Directory",
            "browse": "Browse",
            "folder_list": "Empty Folders",
            "path": "Path",
            "modified": "Modified",
            "status": "Status",
            "select": "Select",
            "deleted": "Deleted",
            "not_deleted": "Pending",
            "select_all": "Select All",
            "deselect_all": "Deselect All",
            "start_delete": "Delete Selected",
            "stop": "Stop",
            "stop_scan": "Stop Scan",
            "refresh": "Refresh",
            "overall_progress": "Overall Progress",
            "log": "Log",
            "no_folder_selected": "No folders selected for deletion.",
            "delete_running": "Deletion task is already running.",
            "scan_running": "Scan task is already running.",
            "error_scan": "Failed to scan directory: {error}",
            "deleting": "Deleting {path} ...",
            "deleted_msg": "{path} deleted.",
            "deleted_trash_msg": "{path} moved to trash.",
            "skipped": "{path} is not empty, skipped.",
            "stopped": "Deletion stopped.",
            "scan_stopped": "Scan stopped.",
            "all_done": "All selected folders deleted.",
            "select_dir": "Select Directory",
            "language": "Language",
            "theme": "Theme",
            "dark": "Dark",
            "light": "Light",
            "system": "System",
            "zh": "中文",
            "en": "English",
            "ready": "Ready",
            "selected_count": "Selected: {count}",
            "dialog_error": "Error",
            "dialog_warning": "Warning",
            "dialog_info": "Information",
            "not_found": "No empty folders found.",
            "ignored_folder": "Folder ignored (could not delete previously): {path}",
            "scanning": "Scanning... found {count} empty folders",
            "permission_denied": "Permission denied: {path}",
            "use_recycle": "Move to Trash/Recycle Bin",
            "ignore_rules": "Ignore Rules",
            "add_rule": "Add Rule",
            "remove_rule": "Remove Selected Rule",
            "rule_placeholder": "e.g. *cache*",
            "confirm_delete_title": "Confirm Deletion",
            "confirm_delete_text": "Are you sure you want to delete the following {count} folders?\n\n{folder_list}",
            "open_folder": "Open Folder Location",
            "copy_path": "Copy Path",
            "ignore_this": "Ignore This Folder",
            "scan_stats": "Scanned {total} folders, found {empty} empty folders in {elapsed:.1f}s",
            "space_usage": "Estimated space freed: {size}",
            "settings": "Settings",
            "manage_rules": "Manage Ignore Rules...",
            "clear_ignored": "Clear Ignored List",
            "no_rules": "No ignore rules defined.",
            "sort_ascending": "Sort Ascending",
            "sort_descending": "Sort Descending",
        },
        "zh": {
            "title": "空文件夹清理器",
            "subtitle": "查找并删除电脑中的空文件夹",
            "scan_dir": "扫描目录",
            "browse": "浏览",
            "folder_list": "空文件夹列表",
            "path": "路径",
            "modified": "修改时间",
            "status": "状态",
            "select": "选择",
            "deleted": "已删除",
            "not_deleted": "未删除",
            "select_all": "全选",
            "deselect_all": "取消全选",
            "start_delete": "删除选中",
            "stop": "停止",
            "stop_scan": "停止扫描",
            "refresh": "刷新",
            "overall_progress": "总进度",
            "log": "日志",
            "no_folder_selected": "没有需要删除的文件夹。",
            "delete_running": "删除任务已在运行中。",
            "scan_running": "扫描任务已在运行中。",
            "error_scan": "扫描目录失败: {error}",
            "deleting": "正在删除 {path} ...",
            "deleted_msg": "{path} 已删除。",
            "deleted_trash_msg": "{path} 已移至回收站。",
            "skipped": "{path} 非空，跳过。",
            "stopped": "删除已停止。",
            "scan_stopped": "扫描已停止。",
            "all_done": "所有选中文件夹已删除。",
            "select_dir": "选择目录",
            "language": "语言",
            "theme": "主题",
            "dark": "暗色",
            "light": "明亮",
            "system": "跟随系统",
            "zh": "中文",
            "en": "English",
            "ready": "就绪",
            "selected_count": "已选: {count}",
            "dialog_error": "错误",
            "dialog_warning": "警告",
            "dialog_info": "提示",
            "not_found": "未发现空文件夹。",
            "ignored_folder": "已忽略文件夹（之前无法删除）: {path}",
            "scanning": "正在扫描... 已发现 {count} 个空文件夹",
            "permission_denied": "权限不足: {path}",
            "use_recycle": "移动到回收站",
            "ignore_rules": "忽略规则",
            "add_rule": "添加规则",
            "remove_rule": "删除选中规则",
            "rule_placeholder": "例: *cache*",
            "confirm_delete_title": "确认删除",
            "confirm_delete_text": "确定要删除以下 {count} 个文件夹吗？\n\n{folder_list}",
            "open_folder": "打开文件夹位置",
            "copy_path": "复制路径",
            "ignore_this": "忽略此文件夹",
            "scan_stats": "已扫描 {total} 个文件夹，发现 {empty} 个空文件夹，耗时 {elapsed:.1f} 秒",
            "space_usage": "预估释放空间: {size}",
            "settings": "设置",
            "manage_rules": "管理忽略规则...",
            "clear_ignored": "清除忽略列表",
            "no_rules": "未定义忽略规则。",
            "sort_ascending": "升序排序",
            "sort_descending": "降序排序",
        }
    }

    @classmethod
    def get_text(cls, key, lang="en", **kwargs):
        text = cls.translations.get(lang, cls.translations["en"]).get(key, key)
        if kwargs:
            text = text.format(**kwargs)
        return text


def get_system_language():
    try:
        lang, _ = locale.getdefaultlocale()
        if lang and lang.startswith("zh"):
            return "zh"
    except Exception:
        pass
    return "en"


def get_cluster_size(path):
    try:
        if sys.platform == "win32":
            import ctypes
            sectors_per_cluster = ctypes.c_ulonglong()
            bytes_per_sector = ctypes.c_ulonglong()
            free = ctypes.c_ulonglong()
            total = ctypes.c_ulonglong()
            drive = os.path.splitdrive(path)[0] + "\\"
            ctypes.windll.kernel32.GetDiskFreeSpaceW(
                ctypes.c_wchar_p(drive),
                ctypes.pointer(sectors_per_cluster),
                ctypes.pointer(bytes_per_sector),
                ctypes.pointer(free),
                ctypes.pointer(total)
            )
            return sectors_per_cluster.value * bytes_per_sector.value
        else:
            stat = os.statvfs(path)
            return stat.f_frsize
    except:
        return 4096


def format_size(size):
    if size < 1024:
        return f"{size} B"
    elif size < 1024 * 1024:
        return f"{size / 1024:.1f} KB"
    elif size < 1024 * 1024 * 1024:
        return f"{size / (1024 * 1024):.1f} MB"
    else:
        return f"{size / (1024 * 1024 * 1024):.2f} GB"


class EmptyFolderEngine:
    def __init__(self, scan_dir, lang="en"):
        self.scan_dir = scan_dir
        self.lang = lang
        self.stop_event = threading.Event()
        self.ignored_paths = set()
        self.ignore_patterns = []

    def _t(self, key, **kwargs):
        return I18n.get_text(key, self.lang, **kwargs)

    def set_ignored_paths(self, paths):
        if sys.platform == "win32":
            self.ignored_paths = {p.lower() for p in paths}
        else:
            self.ignored_paths = set(paths)

    def set_ignore_patterns(self, patterns):
        self.ignore_patterns = patterns

    def _is_ignored(self, norm_path):
        if sys.platform == "win32" and norm_path.lower() in self.ignored_paths:
            return True
        elif sys.platform != "win32" and norm_path in self.ignored_paths:
            return True
        name = os.path.basename(norm_path)
        for pat in self.ignore_patterns:
            if fnmatch.fnmatch(name, pat):
                return True
        return False

    def scan_empty_folders_generator(self, stats_callback=None):
        if not os.path.isdir(self.scan_dir):
            raise Exception(self._t("error_scan", error="Directory does not exist"))
        norm_scan_dir = os.path.normpath(self.scan_dir)
        total_dirs = 0
        empty_dirs = 0
        for root, dirs, files in os.walk(self.scan_dir, topdown=False):
            if self.stop_event.is_set():
                break
            norm_root = os.path.normpath(root)
            if norm_root == norm_scan_dir:
                continue
            total_dirs += 1
            if self._is_ignored(norm_root):
                continue
            try:
                if not os.listdir(root):
                    empty_dirs += 1
                    yield norm_root
            except PermissionError:
                continue
        if stats_callback:
            stats_callback(total_dirs, empty_dirs)

    def delete_folders(self, folder_paths, log_signal, progress_signal, use_trash=False):
        total = len(folder_paths)
        self.stop_event.clear()
        failed = []

        for idx, path in enumerate(folder_paths, 1):
            if self.stop_event.is_set():
                log_signal.emit(self._t("stopped"), False)
                break

            log_signal.emit(self._t("deleting", path=path), False)
            try:
                if os.path.exists(path) and not os.listdir(path):
                    if use_trash and _SEND2TRASH_AVAILABLE:
                        send2trash.send2trash(path)
                        log_signal.emit(self._t("deleted_trash_msg", path=path), False)
                    else:
                        os.rmdir(path)
                        log_signal.emit(self._t("deleted_msg", path=path), False)
                else:
                    log_signal.emit(self._t("skipped", path=path), False)
            except PermissionError:
                failed.append(path)
                log_signal.emit(self._t("permission_denied", path=path), True)
            except Exception as e:
                failed.append(path)
                log_signal.emit(f"Error deleting {path}: {str(e)}", True)

            progress_signal.emit(idx, total)

        log_signal.emit(self._t("all_done"), False)
        progress_signal.emit(0, 0)
        return failed

    def stop(self):
        self.stop_event.set()


class ScanThread(QThread):
    folder_found = pyqtSignal(str)
    finished_scan = pyqtSignal()
    error_occurred = pyqtSignal(str)
    stats_ready = pyqtSignal(int, int)

    def __init__(self, engine):
        super().__init__()
        self.engine = engine

    def run(self):
        try:
            for path in self.engine.scan_empty_folders_generator(
                stats_callback=lambda total, empty: self.stats_ready.emit(total, empty)
            ):
                self.folder_found.emit(path)
            self.finished_scan.emit()
        except Exception as e:
            self.error_occurred.emit(str(e))


class DeleteThread(QThread):
    log_signal = pyqtSignal(str, bool)
    progress_signal = pyqtSignal(int, int)
    finished_signal = pyqtSignal(list)
    error_signal = pyqtSignal(str)

    def __init__(self, engine, folder_list, use_trash):
        super().__init__()
        self.engine = engine
        self.folder_list = folder_list
        self.use_trash = use_trash

    def run(self):
        try:
            failed = self.engine.delete_folders(
                self.folder_list,
                self.log_signal,
                self.progress_signal,
                use_trash=self.use_trash
            )
            self.finished_signal.emit(failed)
        except Exception as e:
            self.error_signal.emit(str(e))


class IgnoreRulesDialog(QDialog):
    def __init__(self, parent, patterns, lang):
        super().__init__(parent)
        self.lang = lang
        self.setWindowTitle(I18n.get_text("ignore_rules", self.lang))
        self.setMinimumWidth(400)
        layout = QVBoxLayout(self)

        self.list_widget = QListWidget()
        self.list_widget.addItems(patterns)
        layout.addWidget(self.list_widget)

        input_layout = QHBoxLayout()
        self.rule_input = QLineEdit()
        self.rule_input.setPlaceholderText(I18n.get_text("rule_placeholder", self.lang))
        add_btn = QPushButton(I18n.get_text("add_rule", self.lang))
        add_btn.clicked.connect(self.add_rule)
        input_layout.addWidget(self.rule_input, 1)
        input_layout.addWidget(add_btn)
        layout.addLayout(input_layout)

        btn_layout = QHBoxLayout()
        remove_btn = QPushButton(I18n.get_text("remove_rule", self.lang))
        remove_btn.clicked.connect(self.remove_rule)
        btn_layout.addWidget(remove_btn)
        btn_layout.addStretch()
        layout.addLayout(btn_layout)

        button_box = QDialogButtonBox(QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel)
        button_box.accepted.connect(self.accept)
        button_box.rejected.connect(self.reject)
        layout.addWidget(button_box)

    def add_rule(self):
        text = self.rule_input.text().strip()
        if text:
            self.list_widget.addItem(text)
            self.rule_input.clear()

    def remove_rule(self):
        for item in self.list_widget.selectedItems():
            self.list_widget.takeItem(self.list_widget.row(item))

    def get_patterns(self):
        return [self.list_widget.item(i).text() for i in range(self.list_widget.count())]


class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.settings = QSettings("EmptyFolderCleaner", "Settings")
        self.lang = self.settings.value("language", get_system_language())
        self.theme_mode = self.settings.value("theme_mode", "system")
        self.theme_dark = self.is_system_dark() if self.theme_mode == "system" else self.settings.value("dark_mode", False, bool)

        scan_path = self.settings.value("scan_dir", "")
        if not scan_path or not os.path.isdir(scan_path):
            scan_path = QStandardPaths.writableLocation(QStandardPaths.StandardLocation.HomeLocation)
            if not scan_path or not os.path.isdir(scan_path):
                scan_path = os.path.expanduser('~')

        self.recent_dirs = self.settings.value("recent_dirs", [])
        if not self.recent_dirs:
            self.recent_dirs = [scan_path]
        if scan_path not in self.recent_dirs:
            self.recent_dirs.insert(0, scan_path)
        self.recent_dirs = self.recent_dirs[:5]

        self.ignored_paths_raw = set(self.settings.value("ignored_paths", []))
        self.ignore_patterns = self.settings.value("ignore_patterns", [])
        self.use_trash = self.settings.value("use_trash", True, bool)

        self.engine = EmptyFolderEngine(scan_dir=scan_path, lang=self.lang)
        self.engine.set_ignored_paths(self.ignored_paths_raw)
        self.engine.set_ignore_patterns(self.ignore_patterns)

        self.empty_folders = []
        self.selected_folders = set()
        self.is_running = False
        self.is_scanning = False
        self.scan_thread = None
        self.delete_thread = None
        self.sort_column = -1
        self.sort_order = Qt.SortOrder.AscendingOrder
        self.scan_start_time = 0
        self.cluster_size = get_cluster_size(scan_path)

        self.setWindowTitle("Empty Folder Cleaner")
        self.setMinimumSize(1000, 680)

        self.status_bar = QStatusBar()
        self.setStatusBar(self.status_bar)

        self.setup_ui()
        self.setup_shortcuts()
        self.apply_theme(self.theme_dark)
        self.status_bar.showMessage(I18n.get_text("ready", self.lang))
        self.restore_geometry()
        self.show()
        self.start_scan()

        if self.theme_mode == "system" and _DARKDETECT_AVAILABLE:
            self.dark_timer = QTimer(self)
            self.dark_timer.timeout.connect(self.check_system_theme)
            self.dark_timer.start(2000)

    def check_system_theme(self):
        if self.theme_mode != "system":
            return
        current = self.is_system_dark()
        if current != self.theme_dark:
            self.theme_dark = current
            self.apply_theme(current)

    def closeEvent(self, event):
        self.engine.stop()
        if self.scan_thread and self.scan_thread.isRunning():
            self.scan_thread.quit()
            self.scan_thread.wait(5000)
        if self.delete_thread and self.delete_thread.isRunning():
            self.delete_thread.quit()
            self.delete_thread.wait(5000)
        self.save_settings()
        super().closeEvent(event)

    def save_settings(self):
        self.settings.setValue("language", self.lang)
        self.settings.setValue("theme_mode", self.theme_mode)
        self.settings.setValue("dark_mode", self.theme_dark)
        self.settings.setValue("scan_dir", self.engine.scan_dir)
        self.settings.setValue("geometry", self.saveGeometry())
        self.settings.setValue("splitter_state", self.splitter.saveState())
        self.settings.setValue("ignored_paths", list(self.ignored_paths_raw))
        self.settings.setValue("ignore_patterns", self.ignore_patterns)
        self.settings.setValue("use_trash", self.use_trash)
        self.settings.setValue("recent_dirs", self.recent_dirs)

    def restore_geometry(self):
        geometry = self.settings.value("geometry")
        if geometry:
            self.restoreGeometry(geometry)
        else:
            self.showMaximized()
        splitter_state = self.settings.value("splitter_state")
        if splitter_state:
            self.splitter.restoreState(splitter_state)

    def is_system_dark(self):
        if _DARKDETECT_AVAILABLE:
            return darkdetect.isDark()
        return False

    def setup_shortcuts(self):
        QShortcut(QKeySequence("Ctrl+A"), self, self.select_all)
        QShortcut(QKeySequence("F5"), self, self.start_scan)
        QShortcut(QKeySequence("Escape"), self, self.escape_pressed)

    def escape_pressed(self):
        if self.is_scanning:
            self.stop_scan()
        elif self.is_running:
            self.stop_delete()

    def setup_ui(self):
        central_widget = QWidget()
        self.setCentralWidget(central_widget)
        main_layout = QVBoxLayout(central_widget)
        main_layout.setContentsMargins(0, 0, 0, 0)
        main_layout.setSpacing(0)

        content_widget = QWidget()
        content_layout = QVBoxLayout(content_widget)
        content_layout.setContentsMargins(24, 20, 24, 20)
        content_layout.setSpacing(16)

        header_layout = QHBoxLayout()
        self.title_label = QLabel()
        self.title_label.setFont(QFont("Segoe UI", 28, QFont.Weight.Bold))
        header_layout.addWidget(self.title_label)
        header_layout.addStretch()

        controls_layout = QHBoxLayout()
        theme_layout = QHBoxLayout()
        theme_label = QLabel(I18n.get_text("theme", self.lang))
        self.theme_combo = QComboBox()
        self.theme_combo.addItem(I18n.get_text("system", self.lang), "system")
        self.theme_combo.addItem(I18n.get_text("dark", self.lang), "dark")
        self.theme_combo.addItem(I18n.get_text("light", self.lang), "light")
        idx = self.theme_combo.findData(self.theme_mode)
        self.theme_combo.setCurrentIndex(idx if idx >= 0 else 0)
        self.theme_combo.currentIndexChanged.connect(self.change_theme_mode)
        theme_layout.addWidget(theme_label)
        theme_layout.addWidget(self.theme_combo)
        controls_layout.addLayout(theme_layout)

        self.lang_combo = QComboBox()
        self.lang_combo.addItem("English", "en")
        self.lang_combo.addItem("中文", "zh")
        self.lang_combo.setCurrentIndex(0 if self.lang == "en" else 1)
        self.lang_combo.currentIndexChanged.connect(self.change_language)
        self.lang_combo.setFixedWidth(110)
        controls_layout.addWidget(self.lang_combo)

        self.recycle_checkbox = QCheckBox(I18n.get_text("use_recycle", self.lang))
        self.recycle_checkbox.setChecked(self.use_trash)
        self.recycle_checkbox.stateChanged.connect(lambda state: setattr(self, 'use_trash', state == Qt.CheckState.Checked.value))
        controls_layout.addWidget(self.recycle_checkbox)

        header_layout.addLayout(controls_layout)
        content_layout.addLayout(header_layout)

        self.subtitle_label = QLabel()
        self.subtitle_label.setFont(QFont("Segoe UI", 12))
        content_layout.addWidget(self.subtitle_label)

        dir_layout = QHBoxLayout()
        self.dir_label = QLabel()
        self.dir_label.setFont(QFont("Segoe UI", 10, QFont.Weight.Bold))
        dir_layout.addWidget(self.dir_label)
        self.dir_combo = QComboBox()
        self.dir_combo.setEditable(True)
        self.dir_combo.setInsertPolicy(QComboBox.InsertPolicy.NoInsert)
        self.dir_combo.lineEdit().setReadOnly(True)
        self.update_recent_dirs_combo()
        self.dir_combo.currentTextChanged.connect(self.on_dir_changed)
        dir_layout.addWidget(self.dir_combo, 1)
        self.browse_button = QPushButton()
        self.browse_button.clicked.connect(self.select_directory)
        dir_layout.addWidget(self.browse_button)
        content_layout.addLayout(dir_layout)

        filter_layout = QHBoxLayout()
        self.filter_input = QLineEdit()
        self.filter_input.setPlaceholderText(I18n.get_text("search", self.lang))
        self.filter_input.textChanged.connect(self.apply_filter)
        filter_layout.addWidget(self.filter_input, 1)
        self.btn_manage_rules = QPushButton(I18n.get_text("manage_rules", self.lang))
        self.btn_manage_rules.clicked.connect(self.manage_ignore_rules)
        filter_layout.addWidget(self.btn_manage_rules)
        content_layout.addLayout(filter_layout)

        self.splitter = QSplitter(Qt.Orientation.Vertical)
        table_container = QFrame()
        table_container.setFrameShape(QFrame.Shape.StyledPanel)
        table_layout = QVBoxLayout(table_container)
        table_layout.setContentsMargins(0, 0, 0, 0)

        self.table = QTableWidget()
        self.table.setColumnCount(4)
        self.table.setHorizontalHeaderLabels(["Path", "Modified", "Status", "Select"])
        self.table.horizontalHeader().setSectionResizeMode(0, QHeaderView.ResizeMode.Stretch)
        self.table.horizontalHeader().setSectionResizeMode(1, QHeaderView.ResizeMode.Fixed)
        self.table.horizontalHeader().resizeSection(1, 150)
        self.table.horizontalHeader().setSectionResizeMode(2, QHeaderView.ResizeMode.Fixed)
        self.table.horizontalHeader().resizeSection(2, 120)
        self.table.horizontalHeader().setSectionResizeMode(3, QHeaderView.ResizeMode.Fixed)
        self.table.horizontalHeader().resizeSection(3, 80)
        self.table.verticalHeader().setVisible(False)
        self.table.setSelectionMode(QAbstractItemView.SelectionMode.NoSelection)
        self.table.setEditTriggers(QAbstractItemView.EditTrigger.NoEditTriggers)
        self.table.setAlternatingRowColors(False)
        self.table.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self.table.customContextMenuRequested.connect(self.show_context_menu)
        self.table.horizontalHeader().sectionClicked.connect(self.on_header_clicked)
        table_layout.addWidget(self.table)

        log_container = QFrame()
        log_container.setFrameShape(QFrame.Shape.StyledPanel)
        log_layout = QVBoxLayout(log_container)
        log_layout.setContentsMargins(0, 0, 0, 0)
        self.log_text = QPlainTextEdit()
        self.log_text.setReadOnly(True)
        self.log_text.setMaximumBlockCount(500)
        self.log_text.setFrameShape(QFrame.Shape.NoFrame)
        log_layout.addWidget(self.log_text)

        self.splitter.addWidget(table_container)
        self.splitter.addWidget(log_container)
        self.splitter.setStretchFactor(0, 2)
        self.splitter.setStretchFactor(1, 1)
        content_layout.addWidget(self.splitter, 1)

        progress_layout = QVBoxLayout()
        self.overall_label = QLabel()
        self.overall_label.setFont(QFont("Segoe UI", 10, QFont.Weight.Bold))
        progress_layout.addWidget(self.overall_label)
        overall_row = QHBoxLayout()
        self.overall_progress = QProgressBar()
        self.overall_progress.setRange(0, 100)
        overall_row.addWidget(self.overall_progress, 1)
        self.overall_text = QLabel("0 / 0")
        overall_row.addWidget(self.overall_text)
        progress_layout.addLayout(overall_row)
        self.space_label = QLabel()
        progress_layout.addWidget(self.space_label)
        content_layout.addLayout(progress_layout)

        button_layout = QHBoxLayout()
        self.btn_refresh = QPushButton()
        self.btn_refresh.clicked.connect(self.start_scan)
        self.btn_stop_scan = QPushButton()
        self.btn_stop_scan.clicked.connect(self.stop_scan)
        self.btn_stop_scan.setVisible(False)
        self.btn_select_all = QPushButton()
        self.btn_select_all.clicked.connect(self.select_all)
        self.btn_deselect_all = QPushButton()
        self.btn_deselect_all.clicked.connect(self.deselect_all)
        button_layout.addWidget(self.btn_refresh)
        button_layout.addWidget(self.btn_stop_scan)
        button_layout.addWidget(self.btn_select_all)
        button_layout.addWidget(self.btn_deselect_all)
        button_layout.addStretch()

        self.btn_start = QPushButton()
        self.btn_start.clicked.connect(self.start_delete_with_confirm)
        self.btn_stop = QPushButton()
        self.btn_stop.clicked.connect(self.stop_delete)
        self.btn_stop.setEnabled(False)
        button_layout.addWidget(self.btn_start)
        button_layout.addWidget(self.btn_stop)
        content_layout.addLayout(button_layout)

        main_layout.addWidget(content_widget, 1)
        self.update_texts()

    def update_recent_dirs_combo(self):
        self.dir_combo.clear()
        for d in self.recent_dirs:
            self.dir_combo.addItem(d)
        self.dir_combo.setCurrentText(self.engine.scan_dir)

    def on_dir_changed(self, text):
        if text and os.path.isdir(text) and text != self.engine.scan_dir:
            self.engine.scan_dir = text
            self.cluster_size = get_cluster_size(text)
            self.start_scan()

    def select_directory(self):
        dir_path = QFileDialog.getExistingDirectory(self, I18n.get_text("select_dir", self.lang), self.dir_combo.currentText())
        if dir_path:
            self.add_recent_dir(dir_path)
            self.dir_combo.setCurrentText(dir_path)

    def add_recent_dir(self, path):
        if path in self.recent_dirs:
            self.recent_dirs.remove(path)
        self.recent_dirs.insert(0, path)
        self.recent_dirs = self.recent_dirs[:5]
        self.update_recent_dirs_combo()

    def change_theme_mode(self):
        mode = self.theme_combo.currentData()
        self.theme_mode = mode
        if mode == "system":
            self.theme_dark = self.is_system_dark()
        elif mode == "dark":
            self.theme_dark = True
        else:
            self.theme_dark = False
        self.apply_theme(self.theme_dark)

    def manage_ignore_rules(self):
        dialog = IgnoreRulesDialog(self, self.ignore_patterns, self.lang)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            self.ignore_patterns = dialog.get_patterns()
            self.engine.set_ignore_patterns(self.ignore_patterns)
            self.start_scan()

    def show_context_menu(self, pos):
        item = self.table.itemAt(pos)
        if not item:
            return
        row = item.row()
        path = self.table.item(row, 0).text()
        menu = QMenu(self)
        open_action = menu.addAction(I18n.get_text("open_folder", self.lang))
        copy_action = menu.addAction(I18n.get_text("copy_path", self.lang))
        ignore_action = menu.addAction(I18n.get_text("ignore_this", self.lang))
        action = menu.exec(self.table.viewport().mapToGlobal(pos))
        if action == open_action:
            os.startfile(path) if sys.platform == "win32" else os.system(f'xdg-open "{path}"' if sys.platform != "darwin" else f'open "{path}"')
        elif action == copy_action:
            QApplication.clipboard().setText(path)
        elif action == ignore_action:
            self.ignored_paths_raw.add(os.path.normpath(path))
            self.add_log(I18n.get_text("ignored_folder", self.lang, path=path), True)
            self.start_scan()

    def on_header_clicked(self, logicalIndex):
        if logicalIndex == 0 or logicalIndex == 1:
            if self.sort_column == logicalIndex:
                self.sort_order = Qt.SortOrder.DescendingOrder if self.sort_order == Qt.SortOrder.AscendingOrder else Qt.SortOrder.AscendingOrder
            else:
                self.sort_column = logicalIndex
                self.sort_order = Qt.SortOrder.AscendingOrder
            self.apply_sort()

    def apply_sort(self):
        if self.sort_column < 0:
            return
        self.table.sortItems(self.sort_column, self.sort_order)

    def apply_filter(self):
        text = self.filter_input.text().lower()
        for row in range(self.table.rowCount()):
            path_item = self.table.item(row, 0)
            if path_item:
                self.table.setRowHidden(row, text not in path_item.text().lower())

    def update_texts(self, refresh_table=False):
        t = lambda key: I18n.get_text(key, self.lang)
        self.title_label.setText(t("title"))
        self.subtitle_label.setText(t("subtitle"))
        self.dir_label.setText(t("scan_dir"))
        self.browse_button.setText(t("browse"))
        self.filter_input.setPlaceholderText(t("search"))
        self.table.setHorizontalHeaderLabels([t("path"), t("modified"), t("status"), t("select")])
        self.overall_label.setText(t("overall_progress"))
        self.btn_refresh.setText(t("refresh"))
        self.btn_stop_scan.setText(t("stop_scan"))
        self.btn_select_all.setText(t("select_all"))
        self.btn_deselect_all.setText(t("deselect_all"))
        self.btn_start.setText(t("start_delete"))
        self.btn_stop.setText(t("stop"))
        self.recycle_checkbox.setText(t("use_recycle"))
        self.btn_manage_rules.setText(t("manage_rules"))
        if refresh_table:
            self.refresh_table_display()
        self.update_selected_count()

    def update_selected_count(self):
        count = len(self.selected_folders)
        if not self.is_scanning and not self.is_running:
            self.status_bar.showMessage(
                I18n.get_text("selected_count", self.lang, count=count) + "  " +
                I18n.get_text("ready", self.lang))

    def change_language(self, idx):
        self.lang = self.lang_combo.currentData()
        self.engine.lang = self.lang
        self.update_texts(refresh_table=True)

    def start_scan(self):
        if self.is_running or self.is_scanning:
            self.show_message("warning", I18n.get_text("scan_running", self.lang))
            return
        self.is_scanning = True
        self.update_scan_buttons_state()
        self.status_bar.showMessage(I18n.get_text("scanning", self.lang, count=0))
        self.empty_folders.clear()
        self.selected_folders.clear()
        self.table.setRowCount(0)
        self.sort_column = -1
        self.engine.stop_event.clear()
        self.engine.set_ignored_paths(self.ignored_paths_raw)
        self.engine.set_ignore_patterns(self.ignore_patterns)

        self.overall_progress.setRange(0, 0)
        self.overall_text.setText("")
        self.space_label.setText("")

        self.scan_start_time = time.time()
        self.scan_thread = ScanThread(self.engine)
        self.scan_thread.folder_found.connect(self.on_folder_found)
        self.scan_thread.finished_scan.connect(self.on_scan_finished)
        self.scan_thread.error_occurred.connect(self.on_scan_error)
        self.scan_thread.stats_ready.connect(self.on_stats_ready)
        self.scan_thread.start()

    def stop_scan(self):
        if not self.is_scanning:
            return
        self.engine.stop()
        self.status_bar.showMessage(I18n.get_text("scan_stopped", self.lang))

    def on_folder_found(self, path):
        self.empty_folders.append(path)
        self._append_row(path)
        self.status_bar.showMessage(I18n.get_text("scanning", self.lang, count=len(self.empty_folders)))

    def _append_row(self, path):
        row = self.table.rowCount()
        self.table.insertRow(row)
        self.table.setItem(row, 0, QTableWidgetItem(path))
        try:
            mtime = os.path.getmtime(path)
            mtime_str = time.strftime("%Y-%m-%d %H:%M", time.localtime(mtime))
        except:
            mtime_str = ""
        self.table.setItem(row, 1, QTableWidgetItem(mtime_str))
        status_text = I18n.get_text("not_deleted", self.lang)
        self.table.setItem(row, 2, QTableWidgetItem(status_text))
        cb = QCheckBox()
        cb.setChecked(path in self.selected_folders)
        cb.stateChanged.connect(lambda state, p=path: self.toggle_selection(p, state == Qt.CheckState.Checked.value))
        cb.setEnabled(not self.is_running)
        self.table.setCellWidget(row, 3, cb)

    def toggle_selection(self, path, checked):
        if checked:
            self.selected_folders.add(path)
        else:
            self.selected_folders.discard(path)
        self.update_selected_count()
        self.update_space_estimate()

    def update_space_estimate(self):
        if not self.selected_folders:
            self.space_label.setText("")
            return
        total_space = len(self.selected_folders) * self.cluster_size
        self.space_label.setText(I18n.get_text("space_usage", self.lang, size=format_size(total_space)))

    def refresh_table_display(self):
        for row in range(self.table.rowCount()):
            path_item = self.table.item(row, 0)
            if path_item:
                path = path_item.text()
                checkbox = self.table.cellWidget(row, 3)
                if isinstance(checkbox, QCheckBox):
                    was_checked = path in self.selected_folders
                    checkbox.blockSignals(True)
                    checkbox.setChecked(was_checked)
                    checkbox.setEnabled(not self.is_running)
                    checkbox.blockSignals(False)
        self.update_selected_count()
        self.update_space_estimate()

    def on_stats_ready(self, total, empty):
        elapsed = time.time() - self.scan_start_time
        self.add_log(I18n.get_text("scan_stats", self.lang, total=total, empty=empty, elapsed=elapsed))

    def on_scan_finished(self):
        self.is_scanning = False
        self.update_scan_buttons_state()
        self.overall_progress.setRange(0, 100)
        self.overall_progress.setValue(0)
        self.overall_text.setText(f"0 / {len(self.empty_folders)}")
        self.status_bar.showMessage(
            I18n.get_text("ready", self.lang) if self.empty_folders
            else I18n.get_text("not_found", self.lang))
        if self.sort_column != -1:
            self.apply_sort()

    def on_scan_error(self, error_msg):
        self.is_scanning = False
        self.update_scan_buttons_state()
        self.overall_progress.setRange(0, 100)
        self.show_message("error", error_msg)
        self.status_bar.showMessage(error_msg)

    def select_all(self):
        self.selected_folders = set(self.empty_folders)
        self.refresh_table_display()

    def deselect_all(self):
        self.selected_folders.clear()
        self.refresh_table_display()

    def start_delete_with_confirm(self):
        if self.is_running:
            self.show_message("warning", I18n.get_text("delete_running", self.lang))
            return
        to_delete = [p for p in self.selected_folders if os.path.isdir(p) and not os.listdir(p)]
        if not to_delete:
            self.show_message("info", I18n.get_text("no_folder_selected", self.lang))
            return

        preview_list = "\n".join(to_delete[:20])
        if len(to_delete) > 20:
            preview_list += f"\n... and {len(to_delete) - 20} more"

        msg = QMessageBox(self)
        msg.setWindowTitle(I18n.get_text("confirm_delete_title", self.lang))
        msg.setText(I18n.get_text("confirm_delete_text", self.lang, count=len(to_delete), folder_list=preview_list))
        msg.setIcon(QMessageBox.Icon.Question)
        msg.setStandardButtons(QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No)
        if msg.exec() == QMessageBox.StandardButton.No:
            return

        self.start_delete(to_delete)

    def start_delete(self, to_delete):
        self.is_running = True
        self.update_button_states(running=True)
        self.overall_progress.setRange(0, 100)
        self.overall_progress.setValue(0)
        self.overall_text.setText(f"0 / {len(to_delete)}")
        self.space_label.setText("")

        for row in range(self.table.rowCount()):
            checkbox = self.table.cellWidget(row, 3)
            if isinstance(checkbox, QCheckBox):
                checkbox.setEnabled(False)

        self.delete_thread = DeleteThread(self.engine, to_delete, self.use_trash)
        self.delete_thread.log_signal.connect(self.add_log)
        self.delete_thread.progress_signal.connect(self.update_progress)
        self.delete_thread.finished_signal.connect(self.on_delete_finished)
        self.delete_thread.error_signal.connect(self.on_delete_error)
        self.delete_thread.start()

    def update_scan_buttons_state(self):
        self.btn_refresh.setVisible(not self.is_scanning)
        self.btn_stop_scan.setVisible(self.is_scanning)
        self.btn_start.setEnabled(not self.is_scanning and not self.is_running)

    def update_button_states(self, running):
        self.btn_refresh.setVisible(not running and not self.is_scanning)
        self.btn_stop_scan.setVisible(not running and self.is_scanning)
        self.btn_start.setEnabled(not running)
        self.btn_stop.setEnabled(running)
        self.btn_select_all.setEnabled(not running)
        self.btn_deselect_all.setEnabled(not running)

    def stop_delete(self):
        if not self.is_running:
            return
        self.engine.stop()
        self.btn_stop.setEnabled(False)
        self.status_bar.showMessage(I18n.get_text("stopped", self.lang))

    def on_delete_finished(self, failed_list):
        self.is_running = False
        self.update_button_states(running=False)
        for path in failed_list:
            norm = os.path.normpath(path)
            if sys.platform == "win32":
                norm = norm.lower()
            self.ignored_paths_raw.add(norm)
            self.add_log(I18n.get_text("ignored_folder", self.lang, path=path), True)
        self.settings.setValue("ignored_paths", list(self.ignored_paths_raw))

        success = set(self.selected_folders) - set(failed_list)
        for path in success:
            if path in self.empty_folders:
                self.empty_folders.remove(path)
        self.selected_folders.clear()
        self.refresh_table_display()

        for row in range(self.table.rowCount()):
            checkbox = self.table.cellWidget(row, 3)
            if isinstance(checkbox, QCheckBox):
                checkbox.setEnabled(True)
        self.status_bar.showMessage(I18n.get_text("all_done", self.lang))

    def on_delete_error(self, error_msg):
        self.show_message("error", error_msg)

    def add_log(self, message, error=False):
        color = "red" if error else "inherit"
        self.log_text.appendHtml(
            f'<span style="color:{color};">[{time.strftime("%H:%M:%S")}] {message}</span>')

    def update_progress(self, completed, total):
        if total > 0:
            self.overall_progress.setValue(int(completed / total * 100))
            self.overall_text.setText(f"{completed} / {total}")
        else:
            self.overall_progress.setValue(0)
            self.overall_text.setText("0 / 0")

    def show_message(self, icon_type, text):
        msg = QMessageBox(self)
        msg.setWindowTitle(I18n.get_text("dialog_" + icon_type, self.lang))
        msg.setText(text)
        if icon_type == "info":
            msg.setIcon(QMessageBox.Icon.Information)
        elif icon_type == "warning":
            msg.setIcon(QMessageBox.Icon.Warning)
        else:
            msg.setIcon(QMessageBox.Icon.Critical)
        msg.exec()

    def apply_theme(self, dark):
        if dark:
            palette = QPalette()
            palette.setColor(QPalette.ColorRole.Window, QColor(18, 18, 18))
            palette.setColor(QPalette.ColorRole.WindowText, QColor(230, 230, 230))
            palette.setColor(QPalette.ColorRole.Base, QColor(30, 30, 30))
            palette.setColor(QPalette.ColorRole.Text, QColor(230, 230, 230))
            palette.setColor(QPalette.ColorRole.Button, QColor(50, 50, 50))
            palette.setColor(QPalette.ColorRole.ButtonText, QColor(230, 230, 230))
            palette.setColor(QPalette.ColorRole.Highlight, QColor(0, 200, 200))
            palette.setColor(QPalette.ColorRole.HighlightedText, Qt.GlobalColor.black)
            QApplication.instance().setPalette(palette)
            self.setStyleSheet("""
                QWidget { background-color: #121212; color: #e0e0e0; }
                QPushButton { background-color: #2c2c2c; border: none; border-radius: 8px; padding: 8px 16px; font-size: 13px; }
                QPushButton:hover { background-color: #3c3c3c; }
                QPushButton:pressed { background-color: #505050; }
                QPushButton:disabled { color: #707070; }
                QProgressBar { border: none; border-radius: 6px; background-color: #2c2c2c; text-align: center; height: 10px; }
                QProgressBar::chunk { background-color: #00bcd4; border-radius: 6px; }
                QLineEdit, QPlainTextEdit, QTableWidget, QComboBox { background-color: #1e1e1e; border: 1px solid #3c3c3c; border-radius: 6px; padding: 4px; color: #e0e0e0; }
                QLineEdit:focus, QComboBox:focus { border-color: #00bcd4; }
                QHeaderView::section { background-color: #2c2c2c; padding: 4px; border: none; }
                QTableWidget { gridline-color: #3c3c3c; }
                QComboBox::drop-down { border: none; }
                QStatusBar { background-color: #121212; color: #e0e0e0; }
                QCheckBox { color: #e0e0e0; }
                QCheckBox::indicator { width: 18px; height: 18px; }
                QListWidget { background-color: #1e1e1e; color: #e0e0e0; }
            """)
            self.title_label.setStyleSheet("color: #00e5ff; border-bottom: 2px solid #00e5ff; padding-bottom: 4px;")
            self.subtitle_label.setStyleSheet("color: #aaaaaa;")
        else:
            QApplication.instance().setPalette(QApplication.style().standardPalette())
            self.setStyleSheet("""
                QWidget { background-color: #f5f5f5; color: #202020; }
                QPushButton { background-color: #ffffff; border: 1px solid #d0d0d0; border-radius: 8px; padding: 8px 16px; font-size: 13px; }
                QPushButton:hover { background-color: #e0e0e0; }
                QPushButton:pressed { background-color: #cccccc; }
                QPushButton:disabled { color: #909090; }
                QProgressBar { border: 1px solid #d0d0d0; border-radius: 6px; background-color: #ffffff; text-align: center; height: 10px; }
                QProgressBar::chunk { background-color: #0088cc; border-radius: 6px; }
                QLineEdit, QPlainTextEdit, QTableWidget, QComboBox { background-color: #ffffff; border: 1px solid #d0d0d0; border-radius: 6px; padding: 4px; color: #202020; }
                QLineEdit:focus, QComboBox:focus { border-color: #0088cc; }
                QHeaderView::section { background-color: #f0f0f0; padding: 4px; border: none; }
                QTableWidget { gridline-color: #d0d0d0; }
                QComboBox::drop-down { border: none; }
                QStatusBar { background-color: #f5f5f5; color: #202020; }
                QCheckBox { color: #202020; }
                QCheckBox::indicator { width: 18px; height: 18px; }
                QListWidget { background-color: #ffffff; color: #202020; }
            """)
            self.title_label.setStyleSheet("color: #0055aa; border-bottom: 2px solid #0055aa; padding-bottom: 4px;")
            self.subtitle_label.setStyleSheet("color: #666666;")


if __name__ == "__main__":
    app = QApplication([])
    app.setStyle(QStyleFactory.create("Fusion"))
    window = MainWindow()
    app.exec()