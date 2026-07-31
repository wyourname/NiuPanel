import { type ComputedRef, type Ref } from "vue";
import { useTaskStore } from "../stores/tasks";
import { useTaskBulkCommandRouter } from "./useTaskBulkCommandRouter";
import { useTaskContextCommandRouter } from "./useTaskContextCommandRouter";
import { useTaskDetailCommandRouter } from "./useTaskDetailCommandRouter";
import { useTaskMobileCommandRouter } from "./useTaskMobileCommandRouter";
import type {
  TaskDetailTab,
  TaskLogViewerRef,
  TaskScriptEditorRef,
} from "./taskPageTypes";
import type { Task } from "@/types";

type UseTasksPageCommandsOptions = {
  activeDetailTab: Ref<TaskDetailTab>;
  clearAllSelection: () => void;
  currentActiveTask: Ref<Task | null>;
  currentTask: ComputedRef<Task | undefined>;
  downloadLogs: () => void;
  handleBulkDelete: () => void;
  handleBulkDisable: () => unknown;
  handleBulkEnable: () => unknown;
  handleBulkPin: () => unknown;
  handleBulkResume: () => unknown;
  handleBulkShare: () => unknown;
  handleBulkStop: () => unknown;
  handleBulkUnpin: () => unknown;
  handleDelete: (id: number) => void;
  handleEditScript: (task: Task) => unknown;
  handleMobileSelection: (task: Task, selected: boolean) => void;
  handleToggleEnable: (task: Task, enabled: boolean) => void;
  logViewRef: Ref<TaskLogViewerRef | null>;
  openCronEditor: (task: Task) => void;
  openEdit: (task: Task) => void;
  openLogs: (task: Task) => void;
  openShare: (task: Task) => void;
  openVariableEditor: (taskId: number) => void;
  scriptEditorInstance: Ref<TaskScriptEditorRef | null>;
  selectTask: (task: Task) => void;
  selectionMode: Ref<boolean>;
  taskStore: ReturnType<typeof useTaskStore>;
};

export function useTasksPageCommands({
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
}: UseTasksPageCommandsOptions) {
  const handleEnterSelection = (task: Task) => {
    selectionMode.value = true;
    handleMobileSelection(task, true);
  };

  const handleCancelSelection = () => {
    selectionMode.value = false;
    clearAllSelection();
  };

  const {
    handleMoreCommand,
  } = useTaskDetailCommandRouter({
    activeDetailTab,
    currentTask,
    downloadLogs,
    handleDelete,
    logViewRef,
    openEdit,
    openShare,
  });

  const {
    handleBulkCommand,
    handleBulkMoreCommand,
  } = useTaskBulkCommandRouter({
    handleBulkDelete,
    handleBulkDisable,
    handleBulkEnable,
    handleBulkPin,
    handleBulkResume,
    handleBulkShare,
    handleBulkStop,
    handleBulkUnpin,
  });

  const {
    handleMobileActionCommand,
    handleMobileScriptEditorCommand,
    handleTaskDisable,
    handleTaskEnable,
  } = useTaskMobileCommandRouter({
    currentActiveTask,
    handleDelete,
    handleEditScript,
    handleToggleEnable,
    openCronEditor,
    openEdit,
    openLogs,
    openShare,
    openVariableEditor,
    scriptEditorInstance,
    taskStore,
  });

  const {
    handleContextCommand,
  } = useTaskContextCommandRouter({
    activeDetailTab,
    handleDelete,
    handleEnterSelection,
    openCronEditor,
    openEdit,
    openShare,
    selectTask,
    selectionMode,
    taskStore,
  });

  return {
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
  };
}
