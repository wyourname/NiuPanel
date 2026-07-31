<template>
  <div class="flex-1 overflow-hidden flex flex-col relative">
    <div class="flex-1 overflow-hidden relative">
      <TaskDesktopLogWorkspace
        v-show="activeTab === 'log'"
        ref="desktopLogWorkspaceRef"
        :log-progress-value="logProgressValue"
        :log-qr-code-data="logQrCodeData"
        :runs="runs"
        :selected-run-id="selectedRunId"
        :show-log-progress="showLogProgress"
        :task-status="task.status"
        :timeline-has-more="timelineHasMore"
        :timeline-loading="timelineLoading"
        :timeline-page="timelinePage"
        @expand-qr="emit('expand-qr')"
        @load-more="emit('load-more-timeline')"
        @refresh="emit('refresh-timeline')"
        @select="emit('select-timeline', $event)"
        @ui-event="emit('ui-event', $event)"
      />

      <TaskDesktopScriptWorkspace
        v-if="activeTab === 'script'"
        v-model:content="scriptContentValue"
        :language="scriptLanguage"
        :loading="scriptLoading"
        :options="scriptEditorOptions"
        :saving="scriptSaving"
        :task="task"
        @editor-mount="emit('script-editor-mount', $event)"
        @save="emit('save-script')"
      />

      <div
        v-if="activeTab === 'var'"
        class="full bg-white dark:bg-[#0e1621]"
      >
        <TaskVariableEditor
          :task-id="task.id"
          @success="emit('variables-saved')"
        />
      </div>

      <TaskInfoPanel
        v-if="activeTab === 'info'"
        :task="task"
        @view-log="(logPath, runId) => emit('view-log', logPath, runId)"
      />
    </div>

    <TaskDetailFooterDock
      :active-tab="activeTab"
      :task="task"
      @action="emit('action', $event)"
      @edit="emit('edit', task)"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import type { Task } from "@/types";
import type {
  TaskDetailTab,
  TaskEditorOptions,
  TaskFooterAction,
  TaskLogViewerRef,
  TaskLogUiEvent,
  TaskRunTimelineItem,
  TaskScriptEditorRef,
} from "../../composables/taskPageTypes";
import { useTaskLogViewerBridge } from "../../composables/useTaskLogViewerBridge";
import TaskDesktopLogWorkspace from "./TaskDesktopLogWorkspace.vue";
import TaskDesktopScriptWorkspace from "./TaskDesktopScriptWorkspace.vue";
import TaskDetailFooterDock from "./TaskDetailFooterDock.vue";
import TaskInfoPanel from "./TaskInfoPanel.vue";
import TaskVariableEditor from "./TaskVariableEditor.vue";

const props = defineProps<{
  activeTab: TaskDetailTab;
  logProgressValue: number;
  logQrCodeData: string | null;
  runs: TaskRunTimelineItem[];
  scriptContent: string;
  scriptEditorOptions: TaskEditorOptions;
  scriptLanguage: string;
  scriptLoading: boolean;
  scriptSaving: boolean;
  selectedRunId: number | null;
  showLogProgress: boolean;
  task: Task;
  timelineHasMore: boolean;
  timelineLoading: boolean;
  timelinePage: number;
}>();

const emit = defineEmits<{
  (event: "action", action: TaskFooterAction): void;
  (event: "edit", task: Task): void;
  (event: "expand-qr"): void;
  (event: "load-more-timeline"): void;
  (event: "refresh-timeline"): void;
  (event: "save-script"): void;
  (event: "script-editor-mount", editor: TaskScriptEditorRef): void;
  (event: "select-timeline", runId: number | null): void;
  (event: "ui-event", payload: TaskLogUiEvent): void;
  (event: "update:scriptContent", content: string): void;
  (event: "variables-saved"): void;
  (event: "view-log", logPath: string, runId: number): void;
}>();

const scriptContentValue = computed({
  get: () => props.scriptContent,
  set: (value: string) => emit("update:scriptContent", value),
});

const desktopLogWorkspaceRef = ref<TaskLogViewerRef | null>(null);
defineExpose(useTaskLogViewerBridge(desktopLogWorkspaceRef));
</script>
