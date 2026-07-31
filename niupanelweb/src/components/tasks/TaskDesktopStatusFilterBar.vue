<template>
  <div
    class="px-3.5 py-2 flex items-center gap-1 overflow-x-auto no-scrollbar border-b border-base bg-card"
  >
    <button
      v-for="item in statusPills"
      :key="item.value"
      class="h-8 shrink-0 rounded-md px-2.5 text-[11px] font-semibold transition-colors outline-none"
      :class="
        statusValue === item.value
          ? 'accent-subtle'
          : 'text-muted hover:bg-black/5 dark:hover:bg-white/5'
      "
      @click="statusValue = item.value"
    >
      {{ statusLabels[item.value] || item.label }}
      <span class="ml-1 opacity-60">{{ statusCount(item.value) }}</span>
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
