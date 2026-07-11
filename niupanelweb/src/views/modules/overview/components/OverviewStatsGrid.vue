<template>
  <div class="grid w-full shrink-0 grid-cols-2 overflow-hidden rounded-md border border-light bg-card lg:grid-cols-4">
    <button
      type="button"
      class="overview-stat-cell cursor-pointer text-left"
      @click="emit('open-tasks')"
    >
      <div class="flex items-center justify-between">
        <span class="stat-label">运行任务</span>
        <div class="stat-icon bg-emerald-500/10 text-emerald-500">
          <div class="i-ep-video-play text-sm"></div>
        </div>
      </div>
      <el-skeleton :loading="loading" animated>
        <template #default>
          <div class="stat-value text-default">{{ stats.running }}</div>
        </template>
      </el-skeleton>
      <div class="flex items-center gap-1.5">
        <div class="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></div>
        <span class="text-[10px] font-medium text-emerald-600 dark:text-emerald-300">
          {{ stats.running > 0 ? "正在执行" : "当前空闲" }}
        </span>
      </div>
    </button>

    <button
      type="button"
      class="overview-stat-cell cursor-pointer text-left"
      @click="emit('open-tasks')"
    >
      <div class="flex items-center justify-between">
        <span class="stat-label">今日失败</span>
        <div class="stat-icon bg-rose-500/10 text-rose-500">
          <div class="i-ep-warning-filled text-sm"></div>
        </div>
      </div>
      <el-skeleton :loading="loading" animated>
        <template #default>
          <div
            class="stat-value"
            :class="stats.failed_today > 0 ? 'text-rose-500' : 'text-default'"
          >
            {{ stats.failed_today }}
          </div>
        </template>
      </el-skeleton>
      <div class="flex items-center gap-1">
        <div
          class="w-1.5 h-1.5 rounded-full"
          :class="stats.failed_today > 0 ? 'bg-rose-500' : 'bg-emerald-500'"
        ></div>
        <span
          class="text-[9px] font-medium"
          :class="stats.failed_today > 0 ? 'text-rose-500' : 'text-emerald-500'"
        >
          {{ stats.failed_today > 0 ? "需要处理" : "运行正常" }}
        </span>
      </div>
    </button>

    <button
      type="button"
      class="overview-stat-cell cursor-pointer text-left"
      @click="emit('open-tasks')"
    >
      <div class="flex items-center justify-between">
        <span class="stat-label">总任务</span>
        <div class="stat-icon bg-indigo-500/10 text-indigo-500">
          <div class="i-ep-document text-sm"></div>
        </div>
      </div>
      <el-skeleton :loading="loading" animated>
        <template #default>
          <div class="stat-value text-default">{{ stats.total }}</div>
        </template>
      </el-skeleton>
      <div class="flex items-center gap-1">
        <div class="i-ep-timer text-[9px] text-muted"></div>
        <span class="text-[9px] font-mono text-muted">
          下次运行：{{ nextRunTime }}
        </span>
      </div>
    </button>

    <div class="overview-stat-cell">
      <div class="flex items-center justify-between">
        <span class="stat-label">CPU 负载</span>
        <div class="stat-icon bg-amber-500/10 text-amber-500">
          <div class="i-ep-cpu text-sm"></div>
        </div>
      </div>
      <el-skeleton :loading="loading" animated>
        <template #default>
          <div class="stat-value text-default">{{ sysInfo.cpu_usage }}%</div>
        </template>
      </el-skeleton>
      <div class="flex items-center gap-1.5">
        <div class="flex-1 h-1.5 bg-base rounded-full overflow-hidden">
          <div
            class="h-full rounded-full transition-all duration-1000 ease-out"
            :class="cpuBarClass"
            :style="{ width: Math.min(sysInfo.cpu_usage, 100) + '%' }"
          ></div>
        </div>
        <span class="text-[9px] font-mono text-muted">
          内存 {{ formatFileSize(sysInfo.memory_used) }}
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { formatFileSize } from "../../../../utils/format";
import type { OverviewSystemInfo, OverviewTaskStats } from "@/types";

const props = defineProps<{
  loading: boolean;
  nextRunTime: string;
  stats: OverviewTaskStats;
  sysInfo: OverviewSystemInfo;
}>();

const emit = defineEmits<{
  (event: "open-tasks"): void;
}>();

const cpuBarClass = computed(() => {
  if (props.sysInfo.cpu_usage > 80) return "bg-rose-500";
  if (props.sysInfo.cpu_usage > 50) return "bg-amber-500";
  return "bg-emerald-500";
});
</script>

<style scoped>
.overview-stat-cell {
  position: relative;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  min-height: 96px;
  min-width: 0;
  padding: 0.75rem;
  border-right: 1px solid var(--border-light);
  border-bottom: 1px solid var(--border-light);
  background: var(--bg-card);
  transition: background-color 0.2s;
}

.overview-stat-cell:nth-child(2n) {
  border-right: 0;
}

.overview-stat-cell:nth-child(n + 3) {
  border-bottom: 0;
}

button.overview-stat-cell:hover {
  background: var(--bg-soft);
}

@media (min-width: 1024px) {
  .overview-stat-cell {
    min-height: 112px;
    padding: 1rem;
    border-bottom: 0;
  }

  .overview-stat-cell:nth-child(2n) {
    border-right: 1px solid var(--border-light);
  }

  .overview-stat-cell:last-child {
    border-right: 0;
  }
}

.stat-label {
  font-size: 10px;
  font-weight: 600;
  color: var(--text-muted);
}

.stat-icon {
  width: 1.75rem;
  height: 1.75rem;
  border-radius: 0.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
}

@media (min-width: 768px) {
  .stat-icon {
    width: 2rem;
    height: 2rem;
  }
}

.stat-value {
  font-size: 1.5rem;
  line-height: 2rem;
  font-weight: 700;
}

@media (min-width: 768px) {
  .stat-value {
    font-size: 1.75rem;
    line-height: 2rem;
  }
}
</style>
