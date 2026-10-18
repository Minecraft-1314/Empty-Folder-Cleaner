export const EASE_STANDARD = "cubic-bezier(0.4, 0, 0.2, 1)";

export function el(tag, attributes = {}, children = []) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attributes)) {
    if (value === null || value === undefined || value === false) {
      continue;
    }
    if (key === "class") {
      node.className = value;
    } else if (key === "text") {
      node.textContent = String(value);
    } else if (key === "html") {
      node.innerHTML = value;
    } else if (key === "dataset") {
      for (const [dataKey, dataValue] of Object.entries(value)) {
        node.dataset[dataKey] = String(dataValue);
      }
    } else if (key.startsWith("on") && typeof value === "function") {
      node.addEventListener(key.slice(2).toLowerCase(), value);
    } else if (value === true) {
      node.setAttribute(key, "");
    } else {
      node.setAttribute(key, String(value));
    }
  }

  for (const child of Array.isArray(children) ? children : [children]) {
    if (child === null || child === undefined || child === false) {
      continue;
    }
    node.append(typeof child === "string" ? document.createTextNode(child) : child);
  }

  return node;
}

export function clear(node) {
  while (node.firstChild) {
    node.removeChild(node.firstChild);
  }
}

export function replace(node, children) {
  clear(node);
  for (const child of children) {
    if (child) {
      node.append(child);
    }
  }
}

export function icon(name, size = 18) {
  const node = iconNode(name, size);
  node.setAttribute("data-icon", name);
  return node;
}

function iconNode(name, size) {
  const paths = {
    folder: "M3 7.5A1.5 1.5 0 0 1 4.5 6h4l1.8 2H19.5A1.5 1.5 0 0 1 21 9.5v8A1.5 1.5 0 0 1 19.5 19h-15A1.5 1.5 0 0 1 3 17.5v-10Z",
    search: "M10.5 3a7.5 7.5 0 1 1-4.9 13.2l-3.3 3.3a1 1 0 0 1-1.4-1.4l3.3-3.3A7.5 7.5 0 0 1 10.5 3Zm0 2a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11Z",
    refresh: "M12 5V2L8 6l4 4V7a5 5 0 1 1-5 5H5a7 7 0 1 0 7-7Z",
    stop: "M7 7h10v10H7z",
    trash: "M9 3h6l1 2h4v2H4V5h4l1-2Zm-3 6h12l-1 12H7L6 9Z",
    check: "M9.6 16.2 5.4 12l-1.4 1.4 5.6 5.6L20.4 8.2 19 6.8 9.6 16.2Z",
    close: "M6.4 5 12 10.6 17.6 5 19 6.4 13.4 12 19 17.6 17.6 19 12 13.4 6.4 19 5 17.6 10.6 12 5 6.4 6.4 5Z",
    plus: "M11 5h2v6h6v2h-6v6h-2v-6H5v-2h6V5Z",
    external: "M14 4h6v6h-2V6.4l-7.3 7.3-1.4-1.4L17.6 5H14V4ZM6 6h4v2H7v9h9v-3h2v5H6V6Z",
    copy: "M8 3h9a1 1 0 0 1 1 1v3h-2V5H8v11h11v-3h2v3a1 1 0 0 1-1 1H8a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1Z",
    eyeOff: "M3.5 3.5 20.5 20.5 19.1 21.9l-2.3-2.3A11 11 0 0 1 12 21C6.6 21 2.2 16.9 1 12c.5-1.9 1.6-3.6 3.1-4.9L1.1 4.9 2.5 3.5l1 1ZM12 5c5.4 0 9.8 4.1 11 7-.6 2.3-2.4 4.8-4.9 6.2l-2.2-2.2A5.9 5.9 0 0 0 12 5Zm0 2a5 5 0 0 0-4.6 7L12 18.6 16.6 14A5 5 0 0 0 12 7Z",
    rules: "M4 5h16v2H4V5Zm0 6h10v2H4v-2Zm0 6h16v2H4v-2Z",
    settings:
      "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8Zm0 2a2 2 0 1 1 0 4 2 2 0 0 1 0-4Zm-1-8h2l.5 2.5 1.9.8 2.1-1.4 1.4 1.4-1.4 2.1.8 1.9L21 8v2l-2.7.6-.8 1.9 1.4 2.1-1.4 1.4-2.1-1.4-1.9.8L13 21h-2l-.6-2.6-1.9-.8-2.1 1.4L5 17.6l1.4-2.1-.8-1.9L3 13v-2l2.6-.7.8-1.9L5 6.3 6.4 4.9l2.1 1.4 1.9-.8L11 2Z",
    sun: "M12 7a5 5 0 1 1 0 10 5 5 0 0 1 0-10Zm0-6h0v3h0V1Zm0 19h0v3h0v-3ZM1 11h3v1H1v-1Zm19 0h3v1h-3v-1ZM4.2 4.2 6 6l-1.8 1.8L2.4 6 4.2 4.2Zm12.2 0L18 6l-1.8-1.8L14.4 6 16.4 4.2ZM4.2 19.8 2.4 18l1.8-1.8L6 18l-1.8 1.8Zm12.2 0L14.4 18l1.8 1.8L18 18l-1.6 1.8Z",
    moon: "M12 3a9 9 0 1 0 9 9 7 7 0 0 1-9-9Z",
    globe: "M12 2a10 10 0 1 1 0 20 10 10 0 0 1 0-20Zm7.9 9h-3a15 15 0 0 0-1.3-5.7A8 8 0 0 1 19.9 11ZM12 4.1c.9 1.3 1.6 3.4 1.8 6.9h-3.6c.2-3.5.9-5.6 1.8-6.9ZM4.1 13h3c.2 2.1.6 3.9 1.3 5.7A8 8 0 0 1 4.1 13Zm3-2h-3a8 8 0 0 1 4.3-5.7A15 15 0 0 0 7.1 11ZM12 19.9c-.9-1.3-1.6-3.4-1.8-6.9h3.6c-.2 3.5-.9 5.6-1.8 6.9Zm2.6-1.2c.7-1.8 1.1-3.6 1.3-5.7h3a8 8 0 0 1-4.3 5.7Z",
    warning: "M12 3 1.5 21h21L12 3Zm0 6 1 7h-2l1-7Zm0 9a1.2 1.2 0 1 1 0-2.4A1.2 1.2 0 0 1 12 18Z",
    link: "M10.6 13.4a1 1 0 0 1 0-1.4l2.8-2.8a3 3 0 1 1 4.2 4.2l-1.3 1.3-1.4-1.4 1.3-1.3a1 1 0 0 0-1.4-1.4l-2.8 2.8a1 1 0 0 1-1.4 0Zm2.8-2.8a1 1 0 0 1 0 1.4l-2.8 2.8a3 3 0 1 1-4.2-4.2l1.3-1.3 1.4 1.4-1.3 1.3a1 1 0 0 0 1.4 1.4l2.8-2.8a1 1 0 0 1 1.4 0Z",
    info: "M12 2a10 10 0 1 1 0 20 10 10 0 0 1 0-20Zm0 2a8 8 0 1 0 0 16 8 8 0 0 0 0-16Zm-1 3h2v2h-2V7Zm0 4h2v6h-2v-6Z"
  };
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("width", String(size));
  svg.setAttribute("height", String(size));
  svg.setAttribute("fill", "currentColor");
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("focusable", "false");
  svg.classList.add("shrink-0", "pointer-events-none");
  const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
  path.setAttribute("d", paths[name] ?? paths.folder);
  svg.append(path);
  return svg;
}

