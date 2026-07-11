import type { FileItem } from "@/types";

const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico"];
const ARCHIVE_EXTENSIONS = ["zip", "tar", "gz", "tgz"];
const EDITABLE_EXTENSIONS = [
  "py",
  "js",
  "sh",
  "ts",
  "txt",
  "md",
  "json",
  "yml",
  "yaml",
  "toml",
  "html",
  "css",
  "vue",
  "log",
  "conf",
  "ini",
  "rs",
  "go",
  "lock",
  "gitignore",
  "sql",
  "xml",
];

export const isImageFile = (name: string) => {
  const ext = name.split(".").pop()?.toLowerCase() || "";
  return IMAGE_EXTENSIONS.includes(ext);
};

export const isEditableFile = (name: string) => {
  const ext = name.split(".").pop()?.toLowerCase() || "";
  return EDITABLE_EXTENSIONS.includes(ext);
};

export const isArchiveFile = (name: string) => {
  const lowerName = name.toLowerCase();
  const ext = lowerName.split(".").pop() || "";
  return lowerName.endsWith(".tar.gz") || ARCHIVE_EXTENSIONS.includes(ext);
};

export const getFileExtension = (name: string) => {
  const lowerName = name.toLowerCase();
  if (lowerName.endsWith(".tar.gz")) return "tar.gz";
  return lowerName.includes(".") ? lowerName.split(".").pop() || "" : "";
};

export const getFileTypeLabel = (row: FileItem) => {
  if (row.is_dir) return "目录";
  const ext = getFileExtension(row.name);
  return ext ? ext.toUpperCase() : "文件";
};

export const getFileIconClass = (row: FileItem) => {
  if (row.is_dir) return "i-ep-folder";
  const ext = getFileExtension(row.name);
  if (ext === "py") return "i-logos-python";
  if (ext === "js" || ext === "ts") return "i-logos-nodejs-icon";
  if (ext === "sh") return "i-carbon-terminal";
  if (isImageFile(row.name)) return "i-ep-picture-filled";
  if (isArchiveFile(row.name)) return "i-ep-box";
  if (ext === "json" || ext === "yml" || ext === "yaml") return "i-ep-document";
  if (ext === "md" || ext === "txt" || ext === "log") return "i-ep-notebook";
  if (ext === "html" || ext === "css" || ext === "vue") return "i-ep-magic-stick";
  return "i-ep-document";
};

export const getFileIconBgClass = (row: FileItem) => {
  if (row.is_dir) return "bg-amber-50 dark:bg-amber-950/25 text-amber-500";
  const ext = getFileExtension(row.name);
  if (ext === "py") return "bg-blue-50 dark:bg-blue-950/25 text-blue-500";
  if (ext === "js" || ext === "ts") return "bg-yellow-40/30 dark:bg-yellow-900/20 text-yellow-600 dark:text-yellow-400";
  if (ext === "sh") return "bg-slate-100 dark:bg-slate-800/35 text-slate-500 dark:text-slate-400";
  if (isImageFile(row.name)) return "bg-purple-50 dark:bg-purple-950/25 text-purple-500";
  if (isArchiveFile(row.name)) return "bg-orange-50 dark:bg-orange-950/25 text-orange-500";
  if (ext === "json" || ext === "yml" || ext === "yaml") return "bg-emerald-50 dark:bg-emerald-950/25 text-emerald-500";
  if (ext === "md" || ext === "txt" || ext === "log") return "bg-gray-100 dark:bg-gray-800/30 text-gray-500 dark:text-gray-400";
  if (ext === "html" || ext === "css" || ext === "vue") return "bg-pink-50 dark:bg-pink-950/25 text-pink-500";
  return "bg-sky-50 dark:bg-sky-950/25 text-sky-500";
};

export const formatRelativeFileDate = (timestamp: number) => {
  const date = new Date(timestamp * 1000);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  if (days === 0) return "今天";
  if (days === 1) return "昨天";
  if (days < 7) return `${days}天前`;
  return date.toLocaleDateString([], { month: "short", day: "numeric" });
};

export const formatFullFileDate = (timestamp: number) => {
  const date = new Date(timestamp * 1000);
  return date.toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
};
