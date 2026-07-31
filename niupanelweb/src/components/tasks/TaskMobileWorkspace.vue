<template>
  <TaskMobileListPane
    v-model:search-query="searchQueryValue"
    v-model:status-filter="statusFilterValue"
    :all-tasks="allTasks"
    :is-all-selected="isAllSelected"
    :loading="loading"
    :no-more="noMore"
    :refresh-tasks="refreshTasks"
    :selected-ids="selectedIds"
    :selected-tasks="selectedTasks"
    :selection-mode="selectionMode"
    :tasks="tasks"
    :total-tasks="totalTasks"
    @bulk-command="emit('bulk-command', $event)"
    @bulk-delete="emit('bulk-delete')"
    @bulk-pause="emit('bulk-pause')"
    @bulk-run="emit('bulk-run')"
    @bulk-stop="emit('bulk-stop')"
    @cancel-selection="emit('cancel-selection')"
    @create="emit('create')"
    @delete="emit('delete', $event)"
    @disable="emit('disable', $event)"
    @edit="emit('edit', $event)"
    @edit-cron="emit('edit-cron', $event)"
    @edit-variables="emit('edit-variables', $event)"
    @enable="emit('enable', $event)"
    @enter-selection="emit('enter-selection', $event)"
    @load-more="emit('load-more')"
    @logs="emit('logs', $event)"
    @more-actions="emit('more-actions', $event)"
    @open-create-sheet="createActionSheetVisibleValue = true"
    @pause="emit('pause', $event)"
    @pin="emit('pin', $event)"
    @resume="emit('resume', $event)"
    @run="emit('run', $event)"
    @select-all="emit('select-all')"
    @selection-change="(task, selected) => emit('selection-change', task, selected)"
    @share="emit('share', $event)"
    @stop="emit('stop', $event)"
    @toggle-enable="(task, enabled) => emit('toggle-enable', task, enabled)"
    @unpin="emit('unpin', $event)"
  />

  <TaskMobileActionSheet
    v-model:visible="actionSheetVisibleValue"
    :task="currentActiveTask"
    @command="emit('mobile-action-command', $event)"
  />

  <TaskMobileCreateSheet
    v-model:visible="createActionSheetVisibleValue"
    @create="emit('create')"
    @quick-create="emit('quick-create')"
  />
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Task } from "@/types";
import type {
  TaskBulkCommand,
  TaskMobileActionCommand,
} from "../../composables/taskPageTypes";
import TaskMobileActionSheet from "./TaskMobileActionSheet.vue";
import TaskMobileCreateSheet from "./TaskMobileCreateSheet.vue";
import TaskMobileListPane from "./TaskMobileListPane.vue";

const props = defineProps<{
  actionSheetVisible: boolean;
  allTasks: Task[];
  createActionSheetVisible: boolean;
  currentActiveTask: Task | null;
  filteredTaskCount: number;
  loading: boolean;
  noMore: boolean;
  refreshTasks: () => unknown;
  searchQuery: string;
  selectedIds: number[];
  selectedTasks: Task[];
  selectionMode: boolean;
  statusFilter: string;
  tasks: Task[];
  totalTasks: number;
}>();

const emit = defineEmits<{
  (event: "bulk-command", command: TaskBulkCommand): void;
  (event: "bulk-delete"): void;
  (event: "bulk-pause"): void;
  (event: "bulk-run"): void;
  (event: "bulk-stop"): void;
  (event: "cancel-selection"): void;
  (event: "create"): void;
  (event: "delete", id: number): void;
  (event: "disable", id: number): void;
  (event: "edit", task: Task): void;
  (event: "edit-cron", task: Task): void;
  (event: "edit-variables", taskId: number): void;
  (event: "enable", id: number): void;
  (event: "enter-selection", task: Task): void;
  (event: "load-more"): void;
  (event: "logs", task: Task): void;
  (event: "mobile-action-command", command: TaskMobileActionCommand): void;
  (event: "more-actions", task: Task): void;
  (event: "pause", id: number): void;
  (event: "pin", id: number): void;
  (event: "quick-create"): void;
  (event: "resume", id: number): void;
  (event: "run", id: number): void;
  (event: "select-all"): void;
  (event: "selection-change", task: Task, selected: boolean): void;
  (event: "share", task: Task): void;
  (event: "stop", id: number): void;
  (event: "toggle-enable", task: Task, enabled: boolean): void;
  (event: "unpin", id: number): void;
  (event: "update:actionSheetVisible", visible: boolean): void;
  (event: "update:createActionSheetVisible", visible: boolean): void;
  (event: "update:searchQuery", value: string): void;
  (event: "update:statusFilter", value: string): void;
}>();

const model = <T,>(
  get: () => T,
  update: (value: T) => void,
) => computed({
  get,
  set: update,
});

const actionSheetVisibleValue = model(
  () => props.actionSheetVisible,
  (value) => emit("update:actionSheetVisible", value),
);
const createActionSheetVisibleValue = model(
  () => props.createActionSheetVisible,
  (value) => emit("update:createActionSheetVisible", value),
);
const searchQueryValue = model(
  () => props.searchQuery,
  (value) => emit("update:searchQuery", value),
);
const statusFilterValue = model(
  () => props.statusFilter,
  (value) => emit("update:statusFilter", value),
);

const isAllSelected = computed(
  () =>
    props.selectedIds.length === props.filteredTaskCount &&
    props.filteredTaskCount > 0,
);
</script>
