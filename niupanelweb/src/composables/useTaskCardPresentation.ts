import { computed, type Ref } from "vue";
import type { Task } from "@/types";
import { getEnvIcon as resolveEnvIcon } from "./useTaskPresentation";

export function useTaskCardPresentation(task: Ref<Task>) {
  const getEnvIcon = computed(() => resolveEnvIcon(task.value));

  const statusBarClass = computed(() => {
    if (!task.value.enabled) return "bg-gray-300 dark:bg-gray-600";

    const map: Record<string, string> = {
      Running: "bg-emerald-500 shadow-[0_0_6px_rgba(16,185,129,0.5)]",
      Paused: "bg-amber-400",
      Failed: "bg-rose-500",
      Stopped: "bg-gray-400",
      Pending: "bg-blue-400",
      Success: "bg-emerald-400",
      Finished: "bg-emerald-400",
    };

    return map[task.value.status] || "bg-gray-300";
  });

  const statusDotClass = computed(() => {
    const map: Record<string, string> = {
      Running: "bg-emerald-500",
      Paused: "bg-amber-500",
      Stopped: "bg-gray-400",
      Failed: "bg-rose-500",
      Pending: "bg-blue-400",
      Success: "bg-emerald-500",
      Finished: "bg-emerald-500",
    };

    return map[task.value.status] || "bg-gray-400";
  });

  const statusBadgeClass = computed(() => {
    const map: Record<string, string> = {
      Running: "text-emerald-600 bg-emerald-500/10",
      Paused: "text-amber-600 bg-amber-500/10",
      Stopped: "text-gray-500 bg-gray-500/10",
      Failed: "text-rose-600 bg-rose-500/10",
      Pending: "text-blue-500 bg-blue-500/10",
      Success: "text-emerald-600 bg-emerald-500/10",
      Finished: "text-emerald-600 bg-emerald-500/10",
    };

    return map[task.value.status] || "text-gray-500 bg-gray-500/10";
  });

  const statusText = computed(() => {
    const map: Record<string, string> = {
      Idle: "待运行",
      Running: "执行中",
      Paused: "暂停",
      Stopped: "已停止",
      Failed: "失败",
      Pending: "队列中",
      Finished: "已完成",
      Cancelled: "已取消",
    };

    return map[task.value.status] || task.value.status;
  });

  const scheduleInfo = computed(() => {
    if (task.value.random_config) {
      const { start, end, count } = task.value.random_config;
      return `${start} 至 ${end} 随机运行 ${count} 次`;
    }

    if (task.value.cron_schedule) {
      const fields = task.value.cron_schedule.trim().split(/\s+/);
      const values = fields.length === 6 ? fields.slice(1) : fields;
      const [minute, hour, day, month, weekday] = values;
      const time = /^\d+$/.test(hour ?? "") && /^\d+$/.test(minute ?? "")
        ? `${hour.padStart(2, "0")}:${minute.padStart(2, "0")}`
        : "";

      if (minute?.startsWith("*/") && hour === "*") return `每 ${minute.slice(2)} 分钟`;
      if (time && day === "*" && month === "*" && ["1-5", "MON-FRI"].includes(weekday?.toUpperCase() ?? "")) {
        return `工作日 ${time}`;
      }
      if (time && day === "*" && month === "*" && weekday === "*") return `每天 ${time}`;
      if (time && day === "*" && month === "*" && /^\d$/.test(weekday ?? "")) {
        const weekdayLabels = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];
        return `每${weekdayLabels[Number(weekday)]} ${time}`;
      }
      if (time && /^\d+$/.test(day ?? "") && month === "*") return `每月 ${day} 日 ${time}`;
      return "按计划运行";
    }

    return "手动运行";
  });

  const formatLastTime = computed(() => {
    if (!task.value.last_finished_at) return "尚未运行";

    const date = new Date(task.value.last_finished_at);
    const now = new Date();

    if (date.toDateString() === now.toDateString()) {
      return `今天 ${date.toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      })}`;
    }

    return date.toLocaleDateString([], { month: "numeric", day: "numeric" });
  });

  const nextRunText = computed(() => {
    if (!task.value.enabled || !task.value.next_run_at) return "";

    const date = new Date(task.value.next_run_at);
    if (Number.isNaN(date.getTime())) return "";

    const now = new Date();
    const tomorrow = new Date(now);
    tomorrow.setDate(now.getDate() + 1);

    const time = date.toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });

    if (date.toDateString() === now.toDateString()) return `今天 ${time}`;
    if (date.toDateString() === tomorrow.toDateString()) return `明天 ${time}`;

    return `${date.toLocaleDateString([], {
      month: "numeric",
      day: "numeric",
    })} ${time}`;
  });

  const environmentLabel = computed(() => {
    const type = task.value.env_type === "sh" ? "Shell" : task.value.env_type === "node" ? "Node.js" : "Python";
    return task.value.env_version ? `${type} ${task.value.env_version}` : type;
  });

  const primaryActionLabel = computed(() => {
    if (task.value.status === "Running") return "查看日志";
    if (task.value.status === "Failed") return "重试";
    if (task.value.status === "Paused" || !task.value.enabled) return "查看";
    return "运行";
  });

  const primaryActionIcon = computed(() => {
    if (task.value.status === "Running") return "i-ep-document";
    if (task.value.status === "Failed") return "i-ep-refresh-right";
    if (task.value.status === "Paused" || !task.value.enabled) return "i-ep-view";
    return "i-ep-video-play";
  });

  return {
    formatLastTime,
    environmentLabel,
    getEnvIcon,
    nextRunText,
    primaryActionIcon,
    primaryActionLabel,
    scheduleInfo,
    statusBadgeClass,
    statusBarClass,
    statusDotClass,
    statusText,
  };
}
