"""Main window controller."""

import html
import os
import sys
import time

from PyQt6.QtCore import (
    QSettings, QStandardPaths, Qt, QTimer, QUrl
)
from PyQt6.QtGui import (
    QDesktopServices, QKeySequence, QPalette, QShortcut, QTextCursor
)
from PyQt6.QtWidgets import (
    QAbstractSpinBox, QApplication, QDialog, QFileDialog, QLineEdit,
    QMainWindow, QMenu, QMessageBox, QPlainTextEdit, QStatusBar, QTextEdit
)

from ..core.config import (
    APP_NAME, APP_ORG, CLOSE_MAX_RETRIES, CLOSE_RETRY_INTERVAL_MS, MAX_DELETE_PREVIEW,
    MAX_RECENT_DIRS, MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH, STATUS_UPDATE_INTERVAL,
    TABLE_FLUSH_INTERVAL_MS, TABLE_UPDATE_BATCH, THEME_POLL_INTERVAL_MS,
    LOG_ERROR_COLOR, LOG_MAX_BLOCKS, THREAD_QUIT_TIMEOUT_MS, as_str_list,
    get_system_language
)
from ..core.filesystem import format_size, get_cluster_size
from ..core.i18n import I18n
from ..core.scanner import (
    DeleteThread, EmptyFolderEngine, ScanThread, is_trash_available
)
from .dialogs import IgnoreRulesDialog
from .theme import apply_theme as _apply_theme, system_is_dark
from .widgets import build_ui, fill_theme_combo

SUPPORTED_LANGS = ("en", "zh")
SUPPORTED_THEME_MODES = ("system", "dark", "light")


def _valid_lang(value):
    return value if value in SUPPORTED_LANGS else get_system_language()


def _valid_theme_mode(value):
    return value if value in SUPPORTED_THEME_MODES else "system"


def _modified_text(path):
    try:
        return time.strftime("%Y-%m-%d %H:%M", time.localtime(os.path.getmtime(path)))
    except OSError:
        return ""


