<template>
  <div class="flex h-full overflow-hidden bg-base antialiased relative">
    <!-- DESKTOP LAYOUT -->
    <template v-if="!appStore.isMobile">
      <TaskDesktopLayout
        v-model:search-query="searchQuery"
        v-model:selection-mode="selectionMode"
        v-model:status-filter="statusFilter"
        v-model:active-tab="activeDetailTab"
        v-model:log-search-query="logSearchQuery"
        v-model:script-content="scriptContent"
        v-model:show-search="showSearch"
        :ref="setDesktopWorkspaceRef"
        :is-mobile="appStore.isMobile"
        :all-tasks="taskStore.tasks"
        :loading="taskStore.loading"
        :log-progress-value="logProgressValue"
        :log-qr-code-data="logQrCodeData"
        :no-more="taskStore.noMore"
        :runs="historyTimeline"
        :selected-ids="selectedIds"
        :selected-run-id="selectedHistoryRunId"
        :selected-task-id="selectedTaskId"
        :show-log-progress="showLogProgress"
        :script-language="scriptLanguage"
        :script-loading="scriptLoading"
        :script-editor-options="scriptEditorOptions"
        :script-saving="scriptSaving"
        :task="currentTask"
        :tasks="sortedTasks"
        :timeline-has-more="timelineHasMore"
        :timeline-loading="timelineLoading"
        :timeline-page="timelinePage"
        :total-tasks="taskStore.tasks.length"
        @action="handleAction"
        @back="selectedTaskId = null"
        @bulk-disable="handleBulkDisable"
        @bulk-enable="handleBulkEnable"
        @bulk-more-command="handleBulkMoreCommand"
        @bulk-pause="handleBulkPause"
        @bulk-pin="handleBulkPin"
        @bulk-run="handleBulkRun"
        @bulk-share="handleBulkShare"
        @bulk-stop="handleBulkStop"
        @cancel-selection="handleCancelSelection"
        @context-command="handleContextCommand"
        @create="openCreate(environments)"
        @delete="handleDelete"
        @detail-command="handleMoreCommand"
        @edit="openEdit"
        @expand-qr="expandLogQr = true"
        @load-more="taskStore.loadMoreTasks"
        @load-more-timeline="loadMoreTimeline"
        @more-actions="openActionSheet"
        @open-log-window="openCurrentTaskLogWindow"
        @quick-create="openQuickCreate"
        @refresh-timeline="fetchRunTimeline(currentTask?.id)"
        @run="taskStore.runTask"
        @save-script="saveScriptContent"
        @script-editor-mount="handleScriptEditorMount"
        @select-all="handleSelectAll"
        @select-task="selectTask"
        @select-timeline="selectTimelineRun"
        @selection-change="handleMobileSelection"
        @stop="taskStore.stopTask"
        @toggle-search="toggleHeaderSearch"
        @toggle-enable="handleToggleEnable"
        @ui-event="handleLogUiEvent"
        @variables-saved="taskStore.refreshTasks(true)"
        @view-log="handleHistoryLogViewRequest"
      />
    </template>

    <!-- ========================================== -->
    <!-- MOBILE COMPACT LAYOUT (Nekogram Style)     -->
    <!-- ========================================== -->
    <template v-else>
      <TaskMobileWorkspace
        v-model:action-sheet-visible="actionSheetVisible"
        v-model:create-action-sheet-visible="createActionSheetVisible"
        v-model:search-query="searchQuery"
        v-model:status-filter="statusFilter"
        :current-active-task="currentActiveTask"
        :all-tasks="taskStore.tasks"
        :filtered-task-count="filteredTasks.length"
        :loading="taskStore.loading"
        :no-more="taskStore.noMore"
        :refresh-tasks="refreshMobileTasks"
        :selected-ids="selectedIds"
        :selected-tasks="selectedTasks"
        :selection-mode="selectionMode"
        :tasks="sortedTasks"
        :total-tasks="taskStore.tasks.length"
        @bulk-command="handleBulkCommand"
        @bulk-delete="handleBulkDelete"
        @bulk-pause="handleBulkPause"
        @bulk-run="handleBulkRun"
        @bulk-stop="handleBulkStop"
        @cancel-selection="handleCancelSelection"
        @create="openCreate(environments)"
        @delete="handleDelete"
        @disable="handleTaskDisable"
        @edit="openEdit"
        @edit-cron="openCronEditor"
        @edit-variables="openVariableEditor"
        @enable="handleTaskEnable"
        @enter-selection="handleEnterSelection"
        @load-more="taskStore.loadMoreTasks"
        @logs="openLogs"
        @mobile-action-command="handleMobileActionCommand"
        @more-actions="openActionSheet"
        @pause="taskStore.pauseTask"
        @pin="taskStore.pinTask"
        @quick-create="openQuickCreate"
        @resume="taskStore.resumeTask"
        @run="taskStore.runTask"
        @select-all="handleSelectAll"
        @selection-change="handleMobileSelection"
        @share="openShare"
        @stop="taskStore.stopTask"
        @toggle-enable="handleToggleEnable"
        @unpin="taskStore.unpinTask"
      />
    </template>

    <TaskMobileLogDrawer
      v-if="appStore.isMobile"
      ref="logViewRef"
      v-model="logVisible"
      :task="currentLogTask"
      :runs="historyTimeline"
      :selected-run-id="selectedHistoryRunId"
      :timeline-loading="timelineLoading"
      :timeline-page="timelinePage"
      :timeline-has-more="timelineHasMore"
      :show-log-progress="showLogProgress"
      :log-progress-value="logProgressValue"
      :log-qr-code-data="logQrCodeData"
      :log-search-query="logSearchQuery"
      @action="handleAction"
      @download-logs="downloadLogs"
      @edit="openEdit"
      @edit-cron="openCronEditor"
      @edit-script="handleEditScript"
      @edit-variables="openVariableEditor"
      @expand-qr="expandLogQr = true"
      @load-more-timeline="loadMoreTimeline"
      @refresh-timeline="fetchRunTimeline(currentLogTask?.id)"
      @select-timeline="selectTimelineRun"
      @share="openShare"
      @ui-event="handleLogUiEvent"
    />

    <TaskPageDialogs
      v-model:cron-edit-visible="cronEditVisible"
      v-model:cron-input="cronInput"
      v-model:enable-random="enableRandom"
      v-model:expand-log-qr="expandLogQr"
      v-model:history-log-visible="historyLogVisible"
      v-model:quick-create-url="quickCreateForm.url"
      v-model:quick-create-visible="quickCreateVisible"
      v-model:random-config="randomConfig"
      v-model:script-editor-content="scriptEditorContent"
      v-model:script-editor-visible="scriptEditorVisible"
      v-model:share-visible="shareVisible"
      v-model:variable-editor-visible="variableEditorVisible"
      v-model:wizard-visible="wizardVisible"
      :cron-saving="cronSaving"
      :current-script-task="currentScriptTask"
      :current-task-for-variables="currentTaskForVariables"
      :dialog-editor-language="dialogEditorLanguage"
      :dialog-editor-options="dialogEditorOptions"
      :editing-task="editingTask"
      :editor-word-wrap="editorWordWrap"
      :history-log-content="historyLogContent"
      :history-log-loading="historyLogLoading"
      :history-log-run-id="historyLogRunId"
      :is-mobile="appStore.isMobile"
      :log-qr-code-data="logQrCodeData"
      :quick-creating="quickCreating"
      :script-editor-loading="scriptEditorLoading"
      :script-editor-ready="scriptEditorReady"
      :tasks-to-share="tasksToShare"
      @save-cron="saveCron"
      @script-drawer-close="resetScriptEditorReady"
      @script-editor-command="handleMobileScriptEditorCommand"
      @script-editor-mount="handleScriptEditorMount"
      @script-opened="onScriptDrawerOpened"
      @script-request-close="closeScriptEditor"
      @script-save="saveScript"
      @submit-quick-create="handleQuickCreate"
      @toggle-word-wrap="toggleEditorWordWrap"
      @variable-success="handleVariableEditSuccess"
      @wizard-success="handleWizardSuccess"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { ElMessage } from "element-plus";
