import { HostBridge } from "./bridge.js";
import { debounce } from "./dom.js";
import { I18n } from "./i18n.js";
import { Store } from "./store.js";
import { createHeader } from "./views/header.js";
import { ModalHost } from "./views/modal.js";
import {
  openConfirmDialog,
  openRowMenu,
  openRulesDialog,
  openSettingsDialog
} from "./views/dialogs.js";
import { createResults } from "./views/results.js";
import { createToolbar } from "./views/toolbar.js";

const MAX_RECENT = 8;
const REFRESH_FALLBACK_MS = 50;

class Controller {
  constructor(bridge, i18n, store) {
    this.bridge = bridge;
    this.i18n = i18n;
    this.store = store;
    this.statusMessage = "";
    this.settingsPath = "";
    this.configExists = false;
    this.views = {};
    this.isBootstrapping = true;
    this.isRestoringSettings = false;
    this.refreshFrame = null;
    this.refreshTimer = null;
    this.lastPersistedSettings = null;
    this.settingsSaveEpoch = 0;
    this.settingsSaveQueue = Promise.resolve(true);
  }

  t(key, params) {
    return this.i18n.t(key, params);
  }

  currentFilter() {
    return this.store.filterText;
  }

  describe(code) {
    const key = `error.${code}`;
    const text = this.i18n.t(key);
    return text === key ? this.i18n.t("error.operation_failed") : text;
  }

  setStatus(message) {
    this.statusMessage = message ?? "";
    this.refresh();
  }

  refresh() {
    if (this.refreshFrame !== null || this.refreshTimer !== null) {
      return;
    }

    this.refreshFrame = requestAnimationFrame(() => {
      this.flushRefresh();
    });
    this.refreshTimer = setTimeout(() => this.flushRefresh(), REFRESH_FALLBACK_MS);
  }

  flushRefresh() {
    const frame = this.refreshFrame;
    const timer = this.refreshTimer;
    this.refreshFrame = null;
    this.refreshTimer = null;
    if (frame !== null) {
      cancelAnimationFrame(frame);
    }
    if (timer !== null) {
      clearTimeout(timer);
    }
    this.refreshNow();
  }

  refreshNow() {
    const state = this.snapshot();
    const rows = this.store.visibleRows();
    this.views.header?.update(state);
    this.views.toolbar?.update(state);
    this.views.footer?.update(state);
    this.results?.setData(rows, new Set(this.store.selected), { key: this.store.sortKey, ascending: this.store.sortAscending }, {
      visible: rows.length,
      total: this.store.rows.size
    });
  }

  snapshot() {
    return {
      settings: this.store.settings,
      settingsPath: this.settingsPath,
      configExists: this.configExists,
      capabilities: this.store.capabilities,
      busy: this.store.busy,
      progress: this.store.progress,
      selection: this.store.selectionSummary(),
      rowCount: this.store.rows.size,
      statusMessage: this.statusMessage
    };
  }

  applyTheme() {
    const mode = this.store.settings.themeMode;
    const isDark = mode === "dark" || (mode === "system" && this.store.capabilities.darkSystemTheme);
    document.documentElement.classList.toggle("dark", isDark);
  }

