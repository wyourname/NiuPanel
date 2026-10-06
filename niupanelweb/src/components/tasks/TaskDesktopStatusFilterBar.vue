<template>
  <div
    class="task-status-filters shrink-0 border-b border-base bg-card"
    role="group"
    aria-label="按任务状态筛选"
  >
    <button
      v-for="item in statusPills"
      :key="item.value"
      type="button"
      :aria-pressed="statusValue === item.value"
      :title="`${statusLabels[item.value] || item.label}：${statusCount(item.value)} 个任务`"
      :aria-label="`${statusLabels[item.value] || item.label}：${statusCount(item.value)} 个任务`"
      class="task-status-filter rounded-md text-[11px] font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
      :class="
        statusValue === item.value
          ? 'accent-subtle'
          : 'text-muted hover:bg-black/5 dark:hover:bg-white/5'
      "
      @click="statusValue = item.value"
    >
      {{ statusLabels[item.value] || item.label }}
      <span class="task-status-count" aria-hidden="true">{{ statusCount(item.value) > 99 ? '99+' : statusCount(item.value) }}</span>
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Task } from "@/types";
import { statusPills } from "../../composables/useTaskPresentation";

const props = defineProps<{
  statusFilter: string;
  tasks: Task[];
}>();

const emit = defineEmits<{
  (event: "update:statusFilter", value: string): void;
}>();

const statusLabels: Record<string, string> = {
  all: "全部",
  Running: "执行中",
  Paused: "已暂停",
  Stopped: "已停止",
  Failed: "失败",
};

const statusValue = computed({
  get: () => props.statusFilter,
  set: (value: string) => emit("update:statusFilter", value),
});

const statusCount = (status: string) =>
  status === "all" ? props.tasks.length : props.tasks.filter((task) => task.status === status).length;
</script>

<style scoped>
.task-status-filters { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 2px; padding: 8px; container-type: inline-size; }
.task-status-filter { display: flex; align-items: center; justify-content: center; gap: 3px; min-width: 0; min-height: 32px; padding: 0 2px; white-space: nowrap; cursor: pointer; }
.task-status-count { font-size: 10px; font-weight: 500; font-variant-numeric: tabular-nums; opacity: .65; }
@container (max-width: 280px) { .task-status-count { display: none; } }
</style>
