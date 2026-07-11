import { onMounted, onUnmounted, type ComputedRef, type Ref } from "vue";
import { useTaskStore } from "../stores/tasks";
import type { TaskDetailTab, TaskLogViewerRef } from "./taskPageTypes";
import { useTaskDetailLifecycle } from "./useTaskDetailLifecycle";
import { useTaskListLifecycle } from "./useTaskListLifecycle";
import type { Env, Task } from "@/types";

type UseTasksPageLifecycleOptions = {
  activeDetailTab: Ref<TaskDetailTab>;
  closeLogStream: () => void;
  connectLogStream: () => void;
  currentTask: ComputedRef<Task | undefined>;
  environments: Ref<Env[]>;
  fetchEnvironments: () => Promise<void>;
  fetchRunTimeline: (id?: number, reload?: boolean) => unknown;
  loadScriptContent: () => unknown;
  logSearchQuery: Ref<string>;
  logViewRef: Ref<TaskLogViewerRef | null>;
  logVisible: Ref<boolean>;
  openCreate: (environments?: Env[]) => void;
  openQuickCreate: () => void;
  resetLogArtifacts: () => void;
  searchQuery: Ref<string>;
  selectedHistoryRunId: Ref<number | null>;
  selectedIds: Ref<number[]>;
  selectedTaskId: Ref<number | null>;
  selectionMode: Ref<boolean>;
  statusFilter: Ref<string>;
  taskStore: ReturnType<typeof useTaskStore>;
};

export function useTasksPageLifecycle({
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
}: UseTasksPageLifecycleOptions) {
  const handleOpenCreateEvent = () => {
    openCreate(environments.value);
  };

  const handleOpenQuickImportEvent = () => {
    openQuickCreate();
  };

  useTaskListLifecycle({
    searchQuery,
    selectedIds,
    selectionMode,
    statusFilter,
    taskStore,
  });

  useTaskDetailLifecycle({
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
  });

  onMounted(() => {
    taskStore.init();
    fetchEnvironments();
    window.addEventListener("niu:open-create", handleOpenCreateEvent);
    window.addEventListener(
      "niu:open-quick-import",
      handleOpenQuickImportEvent,
    );
  });

  onUnmounted(() => {
    closeLogStream();
    taskStore.stopStatusStream();
    window.removeEventListener("niu:open-create", handleOpenCreateEvent);
    window.removeEventListener(
      "niu:open-quick-import",
      handleOpenQuickImportEvent,
    );
  });
}
