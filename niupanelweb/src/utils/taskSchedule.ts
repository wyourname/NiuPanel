interface RandomConfig {
  start: string;
  end: string;
  count: number;
}

interface TaskScheduleInput {
  cron_schedule?: string;
  next_run_at?: string;
  random_config?: RandomConfig | null;
}

export function getRandomScheduleSummary(randomConfig?: RandomConfig | null) {
  if (!randomConfig) return "";
  return `${randomConfig.start} - ${randomConfig.end} 随机 ${randomConfig.count} 次`;
}

export function getTaskScheduleSummary(task: TaskScheduleInput) {
  const randomSummary = getRandomScheduleSummary(task.random_config);

  if (task.next_run_at) {
    const date = new Date(task.next_run_at);
    const now = new Date();
    const dateLabel =
      date.toDateString() === now.toDateString()
        ? "今天"
        : date.toLocaleDateString([], { month: "short", day: "numeric" });
    const timeLabel = date.toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });

    return randomSummary
      ? `${dateLabel} ${timeLabel} · ${randomSummary}`
      : `${dateLabel} ${timeLabel}`;
  }

  if (randomSummary) return randomSummary;
  return task.cron_schedule || "手动触发";
}
