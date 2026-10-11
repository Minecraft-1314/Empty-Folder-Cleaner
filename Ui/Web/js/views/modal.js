import { button, clear, el } from "../dom.js";

export class ModalHost {
  constructor(root) {
    this.root = root;
    this.active = null;
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && this.active?.dismissable !== false) {
        this.close();
      }
    });
  }

  get isOpen() {
    return this.active !== null;
  }

  open(options) {
    this.close();
    const previousFocus = document.activeElement;
    const backdrop = el("div", { class: "modal-backdrop", role: "presentation" });
    const card = el("div", {
      class: `modal-card ${options.sizeClass ?? "max-w-lg"}`,
      role: "dialog",
      "aria-modal": "true",
      "aria-label": options.title,
      dataset: { role: "modal-card" }
    });
    backdrop.append(card);
    backdrop.addEventListener("mousedown", (event) => {
      if (event.target === backdrop && options.dismissable !== false) {
        this.close();
      }
    });
    clear(this.root);
    this.root.append(backdrop);
    this.active = { backdrop, card, previousFocus, dismissable: options.dismissable };
    this.build(options);
    const focusTarget = card.querySelector("[data-autofocus]") ?? card.querySelector("button, input, select, textarea");
    focusTarget?.focus();
    return this;
  }

  build(options) {
    const { card } = this.active;
    card.append(
      el("div", { class: "modal-header" }, [
        el("div", {}, [
          el("h2", { text: options.title }),
          options.description ? el("p", { text: options.description }) : null
        ]),
        button({
          label: "",
          iconName: "close",
          className: "btn-ghost btn-icon modal-close",
          title: options.closeLabel ?? "Close",
          ariaLabel: options.closeLabel ?? "Close",
          onClick: () => this.close()
        })
      ]),
      el("div", { class: "modal-body" }, options.body ?? [])
    );
    if (options.footer?.length) {
      card.append(
        el("div", { class: "modal-footer" }, options.footer)
      );
    }
    if (typeof options.translate === "function") {
      for (const node of card.querySelectorAll("[data-i18n]")) {
        const key = node.getAttribute("data-i18n");
        const text = options.translate(key);
        if (text) {
          node.textContent = text;
        }
      }
    }
  }

  close() {
    const active = this.active;
    if (!active) {
      return;
    }
    this.active = null;
    active.backdrop.remove();
    clear(this.root);
    if (active.previousFocus instanceof HTMLElement && document.contains(active.previousFocus)) {
      active.previousFocus.focus();
    }
  }
}
