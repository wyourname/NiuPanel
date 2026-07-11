import type { TaskRunHistoryItem } from "@/types";

export type TaskDetailTab = "log" | "script" | "var" | "info";
export type TaskFooterAction = "run" | "stop" | "pause" | "resume";

export type {
  LogFetcher as TaskLogFetcher,
  LogFetchResult as TaskLogFetchResult,
  LogUiEvent as TaskLogUiEvent,
  LogViewerRef as TaskLogViewerRef,
  LogViewerWriteInput as TaskLogViewerWriteInput,
} from "@/types/logViewer";

export type TaskScriptEditorRef = {
  trigger?: (source: string, handlerId: string) => void;
  updateOptions?: (options: { wordWrap?: "on" | "off" }) => void;
};

export type TaskEditorOptions = Record<string, unknown>;

export type TaskFocusableRef = {
  focus?: () => void;
};

export type TaskRunTimelineItem = TaskRunHistoryItem;

const createCommandGuard = <T extends string>(commands: readonly T[]) => {
  const commandSet = new Set<string>(commands);

  return (command: unknown): command is T =>
    typeof command === "string" && commandSet.has(command);
};

export const taskBulkCommands = [
  "resume",
  "stop",
  "pin",
  "unpin",
  "enable",
  "disable",
  "share",
] as const;

export type TaskBulkCommand = (typeof taskBulkCommands)[number];
export const isTaskBulkCommand = createCommandGuard(taskBulkCommands);

export const taskBulkMoreCommands = [
  "resume",
  "unpin",
  "delete",
] as const;

export type TaskBulkMoreCommand = (typeof taskBulkMoreCommands)[number];
export const isTaskBulkMoreCommand = createCommandGuard(taskBulkMoreCommands);

export const taskContextCommands = [
  "run",
  "stop",
  "edit",
  "script",
  "vars",
  "cron",
  "share",
  "select",
  "pin",
  "delete",
] as const;

export type TaskContextCommand = (typeof taskContextCommands)[number];
export const isTaskContextCommand = createCommandGuard(taskContextCommands);

export const taskDetailMoreCommands = [
  "edit_config",
  "edit_script",
  "share",
  "download_log",
  "clear_screen",
  "delete_task",
] as const;

export type TaskDetailMoreCommand = (typeof taskDetailMoreCommands)[number];
export const isTaskDetailMoreCommand = createCommandGuard(
  taskDetailMoreCommands,
);

export type TaskMobileActionCommand =
  | "logs"
  | "edit"
  | "script"
  | "cron"
  | "variables"
  | "share"
  | "pin"
  | "unpin"
  | "copy"
  | "delete";

export type TaskMobileScriptEditorCommand = "undo" | "redo" | "format";
