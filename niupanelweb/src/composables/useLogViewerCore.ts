import {
  computed,
  ref,
  toValue,
  watch,
  type MaybeRefOrGetter,
} from "vue";
import { AnsiUp } from "ansi_up";
import type {
  LogFetcher,
  LogUiEvent,
  LogViewerWriteInput,
} from "@/types/logViewer";
import {
  parseLogUiDirective,
  stringifyLogWriteInput,
} from "@/utils/logViewer";

export type LogViewerChunk = {
  id: string;
  content: string;
};

type LogViewerLoadErrorMessages = {
  empty: string;
  failure: string;
};

type UseLogViewerCoreOptions = {
  disableUi: MaybeRefOrGetter<boolean>;
  getScrollElement: () => HTMLElement | null;
  maxChunks: MaybeRefOrGetter<number>;
  onUiEvent: (event: LogUiEvent) => void;
  scrollToBottom: () => void;
  loadErrorMessages?: LogViewerLoadErrorMessages;
  onWrapChange?: () => void;
  preserveScrollOnPrepend?: (insert: () => void) => void;
  searchText?: MaybeRefOrGetter<string>;
};

const CHUNK_SIZE = 128 * 1024;

const escapeRegExp = (value: string) =>
  value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

const normalizeLogLine = (line: string) => line.replace(/\r$/, "").trimEnd();

const getHttpStatus = (error: unknown) => {
  if (typeof error !== "object" || error === null || !("response" in error)) {
    return null;
  }

  const response = (error as { response?: unknown }).response;
  if (
    typeof response !== "object" ||
    response === null ||
    !("status" in response)
  ) {
    return null;
  }

  const status = (response as { status?: unknown }).status;
  return typeof status === "number" ? status : null;
};

