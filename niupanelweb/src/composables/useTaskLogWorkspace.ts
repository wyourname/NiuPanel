import { ref, type ComputedRef, type Ref } from "vue";
import { useAppStore } from "../stores/app";
import { useTaskStore } from "../stores/tasks";
import { useTaskLogActions } from "./useTaskLogActions";
import { useTaskLogDownload } from "./useTaskLogDownload";
import { useTaskLogStream } from "./useTaskLogStream";
import { useTaskLogUiState } from "./useTaskLogUiState";
import { useTaskRunTimeline } from "./useTaskRunTimeline";
import type { TaskDetailTab, TaskLogViewerRef } from "./taskPageTypes";
import type { Task } from "@/types";

type UseTaskLogWorkspaceOptions = {
  activeDetailTab: Ref<TaskDetailTab>;
  currentLogTask: ComputedRef<Task | undefined>;
  currentTask: ComputedRef<Task | undefined>;
};

export function useTaskLogWorkspace({
  activeDetailTab,
  currentLogTask,
  currentTask,
}: UseTaskLogWorkspaceOptions) {
  const appStore = useAppStore();
  const taskStore = useTaskStore();

  const logViewRef = ref<TaskLogViewerRef | null>(null);

  const activeLogTask = () =>
    appStore.isMobile ? currentLogTask.value : currentTask.value;

  const {
    expandLogQr,
    handleLogUiEvent,
    logProgressValue,
    logQrCodeData,
    logSearchInputRef,
    logSearchQuery,
    resetLogArtifacts,
    showLogProgress,
    showSearch,
    toggleHeaderSearch,
  } = useTaskLogUiState({ activeDetailTab });

  const {
    fetchRunTimeline,
    handleHistoryLogView,
    historyLogContent,
    historyLogLoading,
    historyLogRunId,
    historyLogVisible,
    historyTimeline,
    loadMoreTimeline,
    selectTimelineRun: setTimelineRun,
    selectedHistoryRunId,
    timelineHasMore,
    timelineLoading,
    timelinePage,
  } = useTaskRunTimeline({
    activeLogTask,
    currentTask,
  });

  const {
    closeLogStream,
    connectLogStream,
  } = useTaskLogStream({
    activeLogTask,
    logViewRef,
    selectedHistoryRunId,
  });

  const selectTimelineRun = (runId: number | null) => {
    setTimelineRun(runId);
    connectLogStream();
  };

  const { downloadLogs } = useTaskLogDownload({
    activeLogTask,
    selectedHistoryRunId,
  });

  const { handleAction } = useTaskLogActions({
    activeDetailTab,
    activeLogTask,
    appStore,
    logViewRef,
    reconnectLogStream: connectLogStream,
    resetLogArtifacts,
    taskStore,
  });

  return {
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
  };
}
