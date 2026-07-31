import { computed, ref } from "vue";
import { useAppStore } from "../stores/app";
import { useTaskStore } from "../stores/tasks";
import { createTaskDraft } from "./taskCreateDefaults";
import { useTaskDialogBackActions } from "./useTaskDialogBackActions";
import type { Env, Task } from "@/types";
import type { TaskWizardInitialData } from "./useTaskWizardData";

type UseTaskDialogStateOptions = {
  appStore: ReturnType<typeof useAppStore>;
  taskStore: ReturnType<typeof useTaskStore>;
};

export function useTaskDialogState({
  appStore,
  taskStore,
}: UseTaskDialogStateOptions) {
  const wizardVisible = ref(false);
  const editingTask = ref<TaskWizardInitialData | null>(null);
  const logVisible = ref(false);
  const currentLogTaskId = ref<number | null>(null);
  const shareVisible = ref(false);
  const tasksToShare = ref<Task[]>([]);
  const variableEditorVisible = ref(false);
  const currentTaskForVariables = ref<number | null>(null);

  const currentLogTask = computed(() => {
    return taskStore.tasks.find((task) => task.id === currentLogTaskId.value);
  });

  useTaskDialogBackActions({
    appStore,
    logVisible,
    shareVisible,
    variableEditorVisible,
    wizardVisible,
  });

  const openCreate = (
    environments: Env[] = [],
    initialData: TaskWizardInitialData = {},
  ) => {
    editingTask.value = {
      ...createTaskDraft(environments),
      ...initialData,
    };
    wizardVisible.value = true;
  };

  const openEdit = (task: Task) => {
    editingTask.value = { ...task };
    wizardVisible.value = true;
  };

  const handleWizardSuccess = () => {
    wizardVisible.value = false;
    taskStore.refreshTasks(true);
  };

  const openLogs = (task: Task) => {
    currentLogTaskId.value = task.id;
    logVisible.value = true;
  };

  const openVariableEditor = (taskId: number) => {
    currentTaskForVariables.value = taskId;
    variableEditorVisible.value = true;
  };

  const handleVariableEditSuccess = () => {
    variableEditorVisible.value = false;
    taskStore.refreshTasks(true);
  };

  const openShare = (task: Task) => {
    tasksToShare.value = [task];
    shareVisible.value = true;
  };

  return {
    currentLogTask,
    currentTaskForVariables,
    editingTask,
    handleVariableEditSuccess,
    handleWizardSuccess,
    logVisible,
    openCreate,
    openEdit,
    openLogs,
    openShare,
    openVariableEditor,
    shareVisible,
    tasksToShare,
    variableEditorVisible,
    wizardVisible,
  };
}