  async bootstrap() {
    const root = document.getElementById("app");
    const modal = new ModalHost(document.getElementById("modal-root"));
    this.modal = modal;

    const viewState = { language: this.i18n.language };
    this.views.header = createHeader(this);
    this.views.toolbar = createToolbar(this);
    this.results = createResults(this);
    const workspace = document.createElement("main");
    workspace.className = "workspace mx-auto grid min-h-0 w-full max-w-[1800px] gap-3 p-3 sm:gap-4 sm:p-4";
    const sidebar = document.createElement("aside");
    sidebar.className = "glass-panel flex min-h-0 flex-col overflow-hidden rounded-xl border border-white/60 shadow-md dark:border-white/10";
    sidebar.append(this.views.toolbar.node);
    const detail = document.createElement("section");
    detail.className = "detail-pane glass-panel flex min-h-0 flex-col gap-3 overflow-hidden rounded-xl border border-white/60 shadow-md dark:border-white/10";
    detail.append(this.results.node);
    workspace.append(sidebar, detail);
    const footer = document.createElement("footer");
    footer.className = "glass-footer z-20 px-4 py-2.5 sm:px-6";
    const footerRow = document.createElement("div");
    footerRow.className = "mx-auto flex max-w-[1800px] flex-wrap items-center gap-x-5 gap-y-2 text-xs text-slate-500 dark:text-slate-400";
    const footerRows = document.createElement("span");
    footerRows.className = "font-medium text-slate-700 dark:text-slate-200";
    const footerSelection = document.createElement("span");
    const footerStatus = document.createElement("span");
    footerStatus.className = "ml-auto max-w-[42vw] truncate text-right";
    footerRow.append(footerRows, footerSelection, footerStatus);
    footer.append(footerRow);
    const controller = this;
    this.views.footer = {
      node: footer,
      update(state) {
        footerRows.textContent = controller.t("list.count", { count: state.rowCount });
        footerSelection.textContent = controller.t("selection.count", { count: state.selection.count });
        footerStatus.textContent = state.statusMessage ?? "";
      }
    };
    root.append(this.views.header.node, workspace, footer);

    this.i18n.onChange(async (language) => {
      this.store.settings.language = language;
      this.applyTheme();
      this.localizeAll();
      if (!this.isBootstrapping && !this.isRestoringSettings) {
        await this.persistSettings();
      }
    });

    this.localizeAll();
    this.applyTheme();
    this.bridge.onEvent((type, payload) => this.handleEvent(type, payload));
    await this.loadState();
    this.bindShortcuts();
    this.isBootstrapping = false;
    if (!this.statusMessage) {
      this.setStatus(this.t("scan.empty"));
    }
  }

  localizeAll() {
    for (const view of Object.values(this.views)) {
      view?.updateLabels?.();
    }
    this.results?.updateLabels();
    this.refresh();
  }

  async loadState() {
    if (!this.bridge.isAvailable) {
      this.store.capabilities.hostAvailable = false;
      this.applyTheme();
      this.refresh();
      this.setStatus(this.t("warning.hostUnavailable"));
      return;
    }
    try {
      const state = await this.bridge.invoke("app.getState");
      this.store.applyState(state);
      this.lastPersistedSettings = clonePersistedSettings(this.store.snapshotForSave());
      this.settingsPath = state?.settingsPath ?? "";
      this.configExists = state?.configExists === true;
      await this.i18n.setLanguage(state?.effectiveLanguage ?? this.i18n.language);
      this.applyTheme();
      if (state?.loadWarning) {
        this.describeWarning(state.loadWarning);
      }
      this.refresh();
    } catch (error) {
      this.store.capabilities.hostAvailable = false;
      this.applyTheme();
      this.refresh();
      this.setStatus(this.describe(error?.code ?? "host_unavailable"));
    }
  }

  describeWarning(code) {
    const map = {
      legacy_settings_detected: "warning.legacy",
      settings_corrupt: "warning.corrupt",
      settings_recovered_from_backup: "warning.recovered",
      settings_empty: "warning.corrupt",
      settings_upgraded: "warning.legacy"
    };
    const key = map[code];
    if (key) {
      this.setStatus(this.t(key));
    }
  }

  bindShortcuts() {
    document.addEventListener("keydown", (event) => {
      const target = event.target;
      const isTextField =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLSelectElement;
      if (event.key === "F5") {
        event.preventDefault();
        this.onScan();
        return;
      }
      if (event.key === "Escape") {
        if (this.modal.isOpen) {
          return;
        }
        if (isTextField && target.value) {
          target.value = "";
          target.dispatchEvent(new Event("input", { bubbles: true }));
          return;
        }
        this.onStop();
        return;
      }
      if (event.key.toLowerCase() === "a" && (event.ctrlKey || event.metaKey) && !isTextField) {
        event.preventDefault();
        this.onSelectAll();
      }
    });
  }

