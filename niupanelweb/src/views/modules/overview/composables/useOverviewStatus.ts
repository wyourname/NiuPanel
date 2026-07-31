import type { TaskStatus } from "@/types";

const statusDotClassMap: Partial<Record<TaskStatus, string>> = {
  Running: "bg-primary animate-pulse",
  Finished: "bg-emerald-500",
  Failed: "bg-rose-500",
};

const statusTextClassMap: Partial<Record<TaskStatus, string>> = {
  Running: "text-primary",
  Finished: "text-emerald-600",
  Failed: "text-rose-600",
};

const statusLabelMap: Partial<Record<TaskStatus, string>> = {
  Running: "运行中",
  Finished: "已完成",
  Failed: "已失败",
};

export function useOverviewStatus() {
  const getStatusDotClass = (status: TaskStatus) => {
    return statusDotClassMap[status] || "bg-slate-400";
  };

  const getStatusTextClass = (status: TaskStatus) => {
    return statusTextClassMap[status] || "text-gray-500";
  };

  const getStatusLabel = (status: TaskStatus) => {
    return statusLabelMap[status] || status;
  };

  return {
    getStatusDotClass,
    getStatusLabel,
    getStatusTextClass,
  };
}
