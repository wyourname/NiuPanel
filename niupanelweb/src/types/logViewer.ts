export type LogFetchResult = {
  content?: string;
  total_size?: number;
};

export type LogFetcher = (
  offset: number,
  limit: number,
) => Promise<LogFetchResult>;

export type LogViewerWriteInput = unknown;

export type LogUiEvent =
  | { type: "qrcode"; data: string }
  | { type: "close_qrcode" }
  | { type: "progress"; data: number }
  | { type: "close_progress" };

export type LogViewerRef = {
  clear?: () => void;
  init?: (loader: LogFetcher) => Promise<void> | void;
  reset?: () => void;
  scrollToBottom?: () => void;
  setSearch?: (query: string, onlyShowMatches?: boolean) => void;
  toggleWrap?: () => void;
  write?: (data: LogViewerWriteInput) => void;
  writeln?: (data: string) => void;
};