import { useTaskStore } from "../../stores/tasks";
import { useAppStore } from "../../stores/app";
import { useWorkspaceStore } from "../../stores/workspace";
import {
  useWorkspaceTaskCommandStore,
  type WorkspaceTaskCommand,
} from "../../stores/workspaceTaskCommands";
import { useTaskActions } from "../../composables/useTaskActions";
import { useTaskCronEditor } from "../../composables/useTaskCronEditor";
import { useTaskLogWorkspace } from "../../composables/useTaskLogWorkspace";
import { useTaskQuickCreate } from "../../composables/useTaskQuickCreate";
import { useTaskScriptWorkspace } from "../../composables/useTaskScriptWorkspace";
import { useTasksPageCommands } from "../../composables/useTasksPageCommands";
import { useTasksPageLifecycle } from "../../composables/useTasksPageLifecycle";
import { useTasksPageState } from "../../composables/useTasksPageState";
import type {
  TaskFocusableRef,
  TaskLogViewerRef,
} from "../../composables/taskPageTypes";

import TaskDesktopLayout from "../../components/tasks/TaskDesktopLayout.vue";
import TaskMobileLogDrawer from "../../components/tasks/TaskMobileLogDrawer.vue";
import TaskMobileWorkspace from "../../components/tasks/TaskMobileWorkspace.vue";
import TaskPageDialogs from "../../components/tasks/TaskPageDialogs.vue";

