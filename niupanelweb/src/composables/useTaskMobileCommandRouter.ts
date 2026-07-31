import { type Ref } from "vue";
import { ElMessage } from "element-plus";
import { useTaskStore } from "../stores/tasks";
import type {
  TaskMobileActionCommand,
  TaskMobileScriptEditorCommand,
  TaskScriptEditorRef,
} from "./taskPageTypes";
import type { Task } from "@/types";

type UseTaskMobileCommandRouterOptions = {
  currentActiveTask: Ref<Task | null>;
  handleDelete: (id: number) => void;
  handleEditScript: (task: Task) => unknown;
  handleToggleEnable: (task: Task, enabled: boolean) => void;
  openCronEditor: (task: Task) => void;
  openEdit: (task: Task) => void;
  openLogs: (task: Task) => void;
  openShare: (task: Task) => void;
  openVariableEditor: (taskId: number) => void;
  scriptEditorInstance: Ref<TaskScriptEditorRef | null>;
  taskStore: ReturnType<typeof useTaskStore>;
};

export function useTaskMobileCommandRouter({
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
}: UseTaskMobileCommandRouterOptions) {
  const handleMobileScriptEditorCommand = (
    command: TaskMobileScriptEditorCommand,
  ) => {
    const commandMap: Record<TaskMobileScriptEditorCommand, string> = {
      undo: "undo",
      redo: "redo",
      format: "editor.action.formatDocument",
    };
    scriptEditorInstance.value?.trigger?.("keyboard", commandMap[command]);
  };

  const handleTaskEnable = (id: number) => {
    const task = taskStore.tasks.find((item) => item.id === id);
    if (task) handleToggleEnable(task, true);
  };

  const handleTaskDisable = (id: number) => {
    const task = taskStore.tasks.find((item) => item.id === id);
    if (task) handleToggleEnable(task, false);
  };

  const handleMobileActionCommand = (command: TaskMobileActionCommand) => {
    const task = currentActiveTask.value;
    if (!task) return;

    switch (command) {
      case "logs":
        openLogs(task);
        break;
      case "edit":
        openEdit(task);
        break;
      case "script":
        handleEditScript(task);
        break;
      case "cron":
        openCronEditor(task);
        break;
      case "variables":
        openVariableEditor(task.id);
        break;
      case "share":
        openShare(task);
        break;
      case "pin":
        taskStore.pinTask(task.id);
        break;
      case "unpin":
        taskStore.unpinTask(task.id);
        break;
      case "copy":
        ElMessage.info("Not implemented");
        break;
      case "delete":
        handleDelete(task.id);
        break;
    }
  };

  return {
    handleMobileActionCommand,
    handleMobileScriptEditorCommand,
    handleTaskDisable,
    handleTaskEnable,
  };
}
