import {
  nextTick,
  onMounted,
  onUnmounted,
  ref,
  toValue,
  watch,
  type MaybeRefOrGetter,
  type Ref,
} from "vue";
import { useResizeObserver, useWindowSize } from "@vueuse/core";
import {
  Terminal,
  type ITerminalInitOnlyOptions,
  type ITerminalOptions,
} from "xterm";
import { FitAddon } from "xterm-addon-fit";
import type { LogFetcher, LogViewerWriteInput } from "@/types/logViewer";
import { stringifyLogWriteInput } from "@/utils/logViewer";

type XtermPadding = {
  top: number;
  bottom: number;
  left: number;
  right: number;
};

type TerminalOptionsWithPadding = ITerminalOptions &
  ITerminalInitOnlyOptions & {
    padding: XtermPadding;
  };

type UseXtermLogViewerOptions = {
  fontSize: MaybeRefOrGetter<number>;
  isMobile: MaybeRefOrGetter<boolean>;
  terminalRef: Ref<HTMLElement | null>;
};

const HISTORY_CHUNK_SIZE = 100 * 1024;

const createTerminalOptions = (
  isMobile: boolean,
  fontSize: number,
): TerminalOptionsWithPadding => ({
  cursorBlink: false,
  disableStdin: true,
  fontSize: isMobile ? 11 : fontSize,
  fontFamily: "'JetBrains Mono', 'Fira Code', 'Consolas', monospace",
  lineHeight: 1.1,
  theme: {
    background: "#1e293b",
    foreground: "#E2E8F0",
    cursor: "#3b82f6",
    selectionBackground: "rgba(59, 130, 246, 0.3)",
    black: "#000000",
    red: "#ef4444",
    green: "#22c55e",
    yellow: "#eab308",
    blue: "#3b82f6",
    magenta: "#d946ef",
    cyan: "#06b6d4",
    white: "#f8fafc",
    brightBlack: "#64748b",
    brightRed: "#fca5a5",
    brightGreen: "#86efac",
    brightYellow: "#fde047",
    brightBlue: "#93c5fd",
    brightMagenta: "#f0abfc",
    brightCyan: "#67e8f9",
    brightWhite: "#ffffff",
  },
  convertEol: true,
  scrollback: 10000,
  padding: isMobile
    ? { top: 12, bottom: 12, left: 12, right: 12 }
    : { top: 20, bottom: 20, left: 24, right: 20 },
});

export function useXtermLogViewer({
  fontSize,
  isMobile,
  terminalRef,
}: UseXtermLogViewerOptions) {
  const isEmpty = ref(true);
  let term: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let pendingLogs: string[] = [];
  let initialFitTimer: ReturnType<typeof setTimeout> | null = null;
  let resizeFitTimer: ReturnType<typeof setTimeout> | null = null;

  const clearTimer = (timer: ReturnType<typeof setTimeout> | null) => {
    if (timer) clearTimeout(timer);
  };

  const safeFit = () => {
    if (
      terminalRef.value &&
      terminalRef.value.offsetParent !== null &&
      fitAddon
    ) {
      try {
        fitAddon.fit();
      } catch {
        // xterm can throw while the host element is mid-layout.
      }
    }
  };

  const scheduleInitialFit = () => {
    clearTimer(initialFitTimer);
    initialFitTimer = setTimeout(() => {
      safeFit();
      initialFitTimer = null;
    }, 50);
  };

  const scheduleResizeFit = () => {
    clearTimer(resizeFitTimer);
    resizeFitTimer = setTimeout(() => {
      safeFit();
      resizeFitTimer = null;
    }, 100);
  };

  const flushPendingLogs = () => {
    if (pendingLogs.length === 0) return;

    pendingLogs.forEach((line) => term?.write(line));
    pendingLogs = [];
    isEmpty.value = false;
  };

  const initTerminal = () => {
    if (!terminalRef.value) return;

    term = new Terminal(
      createTerminalOptions(toValue(isMobile), toValue(fontSize)),
    );
    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(terminalRef.value);

    scheduleInitialFit();
    void nextTick(flushPendingLogs);
  };

  const write = (data: LogViewerWriteInput) => {
    const text = stringifyLogWriteInput(data);
    if (text === null) return;

    if (!term) {
      pendingLogs.push(text);
      return;
    }

    term.write(text);
    isEmpty.value = false;
  };

  const writeln = (data: string) => {
    write(data + "\r\n");
  };

  const clear = () => {
    term?.clear();
    term?.reset();
    isEmpty.value = true;
  };

  const scrollToBottom = () => {
    term?.scrollToBottom();
  };

  const setSearch = (_query: string, _filter = false) => {
    // Search is intentionally unsupported without xterm-addon-search.
  };

  const copyAll = async () => {
    if (!term) return;

    try {
      term.selectAll();
      const text = term.getSelection();
      term.clearSelection();
      await navigator.clipboard.writeText(text);
    } catch (error: unknown) {
      console.error("Copy failed", error);
    }
  };

  const getChunks = () => {
    if (!term) return [];

    const buffer = term.buffer.active;
    const lines: string[] = [];
    for (let index = 0; index < buffer.length; index++) {
      const line = buffer.getLine(index);
      if (line) lines.push(line.translateToString(true));
    }

    return lines.map((content) => ({ content }));
  };

  const init = async (fetcherFn: LogFetcher) => {
    clear();

    try {
      const meta = await fetcherFn(0, 0);

      if (meta.content) {
        write(meta.content);
      } else if ((meta.total_size ?? 0) > 0) {
        const totalSize = meta.total_size ?? 0;
        const start = Math.max(0, totalSize - HISTORY_CHUNK_SIZE);
        const length = totalSize - start;

        if (length > 0) {
          const res = await fetcherFn(start, length);
          if (res.content) write(res.content);
        }
      }

      scrollToBottom();
      scheduleInitialFit();
    } catch {
      writeln(
        "\r\n\x1b[33m[System] Could not load previous log context.\x1b[0m\r\n",
      );
    }
  };

  useResizeObserver(terminalRef, () => {
    requestAnimationFrame(safeFit);
  });

  const { width, height } = useWindowSize();
  watch([width, height], scheduleResizeFit);

  onMounted(initTerminal);

  onUnmounted(() => {
    clearTimer(initialFitTimer);
    clearTimer(resizeFitTimer);
    term?.dispose();
    term = null;
    fitAddon = null;
    pendingLogs = [];
  });

  return {
    chunks: {
      get map() {
        const chunks = getChunks();
        return chunks.map.bind(chunks);
      },
    },
    clear,
    copyAll,
    fit: safeFit,
    init,
    isEmpty,
    reset: clear,
    scrollToBottom,
    setSearch,
    write,
    writeln,
  };
}
