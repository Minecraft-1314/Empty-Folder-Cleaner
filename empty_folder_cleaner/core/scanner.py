"""Background scanning and deletion workers."""

import fnmatch
import os
import queue
import sys
import threading
from collections import deque
from concurrent.futures import ThreadPoolExecutor

from PyQt6.QtCore import QThread, pyqtSignal

from .config import SCAN_MAX_WORKERS
from .i18n import I18n

try:
    import send2trash
    _SEND2TRASH_AVAILABLE = True
except ImportError:
    _SEND2TRASH_AVAILABLE = False


class EmptyFolderEngine:
    def __init__(self, scan_dir, lang="en"):
        self.scan_dir = scan_dir
        self.lang = lang
        self.stop_event = threading.Event()
        self.ignored_paths = set()
        self.ignore_patterns = []
        self.case_insensitive = sys.platform == "win32"
        self._stats_lock = threading.Lock()
        self._scan_total = 0
        self._scan_empty = 0

    def _t(self, key, **kwargs):
        return I18n.get_text(key, self.lang, **kwargs)

    def set_ignored_paths(self, paths):
        normalized = {os.path.normpath(p) for p in paths}
        self.ignored_paths = (
            {p.lower() for p in normalized} if self.case_insensitive else normalized
        )

    def set_ignore_patterns(self, patterns):
        self.ignore_patterns = patterns

    def _is_ignored(self, norm_path):
        candidate = norm_path.lower() if self.case_insensitive else norm_path
        if candidate in self.ignored_paths:
            return True
        name = os.path.basename(norm_path)
        for pat in self.ignore_patterns:
            if fnmatch.fnmatch(name, pat):
                return True
        return False

    def _iter_top_level_dirs(self):
        try:
            with os.scandir(self.scan_dir) as entries:
                for entry in entries:
                    if self.stop_event.is_set():
                        return
                    try:
                        if entry.is_dir(follow_symlinks=False):
                            yield entry.path
                    except OSError:
                        continue
        except OSError:
            return

    def _scan_subtree(self, sub_root, found_queue):
        total = 0
        empty = 0
        try:
            for root, dirs, files in os.walk(
                sub_root, topdown=False, onerror=lambda error: None
            ):
                if self.stop_event.is_set():
                    break
                norm_root = os.path.normpath(root)
                total += 1
                if self._is_ignored(norm_root):
                    continue
                if not dirs and not files:
                    empty += 1
                    found_queue.put(norm_root)
        except OSError:
            pass
        finally:
            with self._stats_lock:
                self._scan_total += total
                self._scan_empty += empty

    def scan_empty_folders_generator(self, stats_callback=None):
        if not os.path.isdir(self.scan_dir):
            raise Exception(self._t("error_scan", error="Directory does not exist"))

        self._scan_total = 0
        self._scan_empty = 0
        roots = sorted(self._iter_top_level_dirs())
        if not roots:
            if stats_callback:
                stats_callback(0, 0)
            return

        found_queue = queue.Queue()

        def worker(root):
            try:
                self._scan_subtree(root, found_queue)
            finally:
                found_queue.put(None)

        cpu_workers = os.cpu_count() or 1
        worker_count = max(1, min(len(roots), cpu_workers, SCAN_MAX_WORKERS))
        executor = ThreadPoolExecutor(max_workers=worker_count)
        futures = [executor.submit(worker, root) for root in roots]
        pending = len(futures)
        try:
            while pending:
                if self.stop_event.is_set():
                    for future in futures:
                        future.cancel()
                    break
                item = found_queue.get()
                if item is None:
                    pending -= 1
                else:
                    yield item
        finally:
            executor.shutdown(wait=True, cancel_futures=True)
            if stats_callback:
                with self._stats_lock:
                    stats_callback(self._scan_total, self._scan_empty)

    @staticmethod
    def _directory_depth(path):
        return os.path.normpath(path).count(os.sep)

    def _is_within_scan_root(self, path):
        if not path:
            return False
        norm_scan = os.path.normcase(os.path.normpath(self.scan_dir))
        norm_path = os.path.normcase(os.path.normpath(path))
        try:
            return os.path.commonpath([norm_scan, norm_path]) == norm_scan
        except ValueError:
            return False

    def _queue_emptied_ancestors(self, deleted_path, pending, scheduled, log_signal):
        parent = os.path.dirname(deleted_path)
        while parent and self._is_within_scan_root(parent):
            norm_parent = os.path.normpath(parent)
            if norm_parent == os.path.normpath(self.scan_dir):
                break
            if norm_parent in scheduled or self._is_ignored(norm_parent):
                break
            try:
                if not os.path.isdir(parent) or os.listdir(parent):
                    break
            except OSError:
                break
            scheduled.add(norm_parent)
            log_signal.emit(self._t("chain_empty", path=parent), False)
            pending.append(parent)
            break

    def delete_folders(self, folder_paths, log_signal, progress_signal, use_trash=False):
        failed = []
        remaining = []
        deleted_paths = []
        scheduled = set()
        pending = deque()
        for path in folder_paths:
            norm = os.path.normpath(path)
            if norm not in scheduled:
                scheduled.add(norm)
                pending.append(path)

        ordered = sorted(
            pending,
            key=lambda p: (self._directory_depth(p), os.path.basename(p)),
            reverse=True,
        )
        pending = deque(ordered)
        completed = 0

        while pending:
            if self.stop_event.is_set():
                log_signal.emit(self._t("stopped"), False)
                remaining = list(pending)
                break

            path = pending.popleft()
            log_signal.emit(self._t("deleting", path=path), False)
            try:
                if os.path.exists(path) and not os.listdir(path):
                    if use_trash and _SEND2TRASH_AVAILABLE:
                        send2trash.send2trash(path)
                        log_signal.emit(self._t("deleted_trash_msg", path=path), False)
                    else:
                        os.rmdir(path)
                        log_signal.emit(self._t("deleted_msg", path=path), False)
                    deleted_paths.append(path)
                    self._queue_emptied_ancestors(
                        path, pending, scheduled, log_signal
                    )
                else:
                    log_signal.emit(self._t("skipped", path=path), False)
            except PermissionError:
                failed.append(path)
                log_signal.emit(self._t("permission_denied", path=path), True)
            except Exception as e:
                failed.append(path)
                log_signal.emit(self._t("delete_error", path=path, error=str(e)), True)

            completed += 1
            progress_signal.emit(completed, completed + len(pending))

        if not remaining:
            log_signal.emit(self._t("all_done"), False)
        progress_signal.emit(0, 0)
        return failed, remaining, deleted_paths

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
    finished_signal = pyqtSignal(list, list, list)
    error_signal = pyqtSignal(str)

    def __init__(self, engine, folder_list, use_trash):
        super().__init__()
        self.engine = engine
        self.folder_list = folder_list
        self.use_trash = use_trash

    def run(self):
        try:
            failed, remaining, deleted_paths = self.engine.delete_folders(
                self.folder_list,
                self.log_signal,
                self.progress_signal,
                use_trash=self.use_trash
            )
            self.finished_signal.emit(failed, remaining, deleted_paths)
        except Exception as e:
            self.error_signal.emit(str(e))
