<template>
  <TaskShareDialog
    v-if="shareVisibleValue"
    v-model:visible="shareVisibleValue"
    :tasks="tasksToShare"
  />

  <TaskWizardDialog
    v-model:visible="wizardVisibleValue"
    :task="editingTask"
    @success="emit('wizard-success')"
  />

  <TaskMobileVariableDrawer
    v-if="isMobile"
    v-model:visible="variableEditorVisibleValue"
    :task-id="currentTaskForVariables"
    @success="emit('variable-success')"
  />

  <TaskMobileScriptDrawer
    v-if="isMobile"
    v-model:content="scriptEditorContentValue"
    v-model:visible="scriptEditorVisibleValue"
    :language="dialogEditorLanguage"
    :loading="scriptEditorLoading"
    :options="dialogEditorOptions"
    :ready="scriptEditorReady"
    :task="currentScriptTask"
    :word-wrap="editorWordWrap"
    @drawer-close="emit('script-drawer-close')"
    @editor-command="emit('script-editor-command', $event)"
    @editor-mount="emit('script-editor-mount', $event)"
    @opened="emit('script-opened')"
    @request-close="emit('script-request-close')"
    @save="emit('script-save')"
    @toggle-word-wrap="emit('toggle-word-wrap')"
  />

  <TaskCronEditorDialog
    v-model:visible="cronEditVisibleValue"
    v-model:cron="cronInputValue"
    v-model:enable-random="enableRandomValue"
    v-model:random-config="randomConfigValue"
    :saving="cronSaving"
    @save="emit('save-cron')"
  />

  <TaskQuickCreateDialog
    v-model:visible="quickCreateVisibleValue"
    v-model:url="quickCreateUrlValue"
    :creating="quickCreating"
    @submit="emit('submit-quick-create', $event)"
  />

  <TaskHistoryLogDialog
    v-model:visible="historyLogVisibleValue"
    :content="historyLogContent"
    :loading="historyLogLoading"
    :run-id="historyLogRunId"
  />

  <el-image-viewer
    v-if="expandLogQrValue && logQrCodeData"
    :url-list="[logQrCodeData]"
    @close="expandLogQrValue = false"
  />
</template>

<script setup lang="ts">
import { computed } from "vue";
import type {
  Task,
  TaskRandomConfig,
} from "@/types";
import type { TaskWizardInitialData } from "../../composables/useTaskWizardData";
import type {
  TaskEditorOptions,
  TaskMobileScriptEditorCommand,
  TaskScriptEditorRef,
} from "../../composables/taskPageTypes";
import TaskShareDialog from "../common/TaskShareDialog.vue";
import TaskCronEditorDialog from "./TaskCronEditorDialog.vue";
import TaskHistoryLogDialog from "./TaskHistoryLogDialog.vue";
import TaskMobileScriptDrawer from "./TaskMobileScriptDrawer.vue";
import TaskMobileVariableDrawer from "./TaskMobileVariableDrawer.vue";
import TaskQuickCreateDialog from "./TaskQuickCreateDialog.vue";
import TaskWizardDialog from "./TaskWizardDialog.vue";

const props = defineProps<{
  cronEditVisible: boolean;
  cronInput: string;
  cronSaving: boolean;
  currentScriptTask: Task | null;
  currentTaskForVariables: number | null;
  dialogEditorLanguage: string;
  dialogEditorOptions: TaskEditorOptions;
  editingTask: TaskWizardInitialData | null;
  editorWordWrap: boolean;
  enableRandom: boolean;
  expandLogQr: boolean;
  historyLogContent: string;
  historyLogLoading: boolean;
  historyLogRunId: number | null;
  historyLogVisible: boolean;
  isMobile: boolean;
  logQrCodeData: string | null;
  quickCreateUrl: string;
  quickCreateVisible: boolean;
  quickCreating: boolean;
  randomConfig: TaskRandomConfig;
  scriptEditorContent: string;
  scriptEditorLoading: boolean;
  scriptEditorReady: boolean;
  scriptEditorVisible: boolean;
  shareVisible: boolean;
  tasksToShare: Task[];
  variableEditorVisible: boolean;
  wizardVisible: boolean;
}>();

const emit = defineEmits<{
  (event: "save-cron"): void;
  (event: "script-drawer-close"): void;
  (event: "script-editor-command", command: TaskMobileScriptEditorCommand): void;
  (event: "script-editor-mount", editor: TaskScriptEditorRef): void;
  (event: "script-opened"): void;
  (event: "script-request-close"): void;
  (event: "script-save"): void;
  (event: "submit-quick-create", url: string): void;
  (event: "toggle-word-wrap"): void;
  (event: "update:cronEditVisible", visible: boolean): void;
  (event: "update:cronInput", cron: string): void;
  (event: "update:enableRandom", enabled: boolean): void;
  (event: "update:expandLogQr", visible: boolean): void;
  (event: "update:historyLogVisible", visible: boolean): void;
  (event: "update:quickCreateUrl", url: string): void;
  (event: "update:quickCreateVisible", visible: boolean): void;
  (event: "update:randomConfig", value: TaskRandomConfig): void;
  (event: "update:scriptEditorContent", content: string): void;
  (event: "update:scriptEditorVisible", visible: boolean): void;
  (event: "update:shareVisible", visible: boolean): void;
  (event: "update:variableEditorVisible", visible: boolean): void;
  (event: "update:wizardVisible", visible: boolean): void;
  (event: "variable-success"): void;
  (event: "wizard-success"): void;
}>();

const model = <T,>(
  get: () => T,
  update: (value: T) => void,
) => computed({
  get,
  set: update,
});

const cronEditVisibleValue = model(
  () => props.cronEditVisible,
  (value) => emit("update:cronEditVisible", value),
);
const cronInputValue = model(
  () => props.cronInput,
  (value) => emit("update:cronInput", value),
);
const enableRandomValue = model(
  () => props.enableRandom,
  (value) => emit("update:enableRandom", value),
);
const expandLogQrValue = model(
  () => props.expandLogQr,
  (value) => emit("update:expandLogQr", value),
);
const historyLogVisibleValue = model(
  () => props.historyLogVisible,
  (value) => emit("update:historyLogVisible", value),
);
const quickCreateUrlValue = model(
  () => props.quickCreateUrl,
  (value) => emit("update:quickCreateUrl", value),
);
const quickCreateVisibleValue = model(
  () => props.quickCreateVisible,
  (value) => emit("update:quickCreateVisible", value),
);
const randomConfigValue = model(
  () => props.randomConfig,
  (value) => emit("update:randomConfig", value),
);
const scriptEditorContentValue = model(
  () => props.scriptEditorContent,
  (value) => emit("update:scriptEditorContent", value),
);
const scriptEditorVisibleValue = model(
  () => props.scriptEditorVisible,
  (value) => emit("update:scriptEditorVisible", value),
);
const shareVisibleValue = model(
  () => props.shareVisible,
  (value) => emit("update:shareVisible", value),
);
const variableEditorVisibleValue = model(
  () => props.variableEditorVisible,
  (value) => emit("update:variableEditorVisible", value),
);
const wizardVisibleValue = model(
  () => props.wizardVisible,
  (value) => emit("update:wizardVisible", value),
);
</script>
