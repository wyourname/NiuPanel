import { computed, onUnmounted, ref } from "vue";
import { CronExpressionParser } from "cron-parser";
import type { Task } from "@/types";

export type ScheduleStationKind = "past-ok" | "past-fail" | "running" | "future";

export interface ScheduleStation {
  time: number;
  task: Task;
  kind: ScheduleStationKind;
}

const DAY_MS = 86_400_000;
// 防止 "* * * * *" 之类的高频表达式撑爆时间轴
const MAX_OCCURRENCES_PER_TASK = 24;

/**
 * 把任务列表换算成"今日时间轴"上的站点:
 * 过去 = 今天完成过的运行(成功/失败),现在 = 运行中,未来 = cron/next_run_at 推算的计划。
 */
export const useTodaySchedule = (getTasks: () => Task[]) => {
  // clock 每秒走(驱动时钟与 now 游标);tick 每 30 秒走(驱动 cron 重算,避免每秒重新解析)
  const clock = ref(Date.now());
  const tick = ref(Date.now());
  const clockTimer = window.setInterval(() => {
    clock.value = Date.now();
  }, 1000);
  const tickTimer = window.setInterval(() => {
    tick.value = Date.now();
  }, 30_000);
  onUnmounted(() => {
    window.clearInterval(clockTimer);
    window.clearInterval(tickTimer);
  });

  const dayStart = computed(() => new Date(tick.value).setHours(0, 0, 0, 0));
  const dayEnd = computed(() => dayStart.value + DAY_MS);

  const stations = computed<ScheduleStation[]>(() => {
    const list: ScheduleStation[] = [];

    for (const task of getTasks()) {
      if (task.status === "Running") {
        list.push({ time: tick.value, task, kind: "running" });
      } else if (task.last_finished_at) {
        const finishedAt = Date.parse(task.last_finished_at);
        if (
          Number.isFinite(finishedAt) &&
          finishedAt >= dayStart.value &&
          finishedAt <= tick.value
        ) {
          list.push({
            time: finishedAt,
            task,
            kind: task.status === "Failed" ? "past-fail" : "past-ok",
          });
        }
      }

      if (!task.enabled) continue;

      let scheduled = false;
      if (task.cron_schedule) {
        try {
          const expression = CronExpressionParser.parse(task.cron_schedule, {
            currentDate: new Date(tick.value),
          });
          for (let i = 0; i < MAX_OCCURRENCES_PER_TASK; i += 1) {
            const next = expression.next().getTime();
            if (next > dayEnd.value) break;
            list.push({ time: next, task, kind: "future" });
            scheduled = true;
          }
        } catch {
          // 表达式无法解析时退回后端给的 next_run_at
        }
      }

      if (!scheduled && task.next_run_at) {
        const nextAt = Date.parse(task.next_run_at);
        if (
          Number.isFinite(nextAt) &&
          nextAt >= tick.value &&
          nextAt <= dayEnd.value
        ) {
          list.push({ time: nextAt, task, kind: "future" });
        }
      }
    }

    return list.sort((a, b) => a.time - b.time);
  });

  const upcomingCount = computed(
    () => stations.value.filter((station) => station.kind === "future").length,
  );

  return { clock, dayStart, dayEnd, stations, upcomingCount };
};
