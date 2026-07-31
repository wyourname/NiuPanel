<template>
  <section class="glass-card px-5 pb-3 pt-4">
    <header class="flex items-center justify-between">
      <div class="flex items-baseline gap-2">
        <h2 class="text-[12px] font-semibold text-secondary">今日计划</h2>
        <span class="text-[11px] text-muted">{{ headerMeta }}</span>
      </div>
      <div class="flex items-baseline gap-2">
        <span class="text-[11px] text-muted">{{ dateLabel }}</span>
        <span class="font-mono text-[13px] font-semibold tabular-nums text-default">{{ clockLabel }}</span>
      </div>
    </header>

    <div class="relative mt-2 h-[86px]">
      <!-- 轴线:已过去的部分着色 -->
      <div class="absolute inset-x-0 top-[42px] h-[2px] rounded-full bg-[var(--border-light)]"></div>
      <div
        class="absolute left-0 top-[42px] h-[2px] rounded-full bg-[var(--accent-subtle-border)]"
        :style="{ width: `${nowPct}%` }"
      ></div>

      <!-- 小时刻度 -->
      <template v-for="tickItem in hourTicks" :key="tickItem.hour">
        <div
          class="absolute top-[38px] h-[9px] w-px bg-[var(--border-base)] opacity-70"
          :style="{ left: `${tickItem.pct}%` }"
        ></div>
        <div
          v-if="tickItem.label"
          class="absolute top-[56px] -translate-x-1/2 font-mono text-[10px] tabular-nums text-muted"
          :style="{ left: `${tickItem.pct}%` }"
        >{{ tickItem.label }}</div>
      </template>

      <!-- now 游标 -->
      <div
        class="absolute top-[22px] h-[34px] w-px bg-primary"
        :style="{ left: `${nowPct}%` }"
      ></div>
      <div
        class="absolute top-0 -translate-x-1/2 rounded accent-subtle px-1.5 py-0.5 font-mono text-[10px] font-bold tabular-nums"
        :style="{ left: `${cursorChipPct}%` }"
      >{{ cursorLabel }}</div>

      <!-- 运行站点 -->
      <div
        v-for="cluster in clusters"
        :key="cluster.key"
        class="group absolute top-[42px] z-10"
        :style="{ left: `${cluster.pct}%` }"
      >
        <button
          type="button"
          class="relative flex h-7 w-7 -translate-x-1/2 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full"
          :aria-label="clusterAria(cluster)"
          @click="handleClusterClick(cluster)"
        >
          <span
            v-if="clusterKinds(cluster).length <= 1"
            class="h-[10px] w-[10px] rounded-full ring-2 transition-transform group-hover:scale-125"
            :class="dotClass(cluster)"
          ></span>
          <span
            v-else
            class="flex items-center gap-[3px] transition-transform group-hover:scale-110"
          >
            <span
              v-for="kind in clusterKinds(cluster)"
              :key="kind"
              class="h-[8px] w-[8px] rounded-full ring-2 ring-[var(--bg-card)]"
              :class="stationDotClass(kind)"
            ></span>
          </span>
          <span
            v-if="cluster.items.length > 1"
            class="absolute -right-0.5 -top-0.5 rounded-full bg-card px-1 text-[9px] font-bold leading-[14px] text-secondary ring-1 ring-[var(--border-base)]"
          >{{ cluster.items.length }}</span>
        </button>

        <!-- 站点名(站点较少时直接展示) -->
        <div
          v-if="showInlineLabels"
          class="pointer-events-none absolute left-0 top-[12px] -translate-x-1/2 whitespace-nowrap"
        >
          <div class="max-w-[92px] truncate text-[10px] font-semibold" :class="labelClass(cluster)">
            {{ cluster.items[0].task.name }}<template v-if="cluster.items.length > 1"> +{{ cluster.items.length - 1 }}</template>
          </div>
        </div>

        <!-- 悬停详情 -->
        <div
          class="absolute bottom-[16px] z-30 hidden w-[264px] rounded-lg border border-light bg-card p-1.5 shadow-md group-hover:block"
          :class="popoverAlignClass(cluster)"
        >
          <div class="px-2 pb-1 pt-1.5 font-mono text-[10px] font-bold tabular-nums text-muted">{{ fmtHM(cluster.time) }}</div>
          <div
            v-for="station in cluster.items"
            :key="stationKey(station)"
            class="flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-soft"
          >
            <span class="h-[6px] w-[6px] shrink-0 rounded-full" :class="stationDotClass(station.kind)"></span>
            <button
              type="button"
              class="min-w-0 flex-1 cursor-pointer truncate text-left text-[12px] font-semibold text-default hover:text-primary"
              :title="station.task.name"
              @click="emit('open-log', station.task)"
            >{{ station.task.name }}</button>
            <span class="shrink-0 text-[10px] text-muted">{{ stationMeta(station) }}</span>
            <button
              type="button"
              class="h-6 w-6 shrink-0 cursor-pointer rounded text-secondary flex-center hover:bg-active hover:text-default"
              title="查看日志"
              @click="emit('open-log', station.task)"
            >
              <span class="i-ep-document-copy text-[12px]"></span>
            </button>
            <button
              v-if="station.kind !== 'running'"
              type="button"
              class="h-6 w-6 shrink-0 cursor-pointer rounded text-secondary flex-center hover:bg-active hover:text-primary"
              :title="station.kind === 'future' ? '立即运行' : '再次运行'"
              @click="emit('run', station.task)"
            >
              <span
                :class="station.kind === 'past-fail' ? 'i-ep-refresh-right' : 'i-ep-video-play'"
                class="text-[12px]"
              ></span>
            </button>
          </div>
        </div>
      </div>

      <!-- 空状态 -->
      <div v-if="!clusters.length" class="absolute inset-x-0 top-[8px] flex justify-center">
        <p class="text-[11px] text-muted">
          <template v-if="loading">正在读取任务…</template>
          <template v-else>
            今日没有计划运行。
            <button type="button" class="link-primary font-semibold" @click="emit('create')">新建一个定时任务</button>
          </template>
        </p>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Task } from "@/types";
