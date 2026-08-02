<template>
  <template v-if="!isMobile">
    <TaskDesktopNavigator
      v-model:search-query="searchQueryValue"
      v-model:selection-mode="selectionModeValue"
      v-model:status-filter="statusFilterValue"
      :all-tasks="allTasks"
      :loading="loading"
      :no-more="noMore"
      :selected-ids="selectedIds"
      :selected-task-id="selectedTaskId"
      :tasks="tasks"
      :total-tasks="totalTasks"
      @bulk-disable="emit('bulk-disable')"
      @bulk-enable="emit('bulk-enable')"
      @bulk-more-command="emit('bulk-more-command', $event)"
      @bulk-pause="emit('bulk-pause')"
      @bulk-pin="emit('bulk-pin')"
      @bulk-run="emit('bulk-run')"
      @bulk-share="emit('bulk-share')"
      @bulk-stop="emit('bulk-stop')"
      @cancel-selection="emit('cancel-selection')"
      @context-command="(command, task) => emit('context-command', command, task)"
      @create="emit('create')"
      @delete="emit('delete', $event)"
      @load-more="emit('load-more')"
      @more-actions="emit('more-actions', $event)"
      @quick-create="emit('quick-create')"
      @run="emit('run', $event)"
      @select-all="emit('select-all')"
      @select-task="emit('select-task', $event)"
      @selection-change="(task, selected) => emit('selection-change', task, selected)"
      @stop="emit('stop', $event)"
      @toggle-enable="(task, enabled) => emit('toggle-enable', task, enabled)"
    />

    <TaskDesktopWorkspace
      ref="workspaceRef"
      v-model:active-tab="activeTabValue"
      v-model:log-search-query="logSearchQueryValue"
      v-model:script-content="scriptContentValue"
      v-model:show-search="showSearchValue"
      :log-progress-value="logProgressValue"
      :log-qr-code-data="logQrCodeData"
      :runs="runs"
      :selected-run-id="selectedRunId"
      :show-log-progress="showLogProgress"
      :script-language="scriptLanguage"
      :script-loading="scriptLoading"
      :script-editor-options="scriptEditorOptions"
      :script-saving="scriptSaving"
      :task="task"
      :timeline-has-more="timelineHasMore"
      :timeline-loading="timelineLoading"
      :timeline-page="timelinePage"
      @action="emit('action', $event)"
      @back="emit('back')"
      @detail-command="emit('detail-command', $event)"
      @edit="emit('edit', $event)"
      @expand-qr="emit('expand-qr')"
      @load-more-timeline="emit('load-more-timeline')"
      @refresh-timeline="emit('refresh-timeline')"
      @save-script="emit('save-script')"
      @script-editor-mount="emit('script-editor-mount', $event)"
      @select-timeline="emit('select-timeline', $event)"
      @toggle-search="emit('toggle-search')"
      @ui-event="emit('ui-event', $event)"
      @variables-saved="emit('variables-saved')"
      @view-log="(logPath, runId) => emit('view-log', logPath, runId)"
    />
  </template>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import type { Task } from "@/types";
import type {
  TaskBulkMoreCommand,
  TaskContextCommand,
  TaskDetailMoreCommand,
  TaskDetailTab,
  TaskEditorOptions,
  TaskFocusableRef,
  TaskFooterAction,
  TaskLogFetcher,
  TaskLogUiEvent,
  TaskLogViewerRef,
  TaskRunTimelineItem,
  TaskScriptEditorRef,
} from "../../composables/taskPageTypes";
import TaskDesktopNavigator from "./TaskDesktopNavigator.vue";
import TaskDesktopWorkspace from "./TaskDesktopWorkspace.vue";

type TaskDesktopLayoutExpose = TaskLogViewerRef & TaskFocusableRef;

