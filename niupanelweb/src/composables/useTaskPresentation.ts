import type { Task } from "@/types";

export const ENV_LANGUAGE_MAP: Record<string, string> = {
  node: "javascript",
  python3: "python",
  python: "python",
  sh: "shell",
  bash: "shell",
};

export const statusPills = [
  { label: "All", value: "all" },
  { label: "Active", value: "Running" },
  { label: "Paused", value: "Paused" },
  { label: "Stopped", value: "Stopped" },
  { label: "Failed", value: "Failed" },
];

export const detailTabs = [
  { label: "Console", value: "log", icon: "i-ep-terminal" },
  { label: "Vars", value: "var", icon: "i-ep-key" },
  { label: "Editor", value: "script", icon: "i-ep-edit-pen" },
  { label: "Info", value: "info", icon: "i-ep-info-filled" },
];

export const getStatusLabel = (status: string | undefined): string => {
  switch (status) {
    case "Running":
      return "运行中";
    case "Pending":
      return "队列中";
    case "Finished":
    case "Success":
      return "已完成";
    case "Failed":
      return "失败";
    case "Paused":
      return "已暂停";
    case "Stopped":
      return "已停止";
    case "Cancelled":
      return "已取消";
    case "Idle":
      return "空闲";
    default:
      return status || "未知";
  }
};

export const getAvatarBgClass = (task: Task | null | undefined) => {
  if (!task) return "bg-gray-100 text-gray-400 dark:bg-gray-800";
  const name = task.name.toLowerCase();
  if (name.includes("ddns"))
    return "bg-gradient-to-br from-blue-400 to-blue-600";
  if (name.includes("签到") || name.includes("checkin"))
    return "bg-gradient-to-br from-emerald-400 to-emerald-600";
  if (name.includes("api") || name.includes("sync"))
    return "bg-gradient-to-br from-purple-400 to-purple-600";
  if (name.includes("db") || name.includes("backup"))
    return "bg-gradient-to-br from-amber-400 to-orange-500";
  if (name.includes("test"))
    return "bg-gradient-to-br from-gray-400 to-gray-600";

  const colors = [
    "from-blue-500 to-indigo-600",
    "from-emerald-400 to-teal-500",
    "from-purple-500 to-pink-500",
    "from-orange-400 to-red-500",
    "from-cyan-400 to-blue-500",
  ];
  const idx = task.id ? task.id % colors.length : 0;
  return `bg-gradient-to-br ${colors[idx]}`;
};

export const getEnvIcon = (task: Task | null | undefined) => {
  if (!task) return "i-ep-operation";
  switch (task.env_type) {
    case "node":
      return "i-logos-nodejs-icon";
    case "python3":
    case "python":
      return "i-logos-python";
    case "sh":
    case "bash":
      return "i-carbon-terminal";
    default:
      return "i-ep-document";
  }
};

export const getTimelineStatusDot = (status: string) => {
  if (status === "Finished" || status === "Success")
    return "bg-green-500 shadow-[0_0_8px_rgba(34,197,94,0.6)]";
  if (status === "Failed")
    return "bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.6)]";
  if (status === "Stopped" || status === "Paused")
    return "bg-orange-500 shadow-[0_0_8px_rgba(249,115,22,0.6)]";
  return "bg-blue-500";
};

export const formatDuration = (seconds: number) => {
  if (!seconds) return "-";
  if (seconds < 60) return `${seconds}s`;
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}m ${s}s`;
};

export const getStatusDotClass = (task: Task | null | undefined) => {
  if (!task?.enabled) return "bg-slate-300";
  const map: Record<string, string> = {
    Running:
      "bg-emerald-500 animate-pulse shadow-[0_0_8px_rgba(16,185,129,0.6)]",
    Failed: "bg-rose-500",
    Paused: "bg-amber-500",
  };
  return map[task.status] || "bg-slate-400";
};