import type {
  ScheduleStation,
  ScheduleStationKind,
} from "@/composables/useTodaySchedule";

interface StationCluster {
  key: string;
  pct: number;
  time: number;
  items: ScheduleStation[];
}

const props = defineProps<{
  stations: ScheduleStation[];
  clock: number;
  dayStart: number;
  loading?: boolean;
}>();

const emit = defineEmits<{
  (e: "open-log", task: Task): void;
  (e: "run", task: Task): void;
  (e: "create"): void;
}>();

const DAY_MS = 86_400_000;
// 时间轴上 ~35 分钟内的运行聚合为一个站点,避免密集时相互压盖
const CLUSTER_SPAN_PCT = 2.4;

const KIND_PRIORITY: Record<ScheduleStationKind, number> = {
  running: 3,
  "past-fail": 2,
  future: 1,
  "past-ok": 0,
};

const clampPct = (pct: number) => Math.min(98.5, Math.max(1.5, pct));

const timePct = (time: number) =>
  clampPct(((time - props.dayStart) / DAY_MS) * 100);

const nowPct = computed(() =>
  Math.min(100, Math.max(0, ((props.clock - props.dayStart) / DAY_MS) * 100)),
);
const cursorChipPct = computed(() => Math.min(96, Math.max(4, nowPct.value)));

const clusters = computed<StationCluster[]>(() => {
  const result: StationCluster[] = [];
  for (const station of props.stations) {
    const pct = timePct(station.time);
    const last = result[result.length - 1];
    if (last && pct - last.pct <= CLUSTER_SPAN_PCT) {
      last.items.push(station);
    } else {
      result.push({ key: stationKey(station), pct, time: station.time, items: [station] });
    }
  }
  return result;
});

const showInlineLabels = computed(() => clusters.value.length > 0 && clusters.value.length <= 6);

const hourTicks = [0, 3, 6, 9, 12, 15, 18, 21, 24].map((hour) => ({
  hour,
  pct: (hour / 24) * 100,
  label: hour % 6 === 0 ? String(hour).padStart(2, "0") : "",
}));

const clusterKind = (cluster: StationCluster): ScheduleStationKind =>
  cluster.items.reduce<ScheduleStationKind>(
    (kind, station) =>
      KIND_PRIORITY[station.kind] > KIND_PRIORITY[kind] ? station.kind : kind,
    "past-ok",
  );

