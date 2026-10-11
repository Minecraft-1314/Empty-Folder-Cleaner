export const RowStatus = Object.freeze({
  Pending: "pending",
  Deleted: "deleted",
  Trashed: "trashed",
  Remaining: "remaining",
  Failed: "failed"
});

export const SortKey = Object.freeze({
  Path: "path",
  Modified: "modified",
  Size: "size"
});

const DEFAULT_CLUSTER_SIZE = 4096;

export class Store {
  constructor(clusterSize = DEFAULT_CLUSTER_SIZE) {
    this.clusterSize = clusterSize;
    this.rows = new Map();
    this.selected = new Set();
    this.filterText = "";
    this.sortKey = SortKey.Path;
    this.sortAscending = true;
    this.sortEnabled = true;
    this.busy = null;
    this.progress = { completed: 0, total: 0, active: false };
    this.scanStats = { visited: 0, empty: 0 };
    this.settings = {
      language: "en-US",
      themeMode: "system",
      useRecycleBin: true,
      scanDirectory: "",
      recentDirectories: [],
      ignoredPaths: [],
      ignorePatterns: []
    };
    this.capabilities = { recycleBin: true, hostAvailable: true, darkSystemTheme: false };
    this.listeners = new Map();
    this.visibleRowsCache = [];
    this.visibleRowsDirty = true;
  }

