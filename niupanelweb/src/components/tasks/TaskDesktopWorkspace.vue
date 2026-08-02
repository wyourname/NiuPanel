<template>
  <section class="flex-1 flex flex-col bg-card relative overflow-hidden">
    <template v-if="task">
      <TaskDetailHeader
        v-model:active-tab="activeTabValue"
        :is-mobile="false"
        :show-search="showSearch"
        :task="task"
        @back="emit('back')"
        @command="emit('detail-command', $event)"
        @toggle-search="emit('toggle-search')"
      />

      <div
        class="flex-1 overflow-hidden flex flex-col relative bg-base"
      >
        <TaskLogSearchOverlay
          ref="searchOverlayRef"
          v-model:query="logSearchQueryValue"
          v-model:visible="showSearchValue"
        />

        <TaskDesktopDetailWorkspace
          ref="detailWorkspaceRef"
          v-model:script-content="scriptContentValue"
          :active-tab="activeTab"
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
          @edit="emit('edit', $event)"
          @expand-qr="emit('expand-qr')"
          @load-more-timeline="emit('load-more-timeline')"
          @refresh-timeline="emit('refresh-timeline')"
          @save-script="emit('save-script')"
          @script-editor-mount="emit('script-editor-mount', $event)"
          @select-timeline="emit('select-timeline', $event)"
          @ui-event="emit('ui-event', $event)"
          @variables-saved="emit('variables-saved')"
          @view-log="(logPath, runId) => emit('view-log', logPath, runId)"
        />
      </div>
    </template>

    <TaskEmptyWorkspace v-else />
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import type { Task } from "@/types";
import type {
  TaskDetailMoreCommand,
  TaskDetailTab,
  TaskEditorOptions,
  TaskFocusableRef,
  TaskFooterAction,
  TaskLogViewerRef,
  TaskLogUiEvent,
  TaskRunTimelineItem,
  TaskScriptEditorRef,
} from "../../composables/taskPageTypes";
import { useTaskLogViewerBridge } from "../../composables/useTaskLogViewerBridge";
import TaskDesktopDetailWorkspace from "./TaskDesktopDetailWorkspace.vue";
import TaskDetailHeader from "./TaskDetailHeader.vue";
import TaskEmptyWorkspace from "./TaskEmptyWorkspace.vue";
import TaskLogSearchOverlay from "./TaskLogSearchOverlay.vue";

const props = defineProps<{
  activeTab: TaskDetailTab;
  logProgressValue: number;
  logQrCodeData: string | null;
  logSearchQuery: string;
  runs: TaskRunTimelineItem[];
  scriptContent: string;
  scriptEditorOptions: TaskEditorOptions;
  scriptLanguage: string;
  scriptLoading: boolean;
  scriptSaving: boolean;
  selectedRunId: number | null;
  showLogProgress: boolean;
  showSearch: boolean;
  task?: Task;
  timelineHasMore: boolean;
  timelineLoading: boolean;
  timelinePage: number;
}>();

const emit = defineEmits<{
  (event: "action", action: TaskFooterAction): void;
  (event: "back"): void;
  (event: "detail-command", command: TaskDetailMoreCommand): void;
  (event: "edit", task: Task): void;
  (event: "expand-qr"): void;
  (event: "load-more-timeline"): void;
  (event: "refresh-timeline"): void;
  (event: "save-script"): void;
  (event: "script-editor-mount", editor: TaskScriptEditorRef): void;
  (event: "select-timeline", runId: number | null): void;
  (event: "toggle-search"): void;
  (event: "ui-event", payload: TaskLogUiEvent): void;
  (event: "update:activeTab", tab: TaskDetailTab): void;
  (event: "update:logSearchQuery", query: string): void;
  (event: "update:scriptContent", content: string): void;
  (event: "update:showSearch", visible: boolean): void;
  (event: "variables-saved"): void;
  (event: "view-log", logPath: string, runId: number): void;
}>();

const activeTabValue = computed({
  get: () => props.activeTab,
  set: (value: TaskDetailTab) => emit("update:activeTab", value),
});

const logSearchQueryValue = computed({
  get: () => props.logSearchQuery,
  set: (value: string) => emit("update:logSearchQuery", value),
});

const scriptContentValue = computed({
  get: () => props.scriptContent,
  set: (value: string) => emit("update:scriptContent", value),
});

const showSearchValue = computed({
  get: () => props.showSearch,
  set: (value: boolean) => emit("update:showSearch", value),
});

const detailWorkspaceRef = ref<TaskLogViewerRef | null>(null);
const searchOverlayRef = ref<TaskFocusableRef | null>(null);
const logViewerBridge = useTaskLogViewerBridge(detailWorkspaceRef);

defineExpose({
  ...logViewerBridge,
  focus: () => searchOverlayRef.value?.focus?.(),
});
</script>
