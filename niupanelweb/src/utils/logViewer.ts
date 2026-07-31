import type { LogUiEvent, LogViewerWriteInput } from "@/types/logViewer";

const ANSI_ESCAPE_PATTERN = /\x1B\[[0-?]*[ -/]*[@-~]/g;

export type LogUiDirectiveResult = {
  displayLine: string;
  event?: LogUiEvent;
};

export const stripAnsi = (content: string) =>
  content.replace(ANSI_ESCAPE_PATTERN, "");

export const isLogSystemMessage = (content: string) => {
  const clean = stripAnsi(content);
  return clean.includes("[UI]") || clean.includes("[System]");
};

export const stripLogUiPrefix = (content: string) =>
  stripAnsi(content)
    .replace(/\[UI\]|\[System\]/g, "")
    .trim();

export const stringifyLogWriteInput = (
  data: LogViewerWriteInput,
): string | null => {
  if (data === undefined || data === null) return null;
  if (typeof data === "string") return data;

  try {
    const serialized = JSON.stringify(data);
    if (typeof serialized === "string") return serialized;
  } catch {
    // Fall back to String() for values like bigint or circular structures.
  }

  return String(data);
};

export const parseLogUiDirective = (
  line: string,
): LogUiDirectiveResult | null => {
  const clean = stripAnsi(line).trim();

  if (clean.includes("[UI:QRCODE]")) {
    const data = clean.substring(clean.indexOf("[UI:QRCODE]") + 11).trim();
    return {
      displayLine: "[UI] 二维码显示中...",
      event: data ? { type: "qrcode", data } : undefined,
    };
  }

  if (clean.includes("[UI:CLOSE_QRCODE]")) {
    return {
      displayLine: "[UI] 二维码已关闭",
      event: { type: "close_qrcode" },
    };
  }

  if (clean.includes("[UI:PROGRESS]")) {
    const valStr = clean.substring(clean.indexOf("[UI:PROGRESS]") + 13).trim();
    const rawValue = parseInt(valStr, 10);

    return {
      displayLine: `[UI] 进度已更新 (${rawValue}%)`,
      event: Number.isNaN(rawValue)
        ? undefined
        : { type: "progress", data: Math.min(100, Math.max(0, rawValue)) },
    };
  }

  if (clean.includes("[UI:CLOSE_PROGRESS]")) {
    return {
      displayLine: "[UI] 进度条已隐藏",
      event: { type: "close_progress" },
    };
  }

  return null;
};
