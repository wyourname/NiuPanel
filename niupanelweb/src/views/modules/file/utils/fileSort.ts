import type { FileItem } from "@/types";
import { getFileExtension } from "./fileDisplay";

export type FileSortMode =
  | "mtime-asc"
  | "mtime-desc"
  | "name"
  | "size"
  | "type";

export const FILE_SORT_OPTIONS: ReadonlyArray<{
  icon: string;
  label: string;
  value: FileSortMode;
}> = [
  { value: "name", label: "名称", icon: "i-ep-sort" },
  {
    value: "mtime-desc",
    label: "修改时间 · 最新优先",
    icon: "i-ep-bottom",
  },
  {
    value: "mtime-asc",
    label: "修改时间 · 最早优先",
    icon: "i-ep-top",
  },
  { value: "size", label: "大小", icon: "i-ep-files" },
  { value: "type", label: "类型", icon: "i-ep-collection" },
];

export const isFileSortMode = (value: unknown): value is FileSortMode =>
  FILE_SORT_OPTIONS.some((option) => option.value === value);

export const normalizeFileSortMode = (value: string | null): FileSortMode => {
  if (value === "mtime") return "mtime-desc";
  return isFileSortMode(value) ? value : "name";
};

export const getFileSortLabel = (mode: FileSortMode) => {
  if (mode === "mtime-desc") return "最近修改";
  if (mode === "mtime-asc") return "最早修改";
  return FILE_SORT_OPTIONS.find((option) => option.value === mode)?.label ?? "名称";
};

export const toggleModifiedTimeSort = (mode: FileSortMode): FileSortMode =>
  mode === "mtime-desc" ? "mtime-asc" : "mtime-desc";

const compareNames = (a: FileItem, b: FileItem) =>
  a.name.localeCompare(b.name, "zh-CN", {
    numeric: true,
    sensitivity: "base",
  });

const compareModifiedTime = (
  a: FileItem,
  b: FileItem,
  direction: "asc" | "desc",
) => {
  const aTime = typeof a.mtime === "number" ? a.mtime : null;
  const bTime = typeof b.mtime === "number" ? b.mtime : null;

  if (aTime === null && bTime === null) return compareNames(a, b);
  if (aTime === null) return 1;
  if (bTime === null) return -1;

  const result = direction === "asc" ? aTime - bTime : bTime - aTime;
  return result || compareNames(a, b);
};

export const sortFileItemsForView = (
  items: FileItem[],
  mode: FileSortMode,
) =>
  [...items].sort((a, b) => {
    const directorySort = Number(b.is_dir) - Number(a.is_dir);
    if (directorySort !== 0) return directorySort;

    if (mode === "mtime-desc") return compareModifiedTime(a, b, "desc");
    if (mode === "mtime-asc") return compareModifiedTime(a, b, "asc");

    if (mode === "size") {
      const result = Number(b.size || 0) - Number(a.size || 0);
      return result || compareNames(a, b);
    }

    if (mode === "type") {
      const result = getFileExtension(a.name).localeCompare(
        getFileExtension(b.name),
      );
      return result || compareNames(a, b);
    }

    return compareNames(a, b);
  });
