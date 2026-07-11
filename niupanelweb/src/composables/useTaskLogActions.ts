import { nextTick, type Ref } from "vue";
import * as taskApi from "../api/tasks";
import { useAppStore } from "../stores/app";
import { useTaskStore } from "../stores/tasks";
import type { TaskDetailTab, TaskLogViewerRef } from "./taskPageTypes";
import type { Task } from "@/types";

type UseTaskLogActionsOptions = {
  activeDetailTab: Ref<TaskDetailTab>;
  activeLogTask: () => Task | undefined;
  appStore: ReturnType<typeof useAppStore>;
  logViewRef: Ref<TaskLogViewerRef | null>;
  reconnectLogStream: () => void;
  resetLogArtifacts: () => void;
  taskStore: ReturnType<typeof useTaskStore>;
};

export function useTaskLogActions({
  activeDetailTab,
  activeLogTask,
  appStore,
  logViewRef,
  reconnectLogStream,
  resetLogArtifacts,
  taskStore,
}: UseTaskLogActionsOptions) {
  const handleAction = async (action: string) => {
    const task = activeLogTask();
    const id = task?.id;
    if (!id) return;

    if (action === "run") {
      logViewRef.value?.clear?.();
      resetLogArtifacts();

      await taskStore.runTask(id);

      if (!appStore.isMobile) activeDetailTab.value = "log";
      nextTick(() => setTimeout(reconnectLogStream, 150));
    } else if (action === "stop") {
      await taskStore.stopTask(id);
    } else if (action === "pause") {
      await taskApi.pauseTasks([id]);
    } else if (action === "resume") {
      await taskApi.resumeTasks([id]);
    }

    taskStore.refreshTasks(true);
  };

  return {
    handleAction,
  };
}
