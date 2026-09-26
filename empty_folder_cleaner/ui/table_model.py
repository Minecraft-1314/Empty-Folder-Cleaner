"""Table model and sort/filter proxy for empty folder results."""

from PyQt6.QtCore import (
    QAbstractTableModel, QModelIndex, QSortFilterProxyModel, Qt
)

from ..core.i18n import I18n


class FolderTableModel(QAbstractTableModel):
    HEADER_KEYS = ("path", "modified", "status", "select")

    def __init__(self, lang="en", parent=None):
        super().__init__(parent)
        self._lang = lang
        self._rows = []
        self._path_rows = {}
        self._selected_paths = set()
        self._selectable = True

    def rowCount(self, parent=QModelIndex()):
        return 0 if parent.isValid() else len(self._rows)

    def columnCount(self, parent=QModelIndex()):
        return 0 if parent.isValid() else len(self.HEADER_KEYS)

    def data(self, index, role=Qt.ItemDataRole.DisplayRole):
        if not index.isValid():
            return None
        path, mtime_text = self._rows[index.row()]
        column = index.column()
        if role == Qt.ItemDataRole.DisplayRole:
            if column == 0:
                return path
            if column == 1:
                return mtime_text
            if column == 2:
                return I18n.get_text("not_deleted", self._lang)
        if role == Qt.ItemDataRole.ToolTipRole and column == 0:
            return path
        if role == Qt.ItemDataRole.CheckStateRole and column == 3:
            return (
                Qt.CheckState.Checked
                if path in self._selected_paths
                else Qt.CheckState.Unchecked
            )
        return None

    def headerData(self, section, orientation, role=Qt.ItemDataRole.DisplayRole):
        if (
            orientation == Qt.Orientation.Horizontal
            and role == Qt.ItemDataRole.DisplayRole
            and 0 <= section < len(self.HEADER_KEYS)
        ):
            return I18n.get_text(self.HEADER_KEYS[section], self._lang)
        return None

    def flags(self, index):
        if not index.isValid():
            return Qt.ItemFlag.NoItemFlags
        flags = Qt.ItemFlag.ItemIsEnabled
        if index.column() == 3 and self._selectable:
            flags |= Qt.ItemFlag.ItemIsUserCheckable
        return flags

    def setData(self, index, value, role=Qt.ItemDataRole.EditRole):
        if (
            not index.isValid()
            or index.column() != 3
            or role != Qt.ItemDataRole.CheckStateRole
            or not self._selectable
        ):
            return False
        if isinstance(value, int):
            state = Qt.CheckState(value)
        else:
            state = value
        path = self._rows[index.row()][0]
        was_selected = path in self._selected_paths
        if state == Qt.CheckState.Checked:
            self._selected_paths.add(path)
        else:
            self._selected_paths.discard(path)
        if was_selected == (path in self._selected_paths):
            return True
        self.dataChanged.emit(index, index, [Qt.ItemDataRole.CheckStateRole])
        return True

    def add_folder(self, path, mtime_text=""):
        if path in self._path_rows:
            return False
        row = len(self._rows)
        self.beginInsertRows(QModelIndex(), row, row)
        self._rows.append((path, mtime_text))
        self._path_rows[path] = row
        self.endInsertRows()
        return True

    def add_folders(self, paths):
        """Insert many rows under a single reset, avoiding per-row view churn."""
        fresh = []
        seen = set()
        for path, mtime_text in paths:
            if path in self._path_rows or path in seen:
                continue
            seen.add(path)
            fresh.append((path, mtime_text))
        if not fresh:
            return 0
        self.beginResetModel()
        for path, mtime_text in fresh:
            self._path_rows[path] = len(self._rows)
            self._rows.append((path, mtime_text))
        self.endResetModel()
        return len(fresh)

    def remove_paths(self, paths):
        paths = set(paths)
        if not paths:
            return False
        if not any(path in paths for path, _ in self._rows):
            return False
        self.beginResetModel()
        self._rows = [(path, mtime) for path, mtime in self._rows if path not in paths]
        self._path_rows = {
            path: row for row, (path, _) in enumerate(self._rows)
        }
        self._selected_paths.difference_update(paths)
        self.endResetModel()
        return True

    def clear(self):
        if not self._rows and not self._selected_paths:
            return
        self.beginResetModel()
        self._rows.clear()
        self._path_rows.clear()
        self._selected_paths.clear()
        self.endResetModel()

    def all_paths(self):
        return [path for path, _ in self._rows]

    def path_at_row(self, row):
        if 0 <= row < len(self._rows):
            return self._rows[row][0]
        return ""

    def has_path(self, path):
        return path in self._path_rows

    def set_language(self, lang):
        if lang == self._lang:
            return
        self._lang = lang
        if self._rows:
            top_left = self.index(0, 2)
            bottom_right = self.index(self.rowCount() - 1, 2)
            self.dataChanged.emit(
                top_left, bottom_right, [Qt.ItemDataRole.DisplayRole]
            )
        self.headerDataChanged.emit(
            Qt.Orientation.Horizontal, 0, self.columnCount() - 1
        )

    def set_selectable(self, selectable):
        if self._selectable == selectable:
            return
        self._selectable = selectable
        if self._rows:
            top_left = self.index(0, 3)
            bottom_right = self.index(self.rowCount() - 1, 3)
            self.dataChanged.emit(
                top_left, bottom_right, [Qt.ItemDataRole.CheckStateRole]
            )

    def selected_paths(self):
        return set(self._selected_paths)

    def selected_count(self):
        return len(self._selected_paths)

    def select_all(self):
        if not self._rows:
            return
        self._selected_paths = set(self.all_paths())
        self._emit_selection_changed()

    def clear_selection(self):
        if not self._selected_paths:
            return
        self._selected_paths.clear()
        self._emit_selection_changed()

    def set_selected_paths(self, paths):
        self._selected_paths = set(paths)
        self._emit_selection_changed()

    def _emit_selection_changed(self):
        if not self._rows:
            return
        top_left = self.index(0, 3)
        bottom_right = self.index(self.rowCount() - 1, 3)
        self.dataChanged.emit(
            top_left, bottom_right, [Qt.ItemDataRole.CheckStateRole]
        )


class FolderProxyModel(QSortFilterProxyModel):
    def __init__(self, parent=None):
        super().__init__(parent)
        self._path_filter = ""
        self.setDynamicSortFilter(True)
        self.setFilterCaseSensitivity(Qt.CaseSensitivity.CaseInsensitive)

    def set_path_filter(self, text):
        self._path_filter = text.strip().lower()
        self.invalidateFilter()

    def filterAcceptsRow(self, source_row, source_parent):
        if not self._path_filter:
            return True
        source = self.sourceModel()
        if source is None:
            return True
        index = source.index(source_row, 0, source_parent)
        path = index.data(Qt.ItemDataRole.DisplayRole) or ""
        return self._path_filter in path.lower()

    def lessThan(self, left, right):
        left_value = left.data() or ""
        right_value = right.data() or ""
        if left.column() == 0:
            return left_value.lower() < right_value.lower()
        if left.column() == 1:
            return left_value < right_value
        return super().lessThan(left, right)
