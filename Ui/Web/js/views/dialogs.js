import { button, clear, el, field, icon } from "../dom.js";

const MAX_PREVIEW_ROWS = 12;
export function openConfirmDialog(modal, controller, paths) {
  const preview = paths.slice(0, MAX_PREVIEW_ROWS);
  const list = el("ul", { class: "max-h-40 space-y-1 overflow-y-auto rounded-xl bg-slate-50/80 p-3 text-xs dark:bg-slate-950/50" });
  for (const path of preview) {
    list.append(el("li", { class: "truncate text-slate-600 dark:text-slate-300", title: path, text: path }));
  }
  if (paths.length > preview.length) {
    list.append(
      el("li", {
        class: "text-xs font-medium text-slate-400",
        text: controller.t("confirm.more", { count: paths.length - preview.length })
      })
    );
  }

  modal.open({
    title: controller.t("confirm.title"),
    description: controller.t("confirm.body", { count: paths.length }),
    closeLabel: controller.t("confirm.cancel"),
    body: [
      list
    ],
    footer: [
      button({
        label: controller.t("confirm.cancel"),
        onClick: () => modal.close()
      }),
      button({
        label: controller.t("confirm.accept"),
        iconName: "trash",
        variant: "danger",
        onClick: () => {
          modal.close();
          controller.onConfirmDelete(paths);
        }
      })
    ]
  });
}

export function openRulesDialog(modal, controller, patterns) {
  const input = field({
    id: "rule-pattern",
    placeholder: controller.t("rules.placeholder"),
    oninput: () => applyError()
  });
  const errorText = el("p", { class: "text-xs text-rose-600 dark:text-rose-400" });
  const listNode = el("ul", { class: "max-h-56 space-y-1 overflow-y-auto rounded-xl bg-slate-50/80 p-2 dark:bg-slate-950/50" });
  let items = [...patterns];
  let selectedIndex = -1;

  const submitButton = button({
    label: controller.t("rules.add"),
    iconName: "plus",
    variant: "primary",
    onClick: () => addCurrent()
  });
  const removeButton = button({
    label: controller.t("rules.remove"),
    iconName: "close",
    onClick: () => {
      if (selectedIndex >= 0 && selectedIndex < items.length) {
        items.splice(selectedIndex, 1);
        selectedIndex = Math.min(selectedIndex, items.length - 1);
        renderItems();
      }
    }
  });

  input.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      addCurrent();
      return;
    }
    event.stopPropagation();
  });

  function addCurrent() {
    const value = input.value.trim();
    if (value.length === 0) {
      applyError();
      return;
    }
    if (value.length > 260 || [...value].some((character) => character.charCodeAt(0) < 32)) {
      applyError(true);
      return;
    }
    if (!items.some((item) => item.toLowerCase() === value.toLowerCase())) {
      items.push(value);
    }
    input.value = "";
    applyError(false);
    renderItems();
  }

  function applyError(invalid = false) {
    input.classList.toggle("field-invalid", invalid);
    errorText.textContent = invalid ? controller.t("rules.hint") : "";
  }

  function renderItems() {
    clear(listNode);
    if (items.length === 0) {
      listNode.append(el("li", { class: "px-2 py-3 text-xs text-slate-400", text: controller.t("rules.empty") }));
      return;
    }
    items.forEach((item, index) => {
      const row = el("li", {
        class: `flex cursor-pointer items-center gap-2 rounded-lg px-2 py-2 text-xs transition-colors duration-150 ${
          index === selectedIndex ? "bg-teal-100/80 dark:bg-teal-500/20" : "hover:bg-slate-100 dark:hover:bg-slate-800"
        }`
      });
      const radio = el("input", {
        type: "radio",
        name: "rule-select",
        class: "h-3.5 w-3.5 shrink-0 accent-teal-600",
        checked: index === selectedIndex,
        onchange: () => {
          selectedIndex = index;
          renderItems();
        }
      });
      row.append(radio, el("span", { class: "truncate font-mono text-slate-700 dark:text-slate-200", title: item, text: item }));
      row.addEventListener("click", () => {
        selectedIndex = index;
        renderItems();
      });
      listNode.append(row);
    });
  }

  renderItems();

  modal.open({
    title: controller.t("rules.title"),
    description: controller.t("rules.hint"),
    closeLabel: controller.t("rules.close"),
    translate: (key) => controller.t(key),
    body: [
      el("div", { class: "flex flex-col gap-2 sm:flex-row" }, [input, submitButton]),
      errorText,
      listNode,
      el("div", { class: "flex justify-start" }, [removeButton])
    ],
    footer: [
      button({
        label: controller.t("rules.close"),
        onClick: () => modal.close()
      }),
      button({
        label: controller.t("rules.apply"),
        iconName: "refresh",
        variant: "primary",
        onClick: () => {
          addCurrent();
          modal.close();
          controller.onApplyRules([...items]);
        }
      })
    ]
  });
}

