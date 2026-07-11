import { nextTick, watch, type ComputedRef, type Ref } from "vue";
import { useAppStore } from "../stores/app";
import type { TaskDetailTab, TaskLogViewerRef } from "./taskPageTypes";
import type { Task } from "@/types";

type UseTaskDetailLifecycleOptions = {
  activeDetailTab: Ref<TaskDetailTab>;
  closeLogStream: () => void;
  connectLogStream: () => void;
  currentTask: ComputedRef<Task | undefined>;
  fetchRunTimeline: (id?: number, reload?: boolean) => unknown;
  loadScriptContent: () => unknown;
  logSearchQuery: Ref<string>;
  logViewRef: Ref<TaskLogViewerRef | null>;
  logVisible: Ref<boolean>;
  resetLogArtifacts: () => void;
  selectedHistoryRunId: Ref<number | null>;
  selectedTaskId: Ref<number | null>;
};

export function useTaskDetailLifecycle({
  activeDetailTab,
  closeLogStream,
  connectLogStream,
  currentTask,
  fetchRunTimeline,
  loadScriptContent,
  logSearchQuery,
  logViewRef,
  logVisible,
  resetLogArtifacts,
  selectedHistoryRunId,
  selectedTaskId,
}: UseTaskDetailLifecycleOptions) {
  const appStore = useAppStore();

  const loadLogWorkspace = (taskId?: number) => {
    selectedHistoryRunId.value = null;
    fetchRunTimeline(taskId);
    nextTick(connectLogStream);
  };

  watch(logSearchQuery, (query) => {
    logViewRef.value?.setSearch?.(query, false);
  });

  watch(
    () => currentTask.value?.status,
    (newStatus, oldStatus) => {
      if (
        newStatus === "Running" &&
        oldStatus !== "Running" &&
        activeDetailTab.value === "log" &&
        !appStore.isMobile
      ) {
        nextTick(connectLogStream);
      }
    },
  );

  watch(activeDetailTab, (tab) => {
    if (tab === "script") {
      loadScriptContent();
      return;
    }

    if (tab === "log" && !appStore.isMobile) {
      loadLogWorkspace(currentTask.value?.id);
    }
  });

  watch(logVisible, (visible) => {
    if (visible) {
      nextTick(connectLogStream);
    } else {
      closeLogStream();
    }
  });

  watch(selectedTaskId, (newId) => {
    if (!newId) return;

    resetLogArtifacts();
    if (activeDetailTab.value === "log") {
      loadLogWorkspace(newId);
    } else if (activeDetailTab.value === "script") {
      loadScriptContent();
    }
  });
}
