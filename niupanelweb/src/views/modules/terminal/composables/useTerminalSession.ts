import { onMounted, onUnmounted, ref, type Ref } from "vue";
import { ElMessage } from "element-plus";
import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import { WebLinksAddon } from "xterm-addon-web-links";
import { WebglAddon } from "xterm-addon-webgl";
import { useAppStore } from "@/stores/app";

type TerminalErrorMessage = {
  error: string;
};

const TERMINAL_THEME = {
  background: "#0b0e14",
  foreground: "#e2e8f0",
  cursor: "#3b82f6",
  selectionBackground: "rgba(59, 130, 246, 0.3)",
  black: "#1f2430",
  red: "#ff6c6b",
  green: "#c3e88d",
  yellow: "#ffcb6b",
  blue: "#82aaff",
  magenta: "#c792ea",
  cyan: "#89ddff",
  white: "#eeffff",
  brightBlack: "#65737e",
  brightRed: "#ff5370",
  brightGreen: "#c3e88d",
  brightYellow: "#ffcb6b",
  brightBlue: "#82aaff",
  brightMagenta: "#c792ea",
  brightCyan: "#89ddff",
  brightWhite: "#ffffff",
};

const isTerminalErrorMessage = (value: unknown): value is TerminalErrorMessage => {
  return (
    typeof value === "object" &&
    value !== null &&
    "error" in value &&
    typeof (value as { error?: unknown }).error === "string"
  );
};

const buildTerminalWebSocketUrl = (serverUrl: string | null | undefined) => {
  if (serverUrl && serverUrl.startsWith("http")) {
    const protocol = serverUrl.startsWith("https") ? "wss:" : "ws:";
    const host = serverUrl.replace(/^https?:\/\//, "");
    return `${protocol}//${host}/api/v1/terminal/ws`;
  }

  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.host}/api/v1/terminal/ws`;
};

export function useTerminalSession(terminalRef: Ref<HTMLElement | null>) {
  const appStore = useAppStore();
  const connected = ref(false);

  let term: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let webglAddon: WebglAddon | null = null;
  let ws: WebSocket | null = null;
  let terminalElement: HTMLElement | null = null;

  const sendInput = (data: string) => {
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: "input", data }));
    }
  };

  const sendResize = (rows: number, cols: number) => {
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: "resize", rows, cols }));
    }
  };

  const handleContainerClick = () => {
    term?.focus();
  };

  const handleContextMenu = (event: MouseEvent) => {
    event.preventDefault();
    navigator.clipboard
      .readText()
      .then((text) => {
        if (!text) return;
        sendInput(text);
        ElMessage.success({
          message: "已从剪贴板粘贴",
          duration: 1500,
          grouping: true,
        });
      })
      .catch(() => {
        ElMessage.warning({
          message: "无法读取剪贴板",
          duration: 2000,
          grouping: true,
        });
      });
  };

  const disposeWebSocket = (silent = false) => {
    if (!ws) return;
    if (silent) {
      ws.onclose = null;
      ws.onerror = null;
    }
    ws.close();
    ws = null;
  };

  const connectWebSocket = () => {
    ws = new WebSocket(buildTerminalWebSocketUrl(appStore.serverUrl));
    ws.binaryType = "arraybuffer";

    ws.onopen = () => {
      connected.value = true;
      term?.write("\x1b[1;32m[System] Terminal connected.\x1b[0m\r\n");
      if (term) {
        sendResize(term.rows, term.cols);
      }
    };

    ws.onmessage = (event) => {
      if (event.data instanceof ArrayBuffer) {
        term?.write(new Uint8Array(event.data));
        return;
      }

      try {
        const message: unknown = JSON.parse(String(event.data));
        if (isTerminalErrorMessage(message)) {
          term?.write(`\x1b[1;31m[Error] ${message.error}\x1b[0m\r\n`);
        }
      } catch {
      }
    };

    ws.onclose = () => {
      connected.value = false;
      term?.write("\r\n\x1b[1;31m[System] Terminal disconnected.\x1b[0m\r\n");
    };

    ws.onerror = () => {
      term?.write("\r\n\x1b[1;31m[Error] Connection error.\x1b[0m\r\n");
    };
  };

  const initTerminal = () => {
    term = new Terminal({
      cursorBlink: true,
      cursorStyle: "bar",
      fontSize: 14,
      lineHeight: 1.2,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Menlo', 'Consolas', monospace",
      theme: TERMINAL_THEME,
      allowProposedApi: true,
    });

    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(new WebLinksAddon());

    terminalElement = terminalRef.value;
    if (terminalElement) {
      term.open(terminalElement);
      try {
        webglAddon = new WebglAddon();
        term.loadAddon(webglAddon);
      } catch {
        webglAddon = null;
      }
      fitAddon.fit();
      terminalElement.addEventListener("click", handleContainerClick);
      terminalElement.addEventListener("contextmenu", handleContextMenu);
    }

    term.attachCustomKeyEventHandler((event) => {
      if (event.ctrlKey && event.code === "KeyC" && term?.hasSelection()) {
        void navigator.clipboard.writeText(term.getSelection()).catch(() => {});
        return false;
      }

      if (event.ctrlKey && event.code === "KeyV") {
        void navigator.clipboard.readText().then(sendInput).catch(() => {});
        return false;
      }

      if (event.key === "Tab") {
        event.preventDefault();
      }
      return true;
    });

    term.onSelectionChange(() => {
      if (!term?.hasSelection()) return;
      const selectedText = term.getSelection();
      if (!selectedText) return;

      void navigator.clipboard.writeText(selectedText).then(() => {
        ElMessage.success({
          message: "已复制到剪贴板",
          duration: 1500,
          grouping: true,
        });
      }).catch(() => {});
    });

    term.onData(sendInput);
    term.onResize((size) => {
      sendResize(size.rows, size.cols);
    });

    connectWebSocket();
  };

  const reconnect = () => {
    disposeWebSocket(true);
    connected.value = false;
    term?.reset();
    connectWebSocket();
  };

  const handleResize = () => {
    if (document.hidden) return;
    fitAddon?.fit();
  };

  const cleanupTerminal = () => {
    window.removeEventListener("resize", handleResize);
    terminalElement?.removeEventListener("click", handleContainerClick);
    terminalElement?.removeEventListener("contextmenu", handleContextMenu);
    disposeWebSocket(true);
    try {
      webglAddon?.dispose();
    } catch {
    }
    webglAddon = null;
    try {
      term?.dispose();
    } catch {
    }
    term = null;
    fitAddon = null;
    terminalElement = null;
  };

  onMounted(() => {
    initTerminal();
    window.addEventListener("resize", handleResize);
  });

  onUnmounted(cleanupTerminal);

  return {
    connected,
    reconnect,
  };
}