  handleEvent(type, payload) {
    switch (type) {
      case "scanStarted":
        this.store.resetRows();
        this.store.setClusterSize(payload?.clusterSize ?? this.store.clusterSize);
        this.store.setBusy("scan");
        this.store.setSortEnabled(false);
        this.store.setProgress(0, 0, false);
        this.statusMessage = this.t("scan.running", { count: 0 });
        this.refresh();
        break;
      case "scanBatch":
        this.store.addRows(payload?.entries ?? []);
        this.statusMessage = this.t("scan.running", { count: this.store.rows.size });
        this.refresh();
        break;
      case "scanProgress":
        this.store.setScanStats(payload?.visitedDirectories ?? 0, payload?.emptyFolders ?? 0);
        break;
      case "scanCompleted":
        this.store.setScanStats(payload?.stats?.visitedDirectories ?? 0, payload?.stats?.emptyFolders ?? 0);
        break;
      case "scanFinished":
        this.finishScan(false);
        break;
      case "scanStopped":
        this.finishScan(true);
        break;
      case "deleteStarted":
        this.store.setBusy("delete");
        this.store.setProgress(0, payload?.total ?? 0, true);
        this.refresh();
        break;
      case "deleteProgress":
        this.store.setProgress(payload?.completed ?? 0, payload?.total ?? 0, true);
        break;
      case "deleteCompleted":
        this.store.applyDeleteResults(payload);
        this.store.setBusy(null);
        this.store.setProgress(0, 0, false);
        this.finishDelete(payload);
        this.refresh();
        break;
      case "settingsSaved":
        this.settingsPath = payload?.settingsPath ?? this.settingsPath;
        this.configExists = true;
        break;
      case "settingsReset":
        if (payload?.settings) {
          this.store.settings = { ...this.store.settings, ...payload.settings };
        }
        this.store.settings.themeMode = "system";
        this.store.settings.useRecycleBin = true;
        this.store.settings.scanDirectory = "";
        this.store.settings.recentDirectories = [];
        this.store.settings.ignoredPaths = [];
        this.store.settings.ignorePatterns = [];
        this.settingsPath = "";
        this.configExists = false;
        this.applyTheme();
        this.store.resetRows();
        this.statusMessage = this.t("notice.settingsReset");
        this.localizeAll();
        this.refresh();
        break;
      case "operationFailed":
        this.store.setBusy(null);
        this.store.setProgress(0, 0, false);
        this.setStatus(this.describe(payload?.code ?? "operation_failed"));
        break;
      default:
        break;
    }
  }

  finishScan(wasCancelled) {
    const { visited, empty } = this.store.scanStats;
    this.store.setSortEnabled(true);
    this.store.setBusy(null);
    this.store.setProgress(0, 0, false);
    const count = this.store.rows.size;
    this.statusMessage = wasCancelled
      ? this.t("scan.stopped", { count })
      : this.t("scan.done", { total: visited, empty });
    this.refresh();
  }

  finishDelete(report) {
    const pathsFor = (pascalCase, camelCase) => report?.[pascalCase] ?? report?.[camelCase] ?? [];
    const deleted = pathsFor("DeletedPaths", "deletedPaths").length;
    const failed = pathsFor("FailedPaths", "failedPaths").length;
    const remaining = pathsFor("RemainingPaths", "remainingPaths").length;
    if (report?.WasCancelled) {
      this.statusMessage = this.t("notice.stopped");
    } else if (remaining > 0) {
      this.statusMessage = this.t("notice.stopped");
    } else if (failed > 0) {
      this.statusMessage = this.t("notice.doneWithErrors", { count: failed });
    } else if (deleted > 0) {
      this.statusMessage = this.t("notice.allDone", { count: deleted });
    } else {
      this.statusMessage = "";
    }
    if (failed > 0) {
      void this.persistSettings();
    }
  }

  onDirectoryInput(value) {
    this.store.settings.scanDirectory = value;
  }