// 站点内出现的不同状态,按优先级从高到低去重;混合状态时并排显示多个小色点
const clusterKinds = (cluster: StationCluster): ScheduleStationKind[] => {
  const seen = new Set<ScheduleStationKind>();
  for (const station of cluster.items) seen.add(station.kind);
  return [...seen].sort((a, b) => KIND_PRIORITY[b] - KIND_PRIORITY[a]);
};

const dotClass = (cluster: StationCluster) => {
  switch (clusterKind(cluster)) {
    case "running":
      return "bg-emerald-500 ring-emerald-500/30 timeline-dot-pulse";
    case "past-fail":
      return "bg-rose-500 ring-rose-500/25";
    case "past-ok":
      return "bg-emerald-400/80 ring-emerald-400/20";
    default:
      return "bg-primary ring-primary/25";
  }
};

const stationDotClass = (kind: ScheduleStationKind) => {
  switch (kind) {
    case "running":
      return "bg-emerald-500";
    case "past-fail":
      return "bg-rose-500";
    case "past-ok":
      return "bg-emerald-400/80";
    default:
      return "bg-primary";
  }
};

const labelClass = (cluster: StationCluster) => {
  switch (clusterKind(cluster)) {
    case "running":
      return "text-emerald-600 dark:text-emerald-400";
    case "past-fail":
      return "text-rose-600 dark:text-rose-400";
    default:
      return "text-secondary";
  }
};

const popoverAlignClass = (cluster: StationCluster) => {
  if (cluster.pct < 14) return "left-[-12px]";
  if (cluster.pct > 86) return "right-[-12px]";
  return "left-1/2 -translate-x-1/2";
};

const stationKey = (station: ScheduleStation) =>
  `${station.task.id}-${station.kind}-${station.time}`;

const pad = (value: number) => String(value).padStart(2, "0");

const fmtHM = (time: number) => {
  const date = new Date(time);
  return `${pad(date.getHours())}:${pad(date.getMinutes())}`;
};

const fmtEta = (time: number) => {
  const diff = time - props.clock;
  if (diff <= 60_000) return "即将运行";
  const minutes = Math.round(diff / 60_000);
  if (minutes < 60) return `${minutes} 分钟后`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest ? `${hours} 时 ${rest} 分后` : `${hours} 小时后`;
};

const stationMeta = (station: ScheduleStation) => {
  switch (station.kind) {
    case "running":
      return "运行中";
    case "past-fail":
      return "失败";
    case "past-ok":
      return "已完成";
    default:
      return fmtEta(station.time);
  }
};

const clusterAria = (cluster: StationCluster) =>
  `${fmtHM(cluster.time)} ${cluster.items.map((item) => item.task.name).join("、")}`;

const handleClusterClick = (cluster: StationCluster) => {
  if (cluster.items.length === 1) emit("open-log", cluster.items[0].task);
};

const headerMeta = computed(() => {
  const upcoming = props.stations.filter((s) => s.kind === "future").length;
  const done = props.stations.filter(
    (s) => s.kind === "past-ok" || s.kind === "past-fail",
  ).length;
  const running = props.stations.filter((s) => s.kind === "running").length;
  const parts: string[] = [];
  if (running) parts.push(`${running} 个运行中`);
  if (upcoming) parts.push(`${upcoming} 次待运行`);
  if (done) parts.push(`${done} 次已完成`);
  return parts.length ? parts.join(" · ") : "暂无计划";
});

const WEEKDAYS = ["日", "一", "二", "三", "四", "五", "六"];

const dateLabel = computed(() => {
  const date = new Date(props.clock);
  return `${date.getMonth() + 1}月${date.getDate()}日 周${WEEKDAYS[date.getDay()]}`;
});

const clockLabel = computed(() => {
  const date = new Date(props.clock);
  return `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
});

const cursorLabel = computed(() => fmtHM(props.clock));
</script>

<style scoped>
@media (prefers-reduced-motion: no-preference) {
  .timeline-dot-pulse {
    animation: timeline-pulse 2s ease-in-out infinite;
  }
}

@keyframes timeline-pulse {
  0%,
  100% {
    box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.35);
  }
  50% {
    box-shadow: 0 0 0 6px rgba(16, 185, 129, 0);
  }
}
</style>