  on(event, listener) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event).add(listener);
    return () => this.listeners.get(event)?.delete(listener);
  }

  emit(event, payload) {
    for (const listener of this.listeners.get(event) ?? []) {
      try {
        listener(payload);
      } catch {
        continue;
      }
    }
  }

  applyState(state) {
    const settings = state?.settings ?? {};
    this.settings = {
      language: settings.language || "en-US",
      themeMode: settings.themeMode || "system",
      useRecycleBin: settings.useRecycleBin === true,
      scanDirectory: settings.scanDirectory || "",
      recentDirectories: Array.isArray(settings.recentDirectories) ? settings.recentDirectories : [],
      ignoredPaths: Array.isArray(settings.ignoredPaths) ? settings.ignoredPaths : [],
      ignorePatterns: Array.isArray(settings.ignorePatterns) ? settings.ignorePatterns : []
    };
    this.capabilities = {
      recycleBin: state?.isRecycleBinAvailable !== false,
      hostAvailable: true,
      darkSystemTheme: state?.isDarkSystemTheme === true
    };
    this.invalidateVisibleRows();
    this.emit("settings", this.settings);
    this.emit("rows", this.visibleRows());
    this.emit("selection", this.selectionSummary());
  }

  resetRows() {
    this.rows.clear();
    this.selected.clear();
    this.scanStats = { visited: 0, empty: 0 };
    this.invalidateVisibleRows();
    this.emit("rows", []);
    this.emit("selection", this.selectionSummary());
  }

  addRows(entries) {
    let added = 0;
    for (const entry of entries ?? []) {
      const path = typeof entry?.path === "string" ? entry.path : "";
      if (path.length === 0 || this.rows.has(path)) {
        continue;
      }
      this.rows.set(path, {
        path,
        modified: entry.modifiedUtc ?? null,
        status: RowStatus.Pending
      });
      added++;
    }
    if (added > 0) {
      this.invalidateVisibleRows();
      this.emit("rows", this.visibleRows());
    }
  }

  setScanStats(visited, empty) {
    this.scanStats = { visited, empty };
  }

  setBusy(busy) {
    this.busy = busy;
    this.emit("busy", busy);
  }

  setProgress(completed, total, active = true) {
    this.progress = { completed, total, active };
    this.emit("progress", this.progress);
  }

  setFilter(text) {
    this.filterText = (text ?? "").trim().toLowerCase();
    this.invalidateVisibleRows();
    this.emit("rows", this.visibleRows());
  }

  toggleSort(key) {
    if (this.sortKey === key) {
      this.sortAscending = !this.sortAscending;
    } else {
      this.sortKey = key;
      this.sortAscending = key === SortKey.Path;
    }
    this.invalidateVisibleRows();
    this.emit("rows", this.visibleRows());
  }

  toggleRow(path) {
    if (!this.rows.has(path)) {
      return;
    }
    if (this.selected.has(path)) {
      this.selected.delete(path);
    } else {
      this.selected.add(path);
    }
    this.emit("selection", this.selectionSummary());
  }

  selectAll() {
    for (const row of this.visibleRows()) {
      if (row.status === RowStatus.Pending) {
        this.selected.add(row.path);
      }
    }
    this.emit("selection", this.selectionSummary());
  }

  clearSelection() {
    this.selected.clear();
    this.emit("selection", this.selectionSummary());
  }

  applyDeleteResults(report) {
    const pathsFor = (pascalCase, camelCase) => report?.[pascalCase] ?? report?.[camelCase] ?? [];
    const mark = (paths, status) => {
      for (const path of paths ?? []) {
        const row = this.rows.get(path);
        if (row && row.status === RowStatus.Pending) {
          row.status = status;
        }
      }
    };
    const deletedPaths = pathsFor("DeletedPaths", "deletedPaths");
    const failedPaths = pathsFor("FailedPaths", "failedPaths");
    const remainingPaths = pathsFor("RemainingPaths", "remainingPaths");
    mark(deletedPaths, RowStatus.Deleted);
    mark(failedPaths, RowStatus.Failed);
    mark(remainingPaths, RowStatus.Remaining);
    this.settings.ignoredPaths = [
      ...new Set([...this.settings.ignoredPaths, ...failedPaths])
    ];
    this.selected.clear();
    this.invalidateVisibleRows();
    this.emit("settings", this.settings);
    this.emit("rows", this.visibleRows());
    this.emit("selection", this.selectionSummary());
  }

  selectionSummary() {
    const paths = this.selectedPaths();
    return { count: paths.length, paths, size: paths.length * this.clusterSize };
  }

  selectedPaths() {
    const paths = [];
    for (const path of this.selected) {
      const row = this.rows.get(path);
      if (row && row.status === RowStatus.Pending) {
        paths.push(path);
      }
    }
    return paths;
  }

  pendingPaths() {
    const paths = [];
    for (const row of this.rows.values()) {
      if (row.status === RowStatus.Pending) {
        paths.push(row.path);
      }
    }
    return paths;
  }

  visibleRows() {
    if (!this.visibleRowsDirty) {
      return this.visibleRowsCache;
    }

    const rows = [...this.rows.values()];
    const filter = this.filterText;
    const filtered = filter ? rows.filter((row) => row.path.toLowerCase().includes(filter)) : rows;
    if (!this.sortEnabled) {
      return filtered;
    }
    const direction = this.sortAscending ? 1 : -1;
    const key = this.sortKey;
    filtered.sort((left, right) => direction * compareRows(left, right, key));
    this.visibleRowsCache = filtered;
    this.visibleRowsDirty = false;
    return this.visibleRowsCache;
  }

  setClusterSize(clusterSize) {
    if (Number.isFinite(clusterSize) && clusterSize > 0) {
      this.clusterSize = clusterSize;
      this.emit("selection", this.selectionSummary());
    }
  }

  setSortEnabled(enabled) {
    if (this.sortEnabled === enabled) {
      return;
    }
    this.sortEnabled = enabled;
    this.invalidateVisibleRows();
    this.emit("rows", this.visibleRows());
  }

  ignorePath(path) {
    if (!this.settings.ignoredPaths.includes(path)) {
      this.settings.ignoredPaths = [...this.settings.ignoredPaths, path];
      this.emit("settings", this.settings);
    }
  }

  snapshotForSave() {
    return {
      language: this.settings.language,
      themeMode: this.settings.themeMode,
      useRecycleBin: this.settings.useRecycleBin,
      scanDirectory: this.settings.scanDirectory,
      recentDirectories: this.settings.recentDirectories,
      ignoredPaths: this.settings.ignoredPaths,
      ignorePatterns: this.settings.ignorePatterns
    };
  }

  invalidateVisibleRows() {
    this.visibleRowsDirty = true;
  }
}

function compareRows(left, right, key) {
  if (key === SortKey.Modified) {
    const leftValue = left.modified ? Date.parse(left.modified) : 0;
    const rightValue = right.modified ? Date.parse(right.modified) : 0;
    if (leftValue !== rightValue) {
      return leftValue - rightValue;
    }
  }
  return left.path.localeCompare(right.path);
}
