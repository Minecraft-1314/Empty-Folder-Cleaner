import { button, clear, el, field, formatSize, icon, swapIcon } from "../dom.js";

const MAX_RECENT_CHIPS = 6;

export function createToolbar(controller) {
  const directoryInput = field({
    id: "scan-directory",
    placeholder: controller.t("scan.label"),
    oninput: () => controller.onDirectoryInput(directoryInput.value)
  });

  const browseButton = button({
    label: controller.t("scan.browse"),
    iconName: "folder",
    onClick: () => controller.onBrowse()
  });

  const scanButton = button({
    label: controller.t("scan.start"),
    iconName: "refresh",
    variant: "primary",
    onClick: () => controller.onScan()
  });

  const stopButton = button({
    label: controller.t("scan.stop"),
    iconName: "stop",
    variant: "danger",
    className: "hidden",
    onClick: () => controller.onStop()
  });

  const searchInput = field({
    id: "result-filter",
    type: "search",
    placeholder: controller.t("scan.search"),
    oninput: () => controller.onFilter(searchInput.value)
  });

  const selectAllButton = button({
    label: controller.t("actions.selectAll"),
    iconName: "check",
    onClick: () => controller.onSelectAll()
  });

  const deselectAllButton = button({
    label: controller.t("actions.deselectAll"),
    iconName: "close",
    onClick: () => controller.onDeselectAll()
  });

  const deleteButton = button({
    label: controller.t("actions.delete"),
    iconName: "trash",
    variant: "danger",
    onClick: () => controller.onDelete()
  });

  const stopDeleteButton = button({
    label: controller.t("actions.stop"),
    iconName: "stop",
    variant: "danger",
    className: "hidden",
    onClick: () => controller.onStop()
  });

  const rulesButton = button({
    label: controller.t("actions.rules"),
    iconName: "rules",
    onClick: () => controller.openRules()
  });

  const recycleSwitch = el("input", {
    type: "checkbox",
    class: "switch",
    id: "recycle-switch",
    onchange: (event) => controller.onRecycleChange(event.target.checked)
  });

  const recycleLabel = el("label", { class: "text-sm text-slate-600 dark:text-slate-300" });
  const recycleRow = el("div", { class: "flex items-center gap-3" }, [recycleSwitch, recycleLabel]);

  const recentRow = el("div", { class: "flex flex-wrap items-center gap-2" });
  const statusText = el("p", { class: "text-sm text-slate-600 dark:text-slate-300" });
  const selectionText = el("p", { class: "text-sm font-medium text-teal-700 dark:text-teal-300" });
  const spaceText = el("p", { class: "text-sm text-slate-500 dark:text-slate-400" });
  const progressTrack = el("div", { class: "progress-track hidden" });
  const progressBar = el("div", { class: "progress-bar", style: "width:0%" });
  progressTrack.append(progressBar);
  const progressText = el("p", { class: "text-xs text-slate-500 dark:text-slate-400" });

  const node = el("section", { class: "surface-enter flex flex-col gap-4 p-4 sm:p-5", style: "--surface-index:1" }, [
    el("div", { class: "flex flex-col gap-2" }, [
      el("label", { class: "label whitespace-nowrap", for: "scan-directory", "data-i18n": "scan.label" }),
      el("div", { class: "flex flex-col gap-2 sm:flex-row" }, [
        el("div", { class: "min-w-0 flex-1" }, [directoryInput]),
        browseButton
      ]),
      el("div", { class: "flex items-end gap-2" }, [scanButton, stopButton])
    ]),
    recentRow,
    el("div", { class: "flex flex-col gap-2" }, [
      el("label", { class: "label whitespace-nowrap", for: "result-filter", "data-i18n": "scan.search" }),
      el("div", { class: "flex items-center gap-2" }, [
        el("span", { class: "text-slate-400" }, [icon("search", 18)]),
        searchInput
      ]),
      el("div", { class: "flex flex-wrap items-center gap-2" }, [
        selectAllButton,
        deselectAllButton,
        deleteButton,
        stopDeleteButton,
        rulesButton
      ])
    ]),
    el("div", { class: "flex flex-col gap-3 border-t border-slate-200/70 pt-4 dark:border-slate-700/60" }, [
      el("div", { class: "flex flex-wrap items-center justify-between gap-3" }, [
        el("div", { class: "flex flex-wrap items-center gap-x-4 gap-y-1" }, [statusText, selectionText, spaceText]),
        recycleRow
      ]),
      el("div", { class: "flex flex-col gap-1.5" }, [progressTrack, progressText])
    ])
  ]);

  function setRecent(directories, enabled) {
    clear(recentRow);
    if (directories.length === 0) {
      return;
    }
    const label = el("span", {
      class: "text-xs text-slate-400",
      "data-i18n": "scan.recent"
    });
    recentRow.append(label);
    for (const directory of directories.slice(0, MAX_RECENT_CHIPS)) {
      const chip = button({
        label: directory,
        className: "chip max-w-[16rem]",
        title: directory,
        disabled: !enabled,
        onClick: () => controller.onSelectDirectory(directory)
      });
      recentRow.append(chip);
    }
  }

  return {
    node,
    directoryInput,
    focusDirectory() {
      directoryInput.focus();
    },
    update(state) {
      const t = (key, params) => controller.t(key, params);
      for (const node2 of node.querySelectorAll("[data-i18n]")) {
        const key = node2.getAttribute("data-i18n");
        node2.textContent = t(key);
      }
      directoryInput.placeholder = t("scan.label");
      searchInput.placeholder = t("scan.search");
      setButton(browseButton, t("scan.browse"), "folder");
      setButton(scanButton, t("scan.start"), "refresh");
      setButton(stopButton, t("scan.stop"), "stop");
      setButton(selectAllButton, t("actions.selectAll"), "check");
      setButton(deselectAllButton, t("actions.deselectAll"), "close");
      setButton(deleteButton, t("actions.delete"), "trash");
      setButton(stopDeleteButton, t("actions.stop"), "stop");
      setButton(rulesButton, t("actions.rules"), "rules");
      recycleLabel.textContent = t("actions.recycle");
      recycleSwitch.checked = state.settings.useRecycleBin;
      recycleSwitch.disabled = !state.capabilities.recycleBin;
      recycleSwitch.title = state.capabilities.recycleBin ? "" : t("actions.recycleUnavailable");

      const isScanning = state.busy === "scan";
      const isDeleting = state.busy === "delete";
      const isBusy = state.busy !== null;
      scanButton.classList.toggle("hidden", isScanning);
      stopButton.classList.toggle("hidden", !isScanning);
      deleteButton.classList.toggle("hidden", isDeleting);
      stopDeleteButton.classList.toggle("hidden", !isDeleting);
      scanButton.disabled = isBusy;
      browseButton.disabled = isBusy;
      directoryInput.disabled = isBusy;
      searchInput.disabled = rowsAreEmpty(state);
      selectAllButton.disabled = isBusy || rowsAreEmpty(state);
      deselectAllButton.disabled = isBusy;
      deleteButton.disabled = isBusy || state.selection.count === 0;
      stopButton.disabled = false;
      stopDeleteButton.disabled = false;
      rulesButton.disabled = isBusy;
      if (document.activeElement !== directoryInput) {
        directoryInput.value = state.settings.scanDirectory;
      }
      setRecent(state.settings.recentDirectories, state.busy === null);
      statusText.textContent = state.statusMessage ?? "";
      selectionText.textContent = t("selection.count", { count: state.selection.count });
      spaceText.textContent =
        state.selection.count > 0 ? t("selection.space", { size: formatSize(state.selection.size) }) : "";
      updateProgress(state.progress);
    }
  };

  function updateProgress(progress) {
    const active = Boolean(progress?.active) && progress.total > 0;
    progressTrack.classList.toggle("hidden", !active);
    progressTrack.classList.toggle("progress-indeterminate", active && progress.completed === 0);
    const ratio = active ? Math.min(100, Math.round((progress.completed / progress.total) * 100)) : 0;
    progressBar.style.width = `${ratio}%`;
    progressText.textContent = active
      ? controller.t("progress.overall", { completed: progress.completed, total: progress.total })
      : "";
  }
}

function setButton(node, label, iconName) {
  const span = node.querySelector("span");
  if (span) {
    span.textContent = label;
  }
  node.setAttribute("aria-label", label);
  node.title = label;
  const existing = node.querySelector("svg");
  if (existing) {
    swapIcon(existing, iconName, Number(existing.getAttribute("width")) || 18);
  }
}

function rowsAreEmpty(state) {
  return state.rowCount === 0;
}
