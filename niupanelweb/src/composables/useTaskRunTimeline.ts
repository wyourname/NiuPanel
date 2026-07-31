import { ref, type ComputedRef } from "vue";
import * as taskApi from "../api/tasks";
import type { TaskRunTimelineItem } from "./taskPageTypes";
import type { Task } from "@/types";

type UseTaskRunTimelineOptions = {
  activeLogTask: () => Task | undefined;
  currentTask: ComputedRef<Task | undefined>;
};

export function useTaskRunTimeline({
  activeLogTask,
  currentTask,
}: UseTaskRunTimelineOptions) {
  const historyTimeline = ref<TaskRunTimelineItem[]>([]);
  const selectedHistoryRunId = ref<number | null>(null);
  const timelineLoading = ref(false);
  const timelinePage = ref(1);
  const timelineHasMore = ref(true);

  const historyLogVisible = ref(false);
  const historyLogContent = ref("");
  const historyLogRunId = ref<number | null>(null);
  const historyLogLoading = ref(false);

  const fetchRunTimeline = async (id?: number, reload = true) => {
    if (!id) return;
    if (reload) {
      timelinePage.value = 1;
      historyTimeline.value = [];
      timelineHasMore.value = true;
    }
    if (!timelineHasMore.value || timelineLoading.value) return;

    timelineLoading.value = true;
    try {
      const res = await taskApi.getTaskHistory(id, timelinePage.value, 20);
      const items = res.data?.items || [];
      if (reload) {
        historyTimeline.value = items;
      } else {
        historyTimeline.value.push(...items);
      }
      timelineHasMore.value = items.length === 20;
      timelinePage.value++;
    } catch (error) {
      console.error("Failed to fetch timeline:", error);
    } finally {
      timelineLoading.value = false;
    }
  };

  const loadMoreTimeline = () => {
    if (timelineHasMore.value && !timelineLoading.value) {
      fetchRunTimeline(activeLogTask()?.id, false);
    }
  };

  const selectTimelineRun = (runId: number | null) => {
    selectedHistoryRunId.value = runId;
  };

  const handleHistoryLogView = async (_logPath: string, runId: number) => {
    if (!currentTask.value) return;
    historyLogVisible.value = true;
    historyLogRunId.value = runId;
    historyLogContent.value = "";
    historyLogLoading.value = true;
    try {
      const res = await taskApi.getTaskRunLog(
        currentTask.value.id,
        runId,
        null,
        null,
      );
      historyLogContent.value = res.data?.content || "（日志为空）";
    } catch (error) {
      historyLogContent.value = `加载失败: ${
        error instanceof Error ? error.message : String(error)
      }`;
    } finally {
      historyLogLoading.value = false;
    }
  };

  return {
    fetchRunTimeline,
    handleHistoryLogView,
    historyLogContent,
    historyLogLoading,
    historyLogRunId,
    historyLogVisible,
    historyTimeline,
    loadMoreTimeline,
    selectTimelineRun,
    selectedHistoryRunId,
    timelineHasMore,
    timelineLoading,
    timelinePage,
  };
}
