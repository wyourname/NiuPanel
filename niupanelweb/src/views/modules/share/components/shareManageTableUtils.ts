import type { StationFile } from "@/types";

export const getShareDisplayName = (item: StationFile) =>
  item.note || item.file_key || item.fileKey || "未命名资源";

export const getDownloadsRemaining = (item: StationFile) =>
  item.downloadsRemaining ?? item.downloads_remaining ?? -1;

export const isDeleteOnDownload = (item: StationFile) =>
  item.deleteOnDownload ?? item.delete_on_download ?? false;

export const getExpiresAt = (item: StationFile) =>
  item.expiresAt ?? item.expires_at ?? null;

export const isExpired = (item: StationFile) => {
  const expiresAt = getExpiresAt(item);
  return !!expiresAt && Date.now() > expiresAt * 1000;
};

export const formatShareSize = (bytes: number) => {
  if (!bytes) return "-";
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(2) + " KB";
  return (bytes / (1024 * 1024)).toFixed(2) + " MB";
};

export const formatShareDate = (timestamp: number) =>
  new Date(timestamp * 1000).toLocaleString();
