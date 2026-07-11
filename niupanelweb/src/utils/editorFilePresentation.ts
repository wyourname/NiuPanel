const FILE_ICON_MAP: Record<string, string> = {
  py: "i-ep-files",
  js: "i-ep-files",
  ts: "i-ep-files",
  sh: "i-ep-monitor",
  bash: "i-ep-monitor",
  json: "i-ep-document",
  html: "i-ep-files",
  css: "i-ep-files",
  vue: "i-ep-files",
  yaml: "i-ep-document",
  yml: "i-ep-document",
  toml: "i-ep-document",
  md: "i-ep-notebook",
  rs: "i-ep-files",
  go: "i-ep-files",
  sql: "i-ep-coin",
  xml: "i-ep-document",
  svg: "i-ep-picture",
  ini: "i-ep-setting",
  conf: "i-ep-setting",
  env: "i-ep-setting",
  log: "i-ep-document",
  txt: "i-ep-document",
  lock: "i-ep-lock",
  gitignore: "i-ep-setting",
};

const FILE_ICON_COLOR_MAP: Record<string, string> = {
  py: "text-blue-500",
  js: "text-yellow-500",
  ts: "text-blue-600",
  sh: "text-green-500",
  bash: "text-green-500",
  json: "text-orange-400",
  html: "text-orange-500",
  css: "text-blue-400",
  vue: "text-emerald-500",
  yaml: "text-pink-400",
  yml: "text-pink-400",
  toml: "text-gray-500",
  md: "text-purple-400",
  rs: "text-orange-600",
  go: "text-cyan-500",
  sql: "text-amber-500",
  xml: "text-orange-400",
  svg: "text-yellow-400",
  ini: "text-gray-500",
  conf: "text-gray-500",
  env: "text-yellow-600",
  log: "text-gray-400",
  txt: "text-gray-400",
};

const getFileExtension = (filename: string) =>
  filename.split(".").pop()?.toLowerCase() || "";

export function getFileIcon(filename: string): string {
  if (!filename) return "i-ep-document";

  const lower = filename.toLowerCase();
  if (lower === "dockerfile") return "i-ep-box";
  if (lower === "makefile") return "i-ep-setting";

  return FILE_ICON_MAP[getFileExtension(filename)] || "i-ep-document";
}

export function getFileIconColor(filename: string): string {
  if (!filename) return "text-gray-400";
  return FILE_ICON_COLOR_MAP[getFileExtension(filename)] || "text-gray-400";
}
