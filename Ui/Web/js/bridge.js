const REQUEST_TIMEOUT_MS = 30000;
const MAX_PENDING = 64;

export class BridgeError extends Error {
  constructor(code, fields = []) {
    super(code);
    this.code = code;
    this.fields = fields;
  }
}

export class HostBridge {
  constructor() {
    this.pending = new Map();
    this.listeners = new Set();
    this.sequence = 0;
    this.isAvailable = typeof window.chrome?.webview?.postMessage === "function";
    if (this.isAvailable) {
      window.chrome.webview.addEventListener("message", (event) => this.handleMessage(event.data));
    }
  }

  onEvent(listener) {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  async invoke(action, payload = {}) {
    if (!this.isAvailable) {
      throw new BridgeError("host_unavailable");
    }
    if (this.pending.size >= MAX_PENDING) {
      throw new BridgeError("too_many_requests");
    }
    const id = `req-${++this.sequence}-${Date.now().toString(36)}`;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new BridgeError("timeout"));
      }, REQUEST_TIMEOUT_MS);
      this.pending.set(id, { resolve, reject, timer });
      try {
        window.chrome.webview.postMessage({ id, action, payload });
      } catch (error) {
        clearTimeout(timer);
        this.pending.delete(id);
        reject(new BridgeError("post_failed"));
      }
    });
  }

  handleMessage(data) {
    if (!data || typeof data !== "object") {
      return;
    }
    if (typeof data.type === "string") {
      for (const listener of this.listeners) {
        try {
          listener(data.type, data.payload ?? null);
        } catch {
          continue;
        }
      }
      return;
    }
    const entry = this.pending.get(data.id);
    if (!entry) {
      return;
    }
    clearTimeout(entry.timer);
    this.pending.delete(data.id);
    if (data.ok) {
      entry.resolve(data.result ?? null);
      return;
    }
    const error = data.error ?? {};
    entry.reject(new BridgeError(error.code ?? "unknown", Array.isArray(error.fields) ? error.fields : []));
  }
}
