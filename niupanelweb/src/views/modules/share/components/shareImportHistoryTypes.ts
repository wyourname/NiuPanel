import type { ImportSourceGroup } from "@/types";

export type DeleteTargetType = "task" | "share" | "source";

export const getImportSourceGroupKey = (group: ImportSourceGroup) =>
  group.share_code || group.url;

export const formatImportHistoryDateCompact = (timestamp: number) => {
  const date = new Date(timestamp);
  const pad = (value: number) => value.toString().padStart(2, "0");
  return `${date.getFullYear()}/${pad(date.getMonth() + 1)}/${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
};