class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.settings = QSettings(APP_ORG, APP_NAME)
        self.lang = _valid_lang(self.settings.value("language", None))
        self.theme_mode = _valid_theme_mode(self.settings.value("theme_mode", None))
        self.theme_dark = system_is_dark() if self.theme_mode == "system" else self.settings.value("dark_mode", False, bool)

        scan_path = self.settings.value("scan_dir", "")
        if not scan_path or not os.path.isdir(scan_path):
            scan_path = QStandardPaths.writableLocation(QStandardPaths.StandardLocation.HomeLocation)
            if not scan_path or not os.path.isdir(scan_path):
                scan_path = os.path.expanduser('~')

        self.recent_dirs = as_str_list(self.settings.value("recent_dirs", []))
        if not self.recent_dirs:
            self.recent_dirs = [scan_path]
        if scan_path not in self.recent_dirs:
            self.recent_dirs.insert(0, scan_path)
        self.recent_dirs = self.recent_dirs[:MAX_RECENT_DIRS]

        self.ignored_paths_raw = set(as_str_list(self.settings.value("ignored_paths", [])))
        self.ignore_patterns = as_str_list(self.settings.value("ignore_patterns", []))
        self.use_trash = self.settings.value("use_trash", True, bool)

        self.engine = EmptyFolderEngine(scan_dir=scan_path, lang=self.lang)
        self.engine.set_ignored_paths(self.ignored_paths_raw)
        self.engine.set_ignore_patterns(self.ignore_patterns)

        self.is_running = False
        self.is_scanning = False
        self.scan_thread = None
        self.delete_thread = None
        self.sort_column = -1
        self.sort_order = Qt.SortOrder.AscendingOrder
        self.scan_start_time = 0
        self._last_status_update = 0.0
        self._pending_rows = []
        self._flush_timer = QTimer(self)
        self._flush_timer.setInterval(TABLE_FLUSH_INTERVAL_MS)
        self._flush_timer.timeout.connect(self._flush_pending_rows)
        self.cluster_size = get_cluster_size(scan_path)

        self.setWindowTitle(I18n.get_text("title", self.lang))
        self.setMinimumSize(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)

        self.status_bar = QStatusBar()
        self.setStatusBar(self.status_bar)

        self.setup_ui()
        self.setup_shortcuts()
        self.apply_theme(self.theme_dark)
        self.status_bar.showMessage(I18n.get_text("ready", self.lang))
        self.restore_geometry()
        self.show()
        self.start_scan()
        self.dark_timer = QTimer(self)
        self.dark_timer.timeout.connect(self.check_system_theme)
        self._sync_theme_timer()

    def _sync_theme_timer(self):
        if self.theme_mode == "system":
            self.dark_timer.start(THEME_POLL_INTERVAL_MS)
        else:
            self.dark_timer.stop()

    def check_system_theme(self):
        if self.theme_mode != "system":
            return
        current = system_is_dark()
        if current != self.theme_dark:
            self.theme_dark = current
            self.apply_theme(current)

    def closeEvent(self, event):
        if not self._shutdown_threads():
            event.ignore()
            self._close_retries = getattr(self, "_close_retries", 0) + 1
            if self._close_retries <= CLOSE_MAX_RETRIES:
                self.status_bar.showMessage(I18n.get_text("closing", self.lang))
                QTimer.singleShot(CLOSE_RETRY_INTERVAL_MS, self.close)
            return
        self._close_retries = 0
        self.engine.stop()
        self.save_settings()
        self.dark_timer.stop()
        super().closeEvent(event)

    def _shutdown_threads(self):
        """Stop workers and wait for them. False means a worker refused to stop."""
        self.engine.stop()
        for thread in (self.scan_thread, self.delete_thread):
            if thread is not None and thread.isRunning():
                if not thread.wait(THREAD_QUIT_TIMEOUT_MS):
                    return False
        return True

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

    def setup_shortcuts(self):
        QShortcut(QKeySequence("Ctrl+A"), self, self.select_all)
        QShortcut(QKeySequence("F5"), self, self.start_scan)
        QShortcut(QKeySequence("Escape"), self, self.escape_pressed)

    def escape_pressed(self):
        focused = QApplication.focusWidget()
        if isinstance(focused, (QLineEdit, QAbstractSpinBox, QPlainTextEdit, QTextEdit)):
            focused.clear()
            return
        if self.is_scanning:
            self.stop_scan()
        elif self.is_running:
            self.stop_delete()

    def setup_ui(self):
        build_ui(self)
        self.model.dataChanged.connect(self._on_model_data_changed)
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
        self.recent_dirs = self.recent_dirs[:MAX_RECENT_DIRS]
        self.update_recent_dirs_combo()

    def change_theme_mode(self):
        mode = self.theme_combo.currentData() or "system"
        self.theme_mode = mode
        if mode == "system":
            self.theme_dark = system_is_dark()
        elif mode == "dark":
            self.theme_dark = True
        else:
            self.theme_dark = False
        self._sync_theme_timer()
        self.apply_theme(self.theme_dark)
        self.settings.setValue("theme_mode", self.theme_mode)
        self.settings.setValue("dark_mode", self.theme_dark)

    def manage_ignore_rules(self):
        dialog = IgnoreRulesDialog(self, self.ignore_patterns, self.lang)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            self.ignore_patterns = dialog.get_patterns()
            self.engine.set_ignore_patterns(self.ignore_patterns)
            self.start_scan()

    def show_context_menu(self, pos):
        index = self.table.indexAt(pos)
        if not index.isValid():
            return
        source_index = self.proxy.mapToSource(index)
        path = self.model.path_at_row(source_index.row())
        menu = QMenu(self)
        open_action = menu.addAction(I18n.get_text("open_folder", self.lang))
        copy_action = menu.addAction(I18n.get_text("copy_path", self.lang))
        ignore_action = menu.addAction(I18n.get_text("ignore_this", self.lang))
        action = menu.exec(self.table.viewport().mapToGlobal(pos))
        if action == open_action:
            QDesktopServices.openUrl(QUrl.fromLocalFile(path))
        elif action == copy_action:
            QApplication.clipboard().setText(path)
        elif action == ignore_action:
            self.ignored_paths_raw.add(os.path.normpath(path))
            self.add_log(I18n.get_text("ignored_folder", self.lang, path=path), True)
            self.start_scan()

    def on_header_clicked(self, logicalIndex):
        if logicalIndex not in (0, 1):
            return
        if self.sort_column == logicalIndex:
            self.sort_order = (
                Qt.SortOrder.DescendingOrder
                if self.sort_order == Qt.SortOrder.AscendingOrder
                else Qt.SortOrder.AscendingOrder
            )
        else:
            self.sort_column = logicalIndex
            self.sort_order = Qt.SortOrder.AscendingOrder
        self.apply_sort()

    def apply_sort(self):
        if self.sort_column < 0:
            return
        self.proxy.sort(self.sort_column, self.sort_order)
        self.table.horizontalHeader().setSortIndicator(
            self.sort_column, self.sort_order
        )

    def apply_filter(self):
        self.proxy.set_path_filter(self.filter_input.text())

    def update_texts(self, refresh_table=False):
        t = lambda key: I18n.get_text(key, self.lang)
        self.setWindowTitle(t("title"))
        self.title_label.setText(t("title"))
        self.subtitle_label.setText(t("subtitle"))
        self.dir_label.setText(t("scan_dir"))
        self.browse_button.setText(t("browse"))
        self.filter_input.setPlaceholderText(t("search"))
        self.model.set_language(self.lang)
        self.overall_label.setText(t("overall_progress"))
        self.theme_label.setText(t("theme"))
        fill_theme_combo(self)
        self.recycle_checkbox.setToolTip(
            "" if is_trash_available() else t("trash_unavailable")
        )
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
        count = self.model.selected_count()
        self.selection_label.setText(
            I18n.get_text("selected_count", self.lang, count=count)
        )

    def change_language(self, idx):
        self.lang = self.lang_combo.currentData() or "en"
        self.engine.lang = self.lang
        self.settings.setValue("language", self.lang)
        self.update_texts(refresh_table=True)

    def start_scan(self):
        if self.is_running or self.is_scanning:
            self.show_message("warning", I18n.get_text("scan_running", self.lang))
            return
        if not self._release_thread("scan_thread"):
            self.show_message("warning", I18n.get_text("scan_running", self.lang))
            return
        self.is_scanning = True
        self._sync_action_states()
        self.status_bar.showMessage(I18n.get_text("scanning", self.lang, count=0))
        self._pending_rows = []
        self.model.clear()
        self.sort_column = -1
        self.sort_order = Qt.SortOrder.AscendingOrder
        self.proxy.sort(-1, Qt.SortOrder.AscendingOrder)
        self.table.horizontalHeader().setSortIndicator(-1, Qt.SortOrder.AscendingOrder)
        self.engine.stop_event.clear()
        self.engine.set_ignored_paths(self.ignored_paths_raw)
        self.engine.set_ignore_patterns(self.ignore_patterns)

        self.overall_progress.setRange(0, 0)
        self.overall_text.setText("")
        self.space_label.setText("")

        self.scan_start_time = time.time()
        self._last_status_update = 0.0
        self.scan_thread = ScanThread(self.engine)
        self.scan_thread.folder_found.connect(self.on_folder_found)
        self.scan_thread.finished_scan.connect(self.on_scan_finished)
        self.scan_thread.error_occurred.connect(self.on_scan_error)
        self.scan_thread.stats_ready.connect(self.on_stats_ready)
        self.scan_thread.start()

    def _release_thread(self, attr):
        """Retire a finished worker, or wait out a still-running one.

        Replacing a QThread reference while its run() is still executing destroys
        the underlying object mid-flight and aborts the process, so the previous
        worker must be fully stopped before a new one is stored.
        """
        thread = getattr(self, attr)
        if thread is None:
            return True
        if thread.isRunning():
            self.engine.stop()
            if not thread.wait(THREAD_QUIT_TIMEOUT_MS):
                return False
        thread.deleteLater()
        setattr(self, attr, None)
        return True

    def stop_scan(self):
        if not self.is_scanning:
            return
        self.engine.stop()
        self.status_bar.showMessage(I18n.get_text("scan_stopped", self.lang))

    def on_folder_found(self, path):
        self._pending_rows.append(path)
        if not self._flush_timer.isActive():
            self._flush_timer.start()

    def _flush_pending_rows(self):
        pending, self._pending_rows = self._pending_rows, []
        if pending:
            self.model.add_folders([(path, _modified_text(path)) for path in pending])
        count = self.model.rowCount()
        now = time.monotonic()
        if now - self._last_status_update >= STATUS_UPDATE_INTERVAL or count % TABLE_UPDATE_BATCH == 0:
            self._last_status_update = now
            self.status_bar.showMessage(I18n.get_text("scanning", self.lang, count=count))
        if not self.is_scanning:
            self._flush_timer.stop()

    def _append_row(self, path):
        self.model.add_folder(path, _modified_text(path))

    def _on_model_data_changed(self, top_left, bottom_right, roles):
        if roles and Qt.ItemDataRole.CheckStateRole not in roles:
            return
        self.update_selected_count()
        self.update_space_estimate()

    def update_space_estimate(self):
        selected_count = self.model.selected_count()
        if not selected_count:
            self.space_label.setText("")
            return
        total_space = selected_count * self.cluster_size
        self.space_label.setText(I18n.get_text("space_usage", self.lang, size=format_size(total_space)))

    def refresh_table_display(self):
        self.update_selected_count()
        self.update_space_estimate()

    def on_stats_ready(self, total, empty):
        elapsed = time.time() - self.scan_start_time
        self.add_log(I18n.get_text("scan_stats", self.lang, total=total, empty=empty, elapsed=elapsed))

    def on_scan_finished(self):
        self.is_scanning = False
        self._flush_timer.stop()
        self._flush_pending_rows()
        self._sync_action_states()
        self.overall_progress.setRange(0, 100)
        self.overall_progress.setValue(0)
        count = self.model.rowCount()
        self.overall_text.setText(f"0 / {count}")
        if self.engine.stop_event.is_set():
            self.status_bar.showMessage(I18n.get_text("scan_stopped", self.lang))
        elif count:
            self.status_bar.showMessage(I18n.get_text("ready", self.lang))
        else:
            self.status_bar.showMessage(I18n.get_text("not_found", self.lang))
        if self.sort_column != -1:
            self.apply_sort()

    def on_scan_error(self, error_msg):
        self.is_scanning = False
        self._flush_timer.stop()
        self._flush_pending_rows()
        self._sync_action_states()
        self.overall_progress.setRange(0, 100)
        self.show_message("error", error_msg)
        self.status_bar.showMessage(error_msg)

    def select_all(self):
        focused = QApplication.focusWidget()
        if isinstance(focused, (QLineEdit, QAbstractSpinBox, QPlainTextEdit, QTextEdit)):
            focused.selectAll()
            return
        self.model.select_all()

    def deselect_all(self):
        self.model.clear_selection()

    def start_delete_with_confirm(self):
        if self.is_running:
            self.show_message("warning", I18n.get_text("delete_running", self.lang))
            return
        if self.is_scanning:
            self.show_message("warning", I18n.get_text("scan_running", self.lang))
            return
        to_delete = []
        for path in self.model.selected_paths():
            if not os.path.isdir(path):
                continue
            try:
                if not os.listdir(path):
                    to_delete.append(path)
            except OSError:
                continue
        if not to_delete:
            self.show_message("info", I18n.get_text("no_folder_selected", self.lang))
            return

        preview_list = "\n".join(to_delete[:MAX_DELETE_PREVIEW])
        if len(to_delete) > MAX_DELETE_PREVIEW:
            preview_list += "\n" + I18n.get_text(
                "and_more", self.lang, count=len(to_delete) - MAX_DELETE_PREVIEW
            )

        msg = QMessageBox(self)
        msg.setWindowTitle(I18n.get_text("confirm_delete_title", self.lang))
        msg.setTextFormat(Qt.TextFormat.PlainText)
        msg.setText(I18n.get_text("confirm_delete_text", self.lang, count=len(to_delete), folder_list=preview_list))
        msg.setIcon(QMessageBox.Icon.Question)
        msg.setStandardButtons(QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No)
        if msg.exec() == QMessageBox.StandardButton.No:
            return

        self.start_delete(to_delete)

    def start_delete(self, to_delete):
        if not self._release_thread("delete_thread"):
            self.show_message("warning", I18n.get_text("delete_running", self.lang))
            return
        self.is_running = True
        self.current_delete_paths = list(to_delete)
        self._sync_action_states()
        self.overall_progress.setRange(0, 100)
        self.overall_progress.setValue(0)
        self.overall_text.setText(f"0 / {len(to_delete)}")
        self.space_label.setText("")

        self.model.set_selectable(False)

        self.delete_thread = DeleteThread(self.engine, to_delete, self.use_trash)
        self.delete_thread.log_signal.connect(self.add_log)
        self.delete_thread.progress_signal.connect(self.update_progress)
        self.delete_thread.finished_signal.connect(self.on_delete_finished)
        self.delete_thread.error_signal.connect(self.on_delete_error)
        self.engine.stop_event.clear()
        self.delete_thread.start()

    def _sync_action_states(self):
        busy = self.is_scanning or self.is_running
        self.btn_refresh.setVisible(not self.is_scanning and not self.is_running)
        self.btn_stop_scan.setVisible(self.is_scanning and not self.is_running)
        self.btn_stop.setEnabled(self.is_running)
        for button in (
            self.btn_start, self.btn_select_all, self.btn_deselect_all,
            self.btn_manage_rules, self.dir_combo, self.browse_button,
        ):
            button.setEnabled(not busy)

    def stop_delete(self):
        if not self.is_running:
            return
        self.engine.stop()
        self.btn_stop.setEnabled(False)
        self.status_bar.showMessage(I18n.get_text("stopped", self.lang))

    def on_delete_finished(self, failed_list, remaining, deleted_paths):
        self.finalize_delete(failed_list, remaining, deleted_paths)

    def finalize_delete(self, failed_list, remaining, deleted_paths):
        self.is_running = False
        self._sync_action_states()
        for path in failed_list:
            norm = os.path.normpath(path)
            if sys.platform == "win32":
                norm = norm.lower()
            self.ignored_paths_raw.add(norm)
            self.add_log(I18n.get_text("ignored_folder", self.lang, path=path), True)
        self.settings.setValue("ignored_paths", list(self.ignored_paths_raw))

        self.model.remove_paths(deleted_paths)
        for path in remaining:
            if not self.model.has_path(path):
                self._append_row(path)
        self.model.set_selected_paths(remaining)
        self.model.set_selectable(True)
        self.refresh_table_display()

        if remaining:
            self.status_bar.showMessage(I18n.get_text("stopped", self.lang))
        elif failed_list:
            self.status_bar.showMessage(
                I18n.get_text("done_with_errors", self.lang, count=len(failed_list))
            )
        else:
            self.status_bar.showMessage(I18n.get_text("all_done", self.lang))

    def on_delete_error(self, error_msg):
        self.show_message("error", error_msg)
        self.finalize_delete([], list(getattr(self, "current_delete_paths", [])), [])

    def add_log(self, message, error=False):
        stamp = f"[{time.strftime('%H:%M:%S')}] "
        color = (
            LOG_ERROR_COLOR if error
            else self.log_text.palette().color(QPalette.ColorRole.Text).name()
        )
        cursor = self.log_text.textCursor()
        cursor.movePosition(QTextCursor.MoveOperation.End)
        cursor.insertHtml(
            f'<span style="color:{color};">{stamp}{html.escape(str(message))}</span>'
        )
        cursor.insertBlock()
        self._trim_log()

    def _trim_log(self):
        document = self.log_text.document()
        while document.blockCount() > LOG_MAX_BLOCKS:
            cursor = QTextCursor(document.firstBlock())
            cursor.select(QTextCursor.SelectionType.BlockUnderCursor)
            cursor.removeSelectedText()
            cursor.deleteChar()

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
        _apply_theme(self, dark)