  onSelectDirectory(directory) {
    this.store.settings.scanDirectory = directory;
    this.rememberRecent(directory);
    this.refresh();
    if (this.store.busy === null) {
      void this.runScan(directory);
    }
  }

  async onBrowse() {
    try {
      const result = await this.bridge.invoke("app.chooseDirectory");
      if (result?.path) {
        this.onSelectDirectory(result.path);
      }
    } catch (error) {
      this.setStatus(this.describe(error?.code ?? "operation_failed"));
    }
  }

  onScan() {
    if (this.store.busy !== null) {
      this.setStatus(this.t("error.operation_busy"));
      return;
    }
    const directory = (this.store.settings.scanDirectory ?? "").trim();
    if (directory.length === 0) {
      this.views.toolbar.focusDirectory();
      this.setStatus(this.t("error.invalid_input"));
      return;
    }
    void this.runScan(directory);
  }

  async runScan(directory) {
    try {
      await this.bridge.invoke("scan.start", {
        root: directory,
        ignoredPaths: this.store.settings.ignoredPaths,
        ignorePatterns: this.store.settings.ignorePatterns
      });
      this.rememberRecent(directory);
      await this.persistSettings();
    } catch (error) {
      this.store.setBusy(null);
      this.setStatus(this.describe(error?.code ?? "operation_failed"));
    }
  }

  onStop() {
    if (this.store.busy === null) {
      return;
    }
    void this.bridge.invoke("operation.stop").catch(() => undefined);
  }

  onFilter(value) {
    this.store.setFilter(value);
    this.refresh();
  }

  onSort(key) {
    this.store.toggleSort(key);
    this.refresh();
  }

  onToggleRow(path) {
    this.store.toggleRow(path);
    this.refresh();
  }

  onSelectAll() {
    this.store.selectAll();
    this.refresh();
  }

  onDeselectAll() {
    this.store.clearSelection();
    this.refresh();
  }

  onRecycleChange(enabled) {
    this.store.settings.useRecycleBin = enabled;
    this.refresh();
    void this.persistSettings();
  }

  onDelete() {
    const paths = this.store.selectedPaths();
    if (paths.length === 0) {
      this.setStatus(this.t("error.no_paths"));
      return;
    }
    openConfirmDialog(this.modal, this, paths);
  }

  onConfirmDelete(paths) {
    if (this.store.busy !== null) {
      this.setStatus(this.t("error.operation_busy"));
      return;
    }
    void this.bridge
      .invoke("delete.start", { paths, useRecycleBin: this.store.settings.useRecycleBin })
      .catch((error) => {
        this.store.setBusy(null);
        this.setStatus(this.describe(error?.code ?? "operation_failed"));
      });
  }

  openRules() {
    openRulesDialog(this.modal, this, this.store.settings.ignorePatterns);
  }

  async onApplyRules(patterns) {
    this.store.settings.ignorePatterns = patterns;
    const saved = await this.persistSettings();
    if (!saved) {
      return;
    }
    if (this.store.settings.scanDirectory) {
      await this.runScan(this.store.settings.scanDirectory);
    }
  }

  openSettings() {
    openSettingsDialog(this.modal, this, this.snapshot());
  }

  async onSaveSettings() {
    const saved = await this.persistSettings();
    if (saved) {
      this.setStatus(this.t("notice.saved"));
    }
    return saved;
  }

  async onResetSettings() {
    try {
      await this.bridge.invoke("settings.reset");
      return true;
    } catch (error) {
      this.setStatus(this.describe(error?.code ?? "operation_failed"));
      return false;
    }
  }

  async onLanguageChange(language) {
    try {
      await this.i18n.setLanguage(language);
    } catch {
      this.setStatus(this.t("error.operation_failed"));
    }
  }

  async onThemeChange(themeMode) {
    this.store.settings.themeMode = themeMode;
    this.applyTheme();
    await this.persistSettings();
  }

  onOpenFolder(path) {
    void this.bridge.invoke("shell.openFolder", path).catch((error) =>
      this.setStatus(this.describe(error?.code ?? "operation_failed"))
    );
  }

