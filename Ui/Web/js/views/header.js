import { button, el, icon } from "../dom.js";

const LANGUAGES = [
  { value: "system", key: "header.language.system" },
  { value: "zh-CN", label: "简体中文" },
  { value: "en-US", label: "English" }
];

const THEMES = [
  { value: "system", key: "header.theme.system" },
  { value: "light", key: "header.theme.light" },
  { value: "dark", key: "header.theme.dark" }
];

export function createHeader(controller) {
  const title = el("h1", { class: "truncate text-[15px] font-semibold leading-tight text-slate-700 dark:text-slate-100", text: controller.t("app.title") });
  const subtitle = el("p", {
    class: "mt-0.5 truncate text-xs text-slate-500 dark:text-slate-400",
    text: controller.t("app.subtitle")
  });

  const languageSelect = el("select", {
    class: "field w-auto min-w-[7.5rem] py-2 pr-8",
    "aria-label": controller.t("header.language"),
    onchange: (event) => controller.onLanguageChange(event.target.value)
  });
  for (const language of LANGUAGES) {
    const attributes = { value: language.value };
    if (language.key) {
      attributes["data-i18n"] = language.key;
    } else {
      attributes.text = language.label;
    }
    languageSelect.append(el("option", attributes));
  }

  const themeSelect = el("select", {
    class: "field w-auto min-w-[7.5rem] py-2 pr-8",
    "aria-label": controller.t("header.theme"),
    onchange: (event) => controller.onThemeChange(event.target.value)
  });
  for (const theme of THEMES) {
    themeSelect.append(el("option", { value: theme.value, "data-i18n": theme.key }));
  }

  const settingsButton = button({
    iconName: "settings",
    className: "btn-icon",
    title: controller.t("header.settings"),
    ariaLabel: controller.t("header.settings"),
    onClick: () => controller.openSettings()
  });

  const aboutButton = button({
    iconName: "info",
    className: "btn-icon",
    title: controller.t("header.about"),
    ariaLabel: controller.t("header.about"),
    onClick: () => controller.openAbout()
  });

  const node = el("header", { class: "surface-enter glass-header z-20 flex flex-wrap items-center justify-between gap-4 px-4 py-3 sm:px-6" }, [
    el("div", { class: "flex min-w-0 items-center gap-3" }, [
      el("span", {
        class: "grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-teal-600 text-white shadow-sm"
      }, [icon("folder", 19)]),
      el("div", { class: "min-w-0" }, [title, subtitle])
    ]),
    el("div", { class: "flex flex-wrap items-center justify-end gap-2 sm:gap-3" }, [
      el("label", { class: "flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400" }, [
        icon("globe", 16),
        languageSelect
      ]),
      el("label", { class: "flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400" }, [
        icon("moon", 16),
        themeSelect
      ]),
      aboutButton,
      settingsButton
    ])
  ]);

  return {
    node,
    update(state) {
      title.textContent = controller.t("app.title");
      subtitle.textContent = controller.t("app.subtitle");
      languageSelect.value = state.settings.language;
      themeSelect.value = state.settings.themeMode;
      settingsButton.title = controller.t("header.settings");
      settingsButton.setAttribute("aria-label", controller.t("header.settings"));
      aboutButton.title = controller.t("header.about");
      aboutButton.setAttribute("aria-label", controller.t("header.about"));
      for (const option of themeSelect.options) {
        const key = option.getAttribute("data-i18n");
        if (key) {
          option.textContent = controller.t(key);
        }
      }
      for (const label of node.querySelectorAll("select")) {
        const key = label === languageSelect ? "header.language" : "header.theme";
        label.setAttribute("aria-label", controller.t(key));
      }
    }
  };
}