export function useLogViewerCore(options: UseLogViewerCoreOptions) {
  const chunks = ref<LogViewerChunk[]>([]);
  const lineBuffer = ref("");
  const ansiUp = new AnsiUp();
  const isWrap = ref(true);
  const localSearchText = ref("");
  const onlyShowMatches = ref(false);

  const isAtBottom = ref(true);
  const hasNewLogs = ref(false);
  const isPullMode = ref(false);
  const fetcher = ref<LogFetcher | null>(null);
  const loadingTop = ref(false);
  const currentStart = ref(0);
  const loadError = ref<string | null>(null);

  let idCounter = 0;
  const getNextId = () => `log-${Date.now()}-${idCounter++}`;

  const searchText = computed(() =>
    options.searchText === undefined
      ? localSearchText.value
      : toValue(options.searchText),
  );

  const filteredChunks = computed(() => {
    if (!searchText.value || !onlyShowMatches.value) return chunks.value;

    const query = searchText.value.toLowerCase();
    return chunks.value.filter((chunk) =>
      chunk.content.toLowerCase().includes(query),
    );
  });

  const processLine = (line: string): string | null => {
    if (toValue(options.disableUi)) return null;

    const directive = parseLogUiDirective(line);
    if (!directive) return null;

    if (directive.event) options.onUiEvent(directive.event);
    return directive.displayLine;
  };

  const createChunk = (line: string): LogViewerChunk => {
    const clean = normalizeLogLine(line);
    const hint = processLine(clean);
    return { id: getNextId(), content: hint || clean };
  };

  const createChunksFromLines = (lines: string[]) => lines.map(createChunk);

  const createChunksFromContent = (content: string) =>
    createChunksFromLines(content.split("\n"));

  const isHighlighted = (content: string) => {
    if (!searchText.value) return false;
    return content.toLowerCase().includes(searchText.value.toLowerCase());
  };

  const renderLine = (text: string) => {
    if (!text) return "";

    let html = ansiUp.ansi_to_html(text);
    if (
      searchText.value &&
      html.toLowerCase().includes(searchText.value.toLowerCase())
    ) {
      const regex = new RegExp(`(${escapeRegExp(searchText.value)})`, "gi");
      html = html.replace(regex, '<mark class="log-mark">$1</mark>');
    }

    return html;
  };

  const clear = () => {
    chunks.value = [];
    lineBuffer.value = "";
  };

  const reset = () => {
    clear();
  };

  const toggleWrap = () => {
    isWrap.value = !isWrap.value;
    options.onWrapChange?.();
  };

  const setSearch = (text: string, filter = false) => {
    localSearchText.value = text;
    onlyShowMatches.value = filter;
  };

  const trimChunks = () => {
    const maxChunks = toValue(options.maxChunks);
    if (chunks.value.length <= maxChunks) return;
    chunks.value = chunks.value.slice(chunks.value.length - maxChunks);
  };

  const write = (data: LogViewerWriteInput) => {
    const rawData = stringifyLogWriteInput(data);
    if (rawData === null) return;

    lineBuffer.value += rawData;
    if (!lineBuffer.value.includes("\n")) return;

    const lines = lineBuffer.value.split("\n");
    lineBuffer.value = lines.pop() || "";

    const el = options.getScrollElement();
    const isAuto =
      !el || el.scrollHeight - el.scrollTop - el.clientHeight <= 100;

    chunks.value = [...chunks.value, ...createChunksFromLines(lines)];
    trimChunks();

    if (isAuto) options.scrollToBottom();
    else hasNewLogs.value = true;
  };

  const writeln = (data: string) => {
    write(data + "\n");
  };

  const init = async (fetcherFn: LogFetcher) => {
    clear();
    currentStart.value = 0;
    hasNewLogs.value = false;
    isAtBottom.value = true;
    isPullMode.value = true;
    fetcher.value = fetcherFn;
    loadingTop.value = true;
    loadError.value = null;

    try {
      const meta = await fetcher.value(0, 0);
      const totalSize = meta.total_size ?? 0;

      if (totalSize > 0) {
        const start = Math.max(0, totalSize - CHUNK_SIZE);
        const data = await fetcher.value(start, totalSize - start);
        if (data.content) {
          chunks.value = createChunksFromContent(data.content);
          currentStart.value = start;
        }
      } else if (options.loadErrorMessages) {
        loadError.value = options.loadErrorMessages.empty;
      }

      options.scrollToBottom();
    } catch (error: unknown) {
      if (!options.loadErrorMessages) throw error;

      loadError.value =
        getHttpStatus(error) === 404
          ? options.loadErrorMessages.empty
          : options.loadErrorMessages.failure;
    } finally {
      loadingTop.value = false;
    }
  };

  const handleScroll = () => {
    const el = options.getScrollElement();
    if (!el) return;

    const { scrollTop, scrollHeight, clientHeight } = el;
    const atBottom = scrollHeight - scrollTop - clientHeight < 100;
    isAtBottom.value = atBottom;
    if (atBottom) hasNewLogs.value = false;
  };

  const loadOlderLogs = async () => {
    if (!fetcher.value || currentStart.value <= 0 || loadingTop.value) return;

    loadingTop.value = true;
    const start = Math.max(0, currentStart.value - CHUNK_SIZE);

    try {
      const data = await fetcher.value(start, currentStart.value - start);
      const added = data.content ? createChunksFromContent(data.content) : [];

      if (added.length > 0) {
        const insert = () => {
          chunks.value.unshift(...added);
        };
        if (options.preserveScrollOnPrepend) {
          options.preserveScrollOnPrepend(insert);
        } else {
          insert();
        }
      }

      currentStart.value = start;
    } finally {
      loadingTop.value = false;
    }
  };

  watch(
    () => chunks.value.length,
    (newLen) => {
      if (newLen > 0 && isAtBottom.value) {
        options.scrollToBottom();
      }
    },
  );

  return {
    chunks,
    currentStart,
    filteredChunks,
    handleScroll,
    hasNewLogs,
    init,
    isAtBottom,
    isHighlighted,
    isPullMode,
    isWrap,
    loadError,
    loadOlderLogs,
    loadingTop,
    renderLine,
    reset,
    clear,
    searchText,
    setSearch,
    toggleWrap,
    write,
    writeln,
  };
}