  onCopyPath(path) {
    void this.bridge
      .invoke("shell.copyPath", path)
      .then(() => this.setStatus(this.t("notice.copyPath")))
      .catch((error) => this.setStatus(this.describe(error?.code ?? "operation_failed")));
  }

  async onIgnoreFolder(path) {
    this.store.ignorePath(path);
    this.setStatus(this.t("notice.ignoreAdded"));
    const saved = await this.persistSettings();
    if (!saved) {
      return;
    }
    if (this.store.settings.scanDirectory) {
      await this.runScan(this.store.settings.scanDirectory);
    }
  }

  onRowMenu(path, position) {
    openRowMenu(this.modal, this, path, position);
  }

  rememberRecent(directory) {
    const trimmed = directory.trim();
    if (trimmed.length === 0) {
      return;
    }
    const next = [trimmed, ...this.store.settings.recentDirectories.filter((item) => item !== trimmed)];
    this.store.settings.recentDirectories = next.slice(0, MAX_RECENT);
  }

  persistSettings() {
    if (!this.bridge.isAvailable) {
      this.setStatus(this.t("warning.hostUnavailable"));
      return Promise.resolve(false);
    }
    const epoch = this.settingsSaveEpoch;
    const previous = this.settingsSaveQueue;
    const operation = previous.then(async () => {
      if (epoch !== this.settingsSaveEpoch) {
        return false;
      }
      return this.executeSettingsSave();
    });
    this.settingsSaveQueue = operation.catch(() => false);
    return operation.catch(() => false);
  }

  async executeSettingsSave() {
    const desired = clonePersistedSettings(this.store.snapshotForSave());
    try {
      const result = await this.bridge.invoke("settings.save", desired);
      if (result?.saved !== true) {
        await this.handleSettingsSaveFailure(result?.warning ?? "operation_failed");
        return false;
      }
      this.lastPersistedSettings = desired;
      return true;
    } catch (error) {
      await this.handleSettingsSaveFailure(error?.code ?? "operation_failed");
      return false;
    }
  }

  async handleSettingsSaveFailure(code) {
    this.settingsSaveEpoch += 1;
    if (this.lastPersistedSettings) {
      await this.restoreSettings(this.lastPersistedSettings);
    }
    this.setStatus(this.describe(code));
  }

  async restoreSettings(previous) {
    const settings = this.store.settings;
    settings.language = previous.language;
    settings.themeMode = previous.themeMode;
    settings.useRecycleBin = previous.useRecycleBin;
    settings.scanDirectory = previous.scanDirectory;
    settings.recentDirectories = [...previous.recentDirectories];
    settings.ignoredPaths = [...previous.ignoredPaths];
    settings.ignorePatterns = [...previous.ignorePatterns];
    this.applyTheme();
    if (this.i18n.language !== previous.language) {
      this.isRestoringSettings = true;
      try {
        await this.i18n.setLanguage(previous.language);
      } catch {
        this.store.settings.language = this.i18n.language;
      } finally {
        this.isRestoringSettings = false;
      }
    }
    this.localizeAll();
    this.refresh();
  }
}

function clonePersistedSettings(settings) {
  return {
    language: settings.language,
    themeMode: settings.themeMode,
    useRecycleBin: settings.useRecycleBin,
    scanDirectory: settings.scanDirectory,
    recentDirectories: [...settings.recentDirectories],
    ignoredPaths: [...settings.ignoredPaths],
    ignorePatterns: [...settings.ignorePatterns]
  };
}

async function main() {
  const i18n = new I18n();
  try {
    await i18n.setLanguage(navigator.language?.toLowerCase().startsWith("zh") ? "zh-CN" : "en-US");
  } catch {
    await i18n.setLanguage("en-US");
  }
  const store = new Store();
  const controller = new Controller(new HostBridge(), i18n, store);
  window.__audit?.setController?.(controller);
  await controller.bootstrap();
}

main().catch(() => {
  document.getElementById("app").textContent = "The interface could not start.";
});