export function swapIcon(node, name, size = 18) {
  if (node.getAttribute("data-icon") === name) {
    return;
  }
  node.replaceWith(icon(name, size));
}

export function button(options) {
  const {
    label,
    iconName,
    variant = "ghost",
    onClick,
    className = "",
    title = "",
    ariaLabel = "",
    disabled = false
  } = options;
  const classes = ["btn", `btn-${variant}`, className].filter(Boolean).join(" ");
  const node = el("button", {
    type: "button",
    class: classes,
    disabled,
    title: title || ariaLabel || label,
    "aria-label": ariaLabel || label,
    dataset: { role: "button" }
  });
  if (iconName) {
    node.append(icon(iconName));
  }
  if (label) {
    node.append(el("span", { class: "truncate", text: label }));
  }
  if (onClick) {
    node.addEventListener("click", onClick);
  }
  return node;
}

export function field(options) {
  const {
    id,
    type = "text",
    placeholder = "",
    value = "",
    onInput,
    oninput,
    autocomplete = "off",
    spellcheck = false
  } = options;
  return el("input", {
    id,
    type,
    class: "field",
    placeholder,
    value,
    autocomplete,
    spellcheck: spellcheck ? "true" : "false",
    oninput: oninput ?? onInput
  });
}

export function formatSize(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** exponent;
  return `${exponent === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[exponent]}`;
}

export function formatDate(iso) {
  const value = new Date(iso);
  if (Number.isNaN(value.getTime())) {
    return "";
  }
  const pad = (number) => String(number).padStart(2, "0");
  return `${value.getFullYear()}-${pad(value.getMonth() + 1)}-${pad(value.getDate())} ${pad(value.getHours())}:${pad(value.getMinutes())}`;
}

export function debounce(callback, delay) {
  let timer = null;
  return (...args) => {
    if (timer !== null) {
      clearTimeout(timer);
    }
    timer = setTimeout(() => {
      timer = null;
      callback(...args);
    }, delay);
  };
}