const props = defineProps<{
  activeTab: TaskDetailTab;
  allTasks: Task[];
  isMobile: boolean;
  loading: boolean;
  logProgressValue: number;
  logQrCodeData: string | null;
  logSearchQuery: string;
  noMore: boolean;
  runs: TaskRunTimelineItem[];
  scriptContent: string;
  scriptEditorOptions: TaskEditorOptions;
  scriptLanguage: string;
  scriptLoading: boolean;
  scriptSaving: boolean;
  searchQuery: string;
  selectedIds: number[];
  selectedRunId: number | null;
  selectedTaskId: number | null;
  selectionMode: boolean;
  showLogProgress: boolean;
  showSearch: boolean;
  statusFilter: string;
  task?: Task;
  tasks: Task[];
  timelineHasMore: boolean;
  timelineLoading: boolean;
  timelinePage: number;
  totalTasks: number;
}>();

const emit = defineEmits<{
  (event: "action", action: TaskFooterAction): void;
  (event: "back"): void;
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
  (event: "detail-command", command: TaskDetailMoreCommand): void;
  (event: "edit", task: Task): void;
  (event: "expand-qr"): void;
  (event: "load-more"): void;
  (event: "load-more-timeline"): void;
  (event: "more-actions", task: Task): void;
  (event: "quick-create"): void;
  (event: "refresh-timeline"): void;
  (event: "run", id: number): void;
  (event: "save-script"): void;
  (event: "script-editor-mount", editor: TaskScriptEditorRef): void;
  (event: "select-all"): void;
  (event: "select-task", task: Task): void;
  (event: "select-timeline", runId: number | null): void;
  (event: "selection-change", task: Task, value: boolean): void;
  (event: "stop", id: number): void;
  (event: "toggle-enable", task: Task, enabled: boolean): void;
  (event: "toggle-search"): void;
  (event: "ui-event", payload: TaskLogUiEvent): void;
  (event: "update:activeTab", tab: TaskDetailTab): void;
  (event: "update:logSearchQuery", query: string): void;
  (event: "update:scriptContent", content: string): void;
  (event: "update:searchQuery", value: string): void;
  (event: "update:selectionMode", value: boolean): void;
  (event: "update:showSearch", visible: boolean): void;
  (event: "update:statusFilter", value: string): void;
  (event: "variables-saved"): void;
  (event: "view-log", logPath: string, runId: number): void;
}>();

const model = <T,>(
  get: () => T,
  update: (value: T) => void,
) => computed({
  get,
  set: update,
});

const activeTabValue = model(
  () => props.activeTab,
  (value) => emit("update:activeTab", value),
);
const logSearchQueryValue = model(
  () => props.logSearchQuery,
  (value) => emit("update:logSearchQuery", value),
);
const scriptContentValue = model(
  () => props.scriptContent,
  (value) => emit("update:scriptContent", value),
);
const searchQueryValue = model(
  () => props.searchQuery,
  (value) => emit("update:searchQuery", value),
);
const selectionModeValue = model(
  () => props.selectionMode,
  (value) => emit("update:selectionMode", value),
);
const showSearchValue = model(
  () => props.showSearch,
  (value) => emit("update:showSearch", value),
);
const statusFilterValue = model(
  () => props.statusFilter,
  (value) => emit("update:statusFilter", value),
);

const workspaceRef = ref<TaskDesktopLayoutExpose | null>(null);

defineExpose({
  clear: () => workspaceRef.value?.clear?.(),
  focus: () => workspaceRef.value?.focus?.(),
  init: (loader: TaskLogFetcher) => {
    workspaceRef.value?.init?.(loader);
  },
  reset: () => workspaceRef.value?.reset?.(),
  scrollToBottom: () => workspaceRef.value?.scrollToBottom?.(),
  setSearch: (query: string, jumpToNext: boolean) => {
    workspaceRef.value?.setSearch?.(query, jumpToNext);
  },
  toggleWrap: () => workspaceRef.value?.toggleWrap?.(),
  write: (data: unknown) => workspaceRef.value?.write?.(data),
  writeln: (data: string) => workspaceRef.value?.writeln?.(data),
});
</script>