const taskStore = useTaskStore();
const appStore = useAppStore();
const workspaceStore = useWorkspaceStore();
const workspaceTaskCommands = useWorkspaceTaskCommandStore();
const refreshMobileTasks = () => taskStore.fetchTasks(true);
const handledWorkspaceCommandId = ref(workspaceTaskCommands.command?.id ?? 0);

type TaskDesktopLayoutExpose = TaskLogViewerRef & TaskFocusableRef;

const isTaskDesktopLayoutExpose = (
  instance: unknown,
): instance is TaskDesktopLayoutExpose => {
  return (
    typeof instance === "object" &&
    instance !== null &&
    "focus" in instance
  );
};

const {
  actionSheetVisible,
  activeDetailTab,
  createActionSheetVisible,
  currentActiveTask,
  currentTask,
  environments,
  fetchEnvironments,
  filteredTasks,
  searchQuery,
  selectedTaskId,
  selectTask,
  selectionMode,
  sortedTasks,
  statusFilter,
  openActionSheet,
} = useTasksPageState(taskStore);

// --- 2. Action Composables ---
const {
  selectedIds,
  selectedTasks,
  wizardVisible,
  editingTask,
  logVisible,
  currentLogTask,
  shareVisible,
  tasksToShare,
  scriptEditorVisible,
  scriptEditorContent,
  scriptEditorLoading,
  currentScriptTask,
  isFileMode,
  variableEditorVisible,
  currentTaskForVariables,
  handleSelectAll,
  clearAllSelection,
  handleMobileSelection,
  handleToggleEnable,
  openCreate,
  openEdit,
  handleWizardSuccess,
  openLogs,
  handleEditScript,
  saveScript,
  openVariableEditor,
  handleVariableEditSuccess,
  openShare,
  handleDelete,
  handleBulkDelete,
  handleBulkRun,
  handleBulkPause,
  handleBulkResume,
  handleBulkStop,
  handleBulkEnable,
  handleBulkDisable,
  handleBulkPin,
  handleBulkUnpin,
  handleBulkShare,
} = useTaskActions();

const {
  cronEditVisible,
  enableRandom,
  randomConfig,
  cronInput,
  cronSaving,
  openCronEditor,
  saveCron,
} = useTaskCronEditor(taskStore.refreshTasks);

const {
  quickCreateVisible,
  quickCreating,
  quickCreateForm,
  openQuickCreate,
  handleQuickCreate,
} = useTaskQuickCreate(taskStore.refreshTasks);

const {
  dialogEditorLanguage,
  dialogEditorOptions,
  editorWordWrap,
  handleScriptEditorMount,
  loadScriptContent,
  onScriptDrawerOpened,
  resetScriptEditorReady,
  saveScriptContent,
  scriptContent,
  scriptEditorInstance,
  scriptEditorOptions,
  scriptEditorReady,
  scriptLanguage,
  scriptLoading,
  scriptSaving,
  closeScriptEditor,
  toggleEditorWordWrap,
} = useTaskScriptWorkspace({
  currentScriptTask,
  currentTask,
  isFileMode,
  saveMobileScript: saveScript,
  scriptEditorContent,
  scriptEditorVisible,
});

const {
  logViewRef,
  logQrCodeData,
  expandLogQr,
  logProgressValue,
  showLogProgress,
  showSearch,
  logSearchQuery,
  logSearchInputRef,
  historyTimeline,
  selectedHistoryRunId,
  timelineLoading,
  timelinePage,
  timelineHasMore,
  historyLogVisible,
  historyLogContent,
  historyLogRunId,
  historyLogLoading,
  closeLogStream,
  connectLogStream,
  downloadLogs,
  fetchRunTimeline,
  handleAction,
  handleHistoryLogView,
  handleLogUiEvent,
  loadMoreTimeline,
  resetLogArtifacts,
  selectTimelineRun,
  toggleHeaderSearch,
} = useTaskLogWorkspace({
  activeDetailTab,
  currentLogTask,
  currentTask,
});

const setDesktopWorkspaceRef = (instance: unknown) => {
  if (!isTaskDesktopLayoutExpose(instance)) {
    logViewRef.value = null;
    logSearchInputRef.value = null;
    return;
  }

  logViewRef.value = instance;
  logSearchInputRef.value = instance;
};

