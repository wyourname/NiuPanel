import { type ComputedRef, type Ref } from "vue";
import type {
  TaskDetailMoreCommand,
  TaskDetailTab,
  TaskLogViewerRef,
} from "./taskPageTypes";
import type { Task } from "@/types";

type UseTaskDetailCommandRouterOptions = {
  activeDetailTab: Ref<TaskDetailTab>;
  currentTask: ComputedRef<Task | undefined>;
  downloadLogs: () => void;
  handleDelete: (id: number) => void;
  logViewRef: Ref<TaskLogViewerRef | null>;
  openEdit: (task: Task) => void;
  openShare: (task: Task) => void;
};

export function useTaskDetailCommandRouter({
  activeDetailTab,
  currentTask,
  downloadLogs,
  handleDelete,
  logViewRef,
  openEdit,
  openShare,
}: UseTaskDetailCommandRouterOptions) {
  const handleMoreCommand = (command: TaskDetailMoreCommand) => {
    if (!currentTask.value) return;

    switch (command) {
      case "edit_config":
        openEdit(currentTask.value);
        break;
      case "edit_script":
        activeDetailTab.value = "script";
        break;
      case "share":
        openShare(currentTask.value);
        break;
      case "download_log":
        downloadLogs();
        break;
      case "clear_screen":
        logViewRef.value?.clear?.();
        break;
      case "delete_task":
        handleDelete(currentTask.value.id);
        break;
    }
  };

  return {
    handleMoreCommand,
  };
}
