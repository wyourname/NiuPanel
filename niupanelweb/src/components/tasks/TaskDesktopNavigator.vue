<template>
  <aside class="w-[clamp(350px,30vw,420px)] flex shrink-0 flex-col border-r border-base bg-card z-20">
    <TaskDesktopNavigatorHeader
      v-model:search-query="searchValue"
      v-model:selection-mode="selectionModeValue"
      :is-all-selected="isAllSelected"
      :selected-count="selectedIds.length"
      :total-tasks="totalTasks"
      @cancel-selection="emit('cancel-selection')"
      @create="emit('create')"
      @quick-create="emit('quick-create')"
      @select-all="emit('select-all')"
    />

    <transition name="el-fade-in" mode="out-in">
      <TaskDesktopBulkToolbar
        v-if="selectionMode"
        key="bulk-bar"
        @bulk-disable="emit('bulk-disable')"
        @bulk-enable="emit('bulk-enable')"
        @bulk-more-command="emit('bulk-more-command', $event)"
        @bulk-pause="emit('bulk-pause')"
        @bulk-pin="emit('bulk-pin')"
        @bulk-run="emit('bulk-run')"
        @bulk-share="emit('bulk-share')"
        @bulk-stop="emit('bulk-stop')"
      />
      <TaskDesktopStatusFilterBar
        v-else
        key="filter-bar"
        v-model:status-filter="statusValue"
        :tasks="allTasks"
      />
    </transition>

    <TaskDesktopTaskList
      :loading="loading"
      :no-more="noMore"
      :selected-ids="selectedIds"
      :selected-task-id="selectedTaskId"
      :selection-mode="selectionMode"
      :tasks="tasks"
      :total-tasks="totalTasks"
      @context-command="(command, task) => emit('context-command', command, task)"
      @delete="emit('delete', $event)"
      @load-more="emit('load-more')"
      @more-actions="emit('more-actions', $event)"
      @run="emit('run', $event)"
      @select-task="emit('select-task', $event)"
      @selection-change="(task, selected) => emit('selection-change', task, selected)"
      @stop="emit('stop', $event)"
      @toggle-enable="(task, enabled) => emit('toggle-enable', task, enabled)"
    />
  </aside>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Task } from "@/types";
import type {
  TaskBulkMoreCommand,
  TaskContextCommand,
} from "../../composables/taskPageTypes";
import TaskDesktopBulkToolbar from "./TaskDesktopBulkToolbar.vue";
import TaskDesktopNavigatorHeader from "./TaskDesktopNavigatorHeader.vue";
import TaskDesktopStatusFilterBar from "./TaskDesktopStatusFilterBar.vue";
import TaskDesktopTaskList from "./TaskDesktopTaskList.vue";

const props = defineProps<{
  allTasks: Task[];
  loading: boolean;
  noMore: boolean;
  searchQuery: string;
  selectedIds: number[];
  selectedTaskId: number | null;
  selectionMode: boolean;
  statusFilter: string;
  tasks: Task[];
  totalTasks: number;
}>();

const emit = defineEmits<{
  (event: "bulk-disable"): void;
  (event: "bulk-enable"): void;
  (event: "bulk-more-command", command: TaskBulkMoreCommand): void;
  (event: "bulk-pause"): void;
  (event: "bulk-pin"): void;
  (event: "bulk-run"): void;
  (event: "bulk-share"): void;
  (event: "bulk-stop"): void;
  (event: "cancel-selection"): void;
  (event: "context-command", command: TaskContextCommand, task: Task): void;
  (event: "create"): void;
  (event: "delete", id: number): void;
  (event: "load-more"): void;
  (event: "more-actions", task: Task): void;
  (event: "quick-create"): void;
  (event: "run", id: number): void;
  (event: "select-all"): void;
  (event: "select-task", task: Task): void;
  (event: "selection-change", task: Task, value: boolean): void;
  (event: "stop", id: number): void;
  (event: "toggle-enable", task: Task, enabled: boolean): void;
  (event: "update:searchQuery", value: string): void;
  (event: "update:selectionMode", value: boolean): void;
  (event: "update:statusFilter", value: string): void;
}>();

const isAllSelected = computed(
  () => props.selectedIds.length === props.tasks.length && props.tasks.length > 0,
);

const searchValue = computed({
  get: () => props.searchQuery,
  set: (value: string) => emit("update:searchQuery", value),
});

const statusValue = computed({
  get: () => props.statusFilter,
  set: (value: string) => emit("update:statusFilter", value),
});

const selectionModeValue = computed({
  get: () => props.selectionMode,
  set: (value: boolean) => emit("update:selectionMode", value),
});
</script>