const openCurrentTaskLogWindow = () => {
  if (!currentTask.value) return;
  workspaceStore.openTaskLogWindow(currentTask.value, selectedHistoryRunId.value);
};

const handleHistoryLogViewRequest = (_logPath: string, runId: number) => {
  if (!currentTask.value || appStore.isMobile) {
    handleHistoryLogView(_logPath, runId);
    return;
  }

  workspaceStore.openTaskLogWindow(currentTask.value, runId);
};

const {
  handleBulkCommand,
  handleBulkMoreCommand,
  handleCancelSelection,
  handleContextCommand,
  handleEnterSelection,
  handleMobileActionCommand,
  handleMobileScriptEditorCommand,
  handleMoreCommand,
  handleTaskDisable,
  handleTaskEnable,
} = useTasksPageCommands({
  activeDetailTab,
  clearAllSelection,
  currentActiveTask,
  currentTask,
  downloadLogs,
  handleBulkDelete,
  handleBulkDisable,
  handleBulkEnable,
  handleBulkPin,
  handleBulkResume,
  handleBulkShare,
  handleBulkStop,
  handleBulkUnpin,
  handleDelete,
  handleEditScript,
  handleMobileSelection,
  handleToggleEnable,
  logViewRef,
  openCronEditor,
  openEdit,
  openLogs,
  openShare,
  openVariableEditor,
  scriptEditorInstance,
  selectTask,
  selectionMode,
  taskStore,
});

const findWorkspaceCommandTask = async (taskId?: number) => {
  if (!taskId) return null;

  let task = taskStore.tasks.find((item) => item.id === taskId);
  if (task) return task;

  await taskStore.refreshTasks(true);
  task = taskStore.tasks.find((item) => item.id === taskId);
  return task ?? null;
};

const handleWorkspaceTaskCommand = async (
  command: WorkspaceTaskCommand | null,
) => {
  if (!command || command.id <= handledWorkspaceCommandId.value) return;

  handledWorkspaceCommandId.value = command.id;
  try {
    if (appStore.isMobile) return;

    if (command.type === "create") {
      if (environments.value.length === 0) {
        await fetchEnvironments();
      }
      openCreate(environments.value);
      return;
    }

    if (command.type === "quick_create") {
      openQuickCreate();
      return;
    }

    if (command.type === "create_upload") {
      if (!command.uploadedFile) {
        ElMessage.warning("未找到上传文件");
        return;
      }

      if (environments.value.length === 0) {
        await fetchEnvironments();
      }

      openCreate(environments.value, {
        scriptSourceMode: "upload",
        uploadedFile: command.uploadedFile,
      });
      return;
    }

    const task = await findWorkspaceCommandTask(command.taskId);
    if (!task) {
      ElMessage.warning("任务不存在或已删除");
      return;
    }

    if (command.type === "edit") openEdit(task);
    else if (command.type === "cron") openCronEditor(task);
    else if (command.type === "select") {
      selectTask(task);
      activeDetailTab.value = "log";
    } else if (command.type === "script") {
      selectTask(task);
      activeDetailTab.value = "script";
    } else if (command.type === "variables") {
      selectTask(task);
      activeDetailTab.value = "var";
    }
  } finally {
    workspaceTaskCommands.clear?.(command.id);
  }
};

watch(
  () => workspaceTaskCommands.command,
  (command) => {
    void handleWorkspaceTaskCommand(command);
  },
  { immediate: true },
);

useTasksPageLifecycle({
  activeDetailTab,
  closeLogStream,
  connectLogStream,
  currentTask,
  environments,
  fetchEnvironments,
  fetchRunTimeline,
  loadScriptContent,
  logSearchQuery,
  logViewRef,
  logVisible,
  openCreate,
  openQuickCreate,
  resetLogArtifacts,
  searchQuery,
  selectedHistoryRunId,
  selectedIds,
  selectedTaskId,
  selectionMode,
  statusFilter,
  taskStore,
});

</script>

<style>
.no-scrollbar::-webkit-scrollbar {
  display: none;
}

.modern-dropdown .el-dropdown-menu__item {
  @apply mx-1 min-h-8 rounded-md px-3 py-1.5 text-[11px] font-semibold;
}

.modern-dialog .el-dialog__header {
  @apply border-b border-light px-6 py-4 mr-0;
}

@media (max-width: 768px) {
  .el-drawer__header {
    margin-bottom: 0 !important;
    padding-bottom: 16px !important;
  }
}

.action-sheet-drawer .el-drawer__body {
  padding: 0 !important;
  overflow: visible !important;
  background: transparent !important;
}
</style>