export function openSettingsDialog(modal, controller, state) {
  const sectionTitle = (iconName, labelKey) =>
    el("div", { class: "flex items-center gap-2 text-xs font-semibold text-slate-500 dark:text-slate-400" }, [
      icon(iconName, 16),
      el("span", { "data-i18n": labelKey })
    ]);
  const recycleSwitch = el("input", {
    type: "checkbox",
    class: "switch",
    checked: state.settings.useRecycleBin,
    disabled: !state.capabilities.recycleBin,
    onchange: (event) => controller.onRecycleChange(event.target.checked)
  });
  const warningBox = el("div", { class: "flex items-start gap-2 rounded-xl bg-amber-50 p-3 text-xs text-amber-800 dark:bg-amber-500/10 dark:text-amber-200" });
  if (!state.capabilities.recycleBin) {
    warningBox.append(icon("warning", 16), el("span", { text: controller.t("warning.recycleUnavailable") }));
  }
  const configExists = state.configExists === true;
  const configStatus = el("span", {
    class: `chip ${configExists ? "bg-teal-50 text-teal-700 dark:bg-teal-500/15 dark:text-teal-200" : "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300"}`,
    "data-i18n": configExists ? "settings.config.exists" : "settings.config.missing"
  });
  const configPathText = el("p", {
    class: "break-all font-mono text-xs leading-5 text-slate-600 dark:text-slate-300",
    "data-i18n": configExists ? null : "settings.config.pathMissing",
    title: state.settingsPath ?? "",
    text: state.settingsPath || controller.t("settings.config.pathMissing")
  });
  const summaryItem = (labelKey, value) =>
    el("div", { class: "min-w-0 border-b border-slate-100 py-2 dark:border-slate-800/70" }, [
      el("p", { class: "text-xs text-slate-400 dark:text-slate-500", "data-i18n": labelKey }),
      el("p", { class: "mt-1 truncate text-sm font-medium text-slate-700 dark:text-slate-200", title: value, text: value })
    ]);
  const languageValue = state.settings.language === "zh-CN"
    ? controller.t("settings.language.zh")
    : controller.t("settings.language.en");
  const themeValue = controller.t(`header.theme.${state.settings.themeMode}`);

  modal.open({
    title: controller.t("warning.title"),
    description: controller.t("settings.description"),
    closeLabel: controller.t("rules.close"),
    sizeClass: "max-w-xl",
    translate: (key) => controller.t(key),
    body: [
      state.capabilities.recycleBin ? null : warningBox,
      sectionTitle("trash", "settings.section.general"),
      el("div", { class: "flex items-center justify-between gap-4 border-y border-slate-200/70 py-3 dark:border-slate-700/60" }, [
        el("div", { class: "flex min-w-0 items-start gap-3" }, [
          el("span", { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-teal-50 text-teal-700 dark:bg-teal-500/15 dark:text-teal-200" }, [icon("trash", 18)]),
          el("div", { class: "min-w-0" }, [
            el("p", { class: "text-sm font-medium text-slate-700 dark:text-slate-200", "data-i18n": "actions.recycle" }),
            el("p", { class: "mt-0.5 text-xs text-slate-400 dark:text-slate-500", "data-i18n": "settings.recycleHint" })
          ])
        ]),
        recycleSwitch
      ]),
      sectionTitle("settings", "settings.summary"),
      el("div", { class: "grid gap-x-5 sm:grid-cols-2" }, [
        summaryItem("header.language", languageValue),
        summaryItem("header.theme", themeValue),
        summaryItem("scan.recent", String(state.settings.recentDirectories.length)),
        summaryItem("settings.ignoredPaths", String(state.settings.ignoredPaths.length)),
        summaryItem("actions.rules", String(state.settings.ignorePatterns.length))
      ]),
      sectionTitle("rules", "settings.section.config"),
      el("div", { class: "flex flex-col gap-2 border-y border-slate-200/70 py-3 dark:border-slate-700/60" }, [
        el("div", { class: "flex flex-wrap items-center justify-between gap-2" }, [
          el("p", { class: "text-sm font-medium text-slate-700 dark:text-slate-200", "data-i18n": "settings.configPath" }),
          configStatus
        ]),
        configPathText
      ]),
      el("div", { class: "flex items-start justify-between gap-4 pt-1" }, [
        el("div", { class: "flex min-w-0 items-start gap-3" }, [
          el("span", { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-rose-50 text-rose-700 dark:bg-rose-500/15 dark:text-rose-200" }, [icon("warning", 18)]),
          el("div", { class: "min-w-0" }, [
            el("p", { class: "text-sm font-medium text-slate-700 dark:text-slate-200", "data-i18n": "settings.reset" }),
            el("p", { class: "mt-0.5 text-xs text-slate-400 dark:text-slate-500", "data-i18n": "settings.resetHint" })
          ])
        ]),
        button({
          label: controller.t("settings.reset"),
          iconName: "trash",
          variant: "danger",
          onClick: async () => {
            if (await controller.onResetSettings()) {
              modal.close();
            }
          }
        })
      ])
    ],
    footer: [
      button({
        label: controller.t("rules.close"),
        onClick: () => modal.close()
      }),
      button({
        label: controller.t("settings.save"),
        variant: "primary",
        onClick: async () => {
          if (await controller.onSaveSettings()) {
            modal.close();
          }
        }
      })
    ]
  });
}

export function openRowMenu(modal, controller, path, position) {
  const menu = el("div", {
    class: "context-menu",
    role: "menu",
    style: `left:${position.x}px;top:${position.y}px`
  });
  const entries = [
    { label: controller.t("row.openFolder"), iconName: "external", run: () => controller.onOpenFolder(path) },
    { label: controller.t("row.copyPath"), iconName: "copy", run: () => controller.onCopyPath(path) },
    { label: controller.t("row.ignore"), iconName: "eyeOff", run: () => controller.onIgnoreFolder(path) }
  ];
  for (const entry of entries) {
    menu.append(
      button({
        label: entry.label,
        iconName: entry.iconName,
        className: "w-full justify-center px-3",
        onClick: () => {
          modal.close();
          entry.run();
        }
      })
    );
  }
  modal.open({
    title: path,
    sizeClass: "max-w-md",
    dismissable: true,
    body: [],
    footer: []
  });
  const card = modal.active.card;
  clear(card);
  card.style.padding = "0.5rem";
  card.append(menu);
  clampToViewport(menu);
}

function clampToViewport(menu) {
  const bounds = menu.getBoundingClientRect();
  const overflowX = bounds.right - window.innerWidth + 8;
  const overflowY = bounds.bottom - window.innerHeight + 8;
  if (overflowX > 0) {
    menu.style.left = `${Math.max(8, bounds.left - overflowX)}px`;
  }
  if (overflowY > 0) {
    menu.style.top = `${Math.max(8, bounds.top - overflowY)}px`;
  }
}


export function openAboutDialog(modal, controller, state) {
  const row = (labelKey, value) =>
    el("div", { class: "flex items-start justify-between gap-4 border-b border-slate-100 py-2 dark:border-slate-800/70" }, [
      el("span", { class: "shrink-0 text-xs text-slate-400 dark:text-slate-500", "data-i18n": labelKey }),
      el("span", { class: "min-w-0 break-all text-right font-mono text-xs text-slate-700 dark:text-slate-200", text: value })
    ]);

  modal.open({
    title: controller.t("about.title"),
    description: controller.t("about.description"),
    closeLabel: controller.t("rules.close"),
    sizeClass: "max-w-lg",
    translate: (key) => controller.t(key),
    body: [
      row("about.version", state.version || "—"),
      row("about.runtime", state.runtimeVersion || "—"),
      row("about.installPath", state.installPath || "—"),
      row("about.settingsPath", state.settingsPath || "—")
    ]
  });
}
