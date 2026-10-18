const FALLBACK_LANGUAGE = "en-US";
const LOAD_TIMEOUT_MS = 8000;
const SUPPORTED = ["zh-CN", "en-US"];

function detectSystemLanguage() {
  const tag = String(navigator.language || "").toLowerCase();
  return tag.startsWith("zh") ? "zh-CN" : "en-US";
}

export function resolveLanguage(setting) {
  if (SUPPORTED.includes(setting)) {
    return setting;
  }
  return detectSystemLanguage();
}

export class I18n {
  constructor() {
    this.language = FALLBACK_LANGUAGE;
    this.dictionaries = new Map();
    this.listeners = new Set();
  }

  onChange(listener) {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  async setLanguage(language) {
    const resolved = resolveLanguage(language);
    await this.load(resolved);
    this.language = resolved;
    document.documentElement.lang = resolved;
    for (const listener of this.listeners) {
      listener(resolved);
    }
  }

  async load(language) {
    if (this.dictionaries.has(language)) {
      return;
    }

    let lastError = null;
    for (let attempt = 0; attempt < 2; attempt += 1) {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), LOAD_TIMEOUT_MS);
      try {
        const response = await fetch(`i18n/${language}.json`, {
          signal: controller.signal,
          cache: "no-store"
        });
        if (!response.ok) {
          throw new Error(`i18n_http_${response.status}`);
        }
        const data = await response.json();
        if (!data || typeof data !== "object" || Array.isArray(data)) {
          throw new Error("i18n_invalid_json");
        }
        this.dictionaries.set(language, data);
        return;
      } catch (error) {
        lastError = error;
      } finally {
        clearTimeout(timer);
      }

      if (attempt === 0) {
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
    }

    throw lastError ?? new Error("i18n_load_failed");
  }

  t(key, params) {
    const primary = this.dictionaries.get(this.language);
    const fallback = this.dictionaries.get(FALLBACK_LANGUAGE);
    const template = primary?.[key] ?? fallback?.[key] ?? key;
    return this.interpolate(template, params);
  }

  interpolate(template, params) {
    if (typeof template !== "string" || !params) {
      return String(template);
    }
    return template.replace(/\{(\w+)\}/g, (match, name) =>
      Object.hasOwn(params, name) ? String(params[name]) : match
    );
  }
}
