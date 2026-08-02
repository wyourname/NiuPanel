<template>
  <div
    class="h-full overflow-y-auto custom-scrollbar p-0 md:p-2"
    @scroll.passive="handleScroll"
  >
    <TaskCardItem
      v-for="task in tasks"
      :key="task.id"
      :task="task"
      :is-selected="isSelected(task)"
      :selection-mode="selectionMode"
      @selection-change="
        (t: Task, val: boolean) => emit('selection-change', t, val)
      "
      @run="emit('run', $event)"
      @stop="emit('stop', $event)"
      @pause="emit('pause', $event)"
      @resume="emit('resume', $event)"
      @enable="emit('enable', $event)"
      @disable="emit('disable', $event)"
      @edit="emit('edit', $event)"
      @share="emit('share', $event)"
      @logs="emit('logs', $event)"
      @toggle-enable="(t: Task, val: boolean) => emit('toggle-enable', t, val)"
      @delete="emit('delete', $event)"
      @pin="emit('pin', $event)"
      @unpin="emit('unpin', $event)"
      @edit-variables="emit('edit-variables', $event)"
      @quick-edit-cron="emit('quick-edit-cron', $event)"
      @more-actions="emit('more-actions', $event)"
      @enter-selection="emit('enter-selection', $event)"
      @edit-cron="emit('edit-cron', $event)"
    />

    <div v-if="loading" class="flex justify-center py-4 text-primary">
      <div class="i-ep-loading animate-spin text-xl"></div>
    </div>

    <div
      v-if="noMore && tasks.length > 0"
      class="text-center py-4 text-xs text-muted"
    >
      没有更多任务了
    </div>

    <div
      v-if="tasks.length === 0 && !loading"
      class="flex flex-col items-center justify-center px-6 py-14 text-center text-muted"
    >
      <div class="h-11 w-11 rounded-md bg-soft flex-center mb-3">
        <div class="i-ep-folder-opened text-[20px]"></div>
      </div>
      <span class="text-[13px] font-bold text-default">还没有任务</span>
      <span class="mt-1 text-[11px]">点击右下角按钮创建第一个任务</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import TaskCardItem from "./TaskCardItem.vue";
import type { Task } from "@/types";

const props = withDefaults(
  defineProps<{
    loading?: boolean;
    noMore?: boolean;
    selectedTasks?: Task[];
    selectionMode?: boolean;
    tasks: Task[];
  }>(),
  {
    loading: false,
    noMore: false,
    selectedTasks: () => [],
    selectionMode: false,
  },
);

const emit = defineEmits<{
  (e: "selection-change", task: Task, val: boolean): void;
  (e: "run", id: number): void;
  (e: "stop", id: number): void;
  (e: "pause", id: number): void;
  (e: "resume", id: number): void;
  (e: "enable", id: number): void;
  (e: "disable", id: number): void;
  (e: "edit", task: Task): void;
  (e: "share", task: Task): void;
  (e: "logs", task: Task): void;
  (e: "toggle-enable", task: Task, val: boolean): void;
  (e: "delete", id: number): void;
  (e: "pin", id: number): void;
  (e: "unpin", id: number): void;
  (e: "edit-variables", id: number): void;
  (e: "quick-edit-cron", task: Task): void;
  (e: "more-actions", task: Task): void;
  (e: "enter-selection", task: Task): void;
  (e: "load-more"): void;
  (e: "edit-cron", task: Task): void;
}>();

const isSelected = (task: Task) => {
  return props.selectedTasks.some((t) => t.id === task.id);
};

const handleScroll = (event: Event) => {
  if (props.loading || props.noMore) return;

  const target = event.currentTarget as HTMLElement;
  const remaining = target.scrollHeight - target.scrollTop - target.clientHeight;
  if (remaining <= 10) emit("load-more");
};
</script>
