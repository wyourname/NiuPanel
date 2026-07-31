import { defineStore } from "pinia";
import { ref } from "vue";
import { ElMessage } from "element-plus";
import * as taskApi from "../api/tasks";
import type { Task } from "@/types";
import {
  applyTaskStatusUpdate,
  isFinishedSystemJob,
  parseTaskStatusPayload,
} from "./taskStatusEvents";
import { createTaskOperations } from "./taskOperations";

export type { Task };

export const useTaskStore = defineStore("tasks", () => {
  // State
  const tasks = ref<Task[]>([]);
  const total = ref(0);
  const loading = ref(false);
  const page = ref(1);
  const pageSize = ref(100);
  const noMore = ref(false);

  // Current filter state to persist across paginated requests
  const currentQuery = ref("");
  const currentStatus = ref("");

  // Actions
  // Initial load or full reset (e.g. Pull-to-refresh)
  const fetchTasks = async (silent = false, q?: string, status?: string) => {
    if (!silent) loading.value = true;
    page.value = 1;
    noMore.value = false;

    // Save current filters
    currentQuery.value = q || "";
    currentStatus.value = status || "";

    try {
      // If a filter is active, omit pagination to get all matches
      const isFiltered = q || (status && status !== "all");
      const sendPage = isFiltered ? undefined : 1;
      const sendPageSize = isFiltered ? undefined : pageSize.value;

      const res = await taskApi.getTasks(sendPage, sendPageSize, q, status);
      if (res.data && res.data.items) {
        tasks.value = res.data.items;
        total.value = res.data.total;
        // If we didn't paginate, we have everything
        if (isFiltered || tasks.value.length >= total.value) noMore.value = true;
      } else {
        tasks.value = [];
        total.value = 0;
        noMore.value = true;
      }
    } catch (error) {
      if (!silent) ElMessage.error("加载任务失败");
    } finally {
      loading.value = false;
    }
  };

  // Load more (Infinite Scroll)
  const loadMoreTasks = async () => {
    if (loading.value || noMore.value) return;
    loading.value = true;

    try {
      const nextPage = page.value + 1;
      const res = await taskApi.getTasks(nextPage, pageSize.value, currentQuery.value, currentStatus.value);

      if (res.data && res.data.items && res.data.items.length > 0) {
        tasks.value.push(...res.data.items);
        page.value = nextPage;
        total.value = res.data.total;
        if (tasks.value.length >= total.value) noMore.value = true;
      } else {
        noMore.value = true;
      }
    } catch (error) {
      // Silent error on load more
    } finally {
      loading.value = false;
    }
  };

  // Refresh current list without resetting scroll position (e.g. status update)
  const refreshTasks = async (silent = false) => {
    if (!silent) loading.value = true;
    try {
      const currentCount = Math.max(tasks.value.length, pageSize.value);
      const res = await taskApi.getTasks(1, currentCount);

      if (res.data && res.data.items) {
        tasks.value = res.data.items;
        total.value = res.data.total;
        page.value = Math.ceil(tasks.value.length / pageSize.value) || 1;
        noMore.value = tasks.value.length >= total.value;
      }
    } catch (e) {
      if (!silent) ElMessage.error("刷新失败");
    } finally {
      loading.value = false;
    }
  };

  const {
    runTask,
    stopTask,
    pauseTask,
    resumeTask,
    deleteTask,
    toggleEnable,
    pinTask,
    unpinTask,
    batchRun,
    batchStop,
    batchPause,
    batchResume,
    batchEnable,
    batchDisable,
    batchDelete,
    batchPin,
    batchUnpin,
  } = createTaskOperations({ loading, refreshTasks });

  // SSE Connection
  let statusEventSource: EventSource | null = null;
  let statusStreamConsumers = 0;

  const init = async () => {
    void fetchTasks();
    statusStreamConsumers += 1;
    startStatusStream();
  };

  const startStatusStream = () => {
    // Setup Real-time Status Updates
    if (statusEventSource) return;

    try {
      statusEventSource = taskApi.streamTaskStatus();

      statusEventSource.onmessage = (event) => {
        try {
          const update = parseTaskStatusPayload(event.data);
          if (!update) return;

          if (isFinishedSystemJob(update)) {
            window.dispatchEvent(
              new CustomEvent("niu:job-finished", { detail: update.raw }),
            );
          }

          applyTaskStatusUpdate(tasks.value, update);
        } catch (e) {
          console.error("Failed to parse status event", e);
        }
      };

      statusEventSource.onerror = (err) => {
        console.warn("SSE connection error, verify backend is running", err);
        // Browser usually auto-reconnects, but we can handle close here if needed
      };
    } catch (e) {
      console.error("Failed to init SSE", e);
    }
  };

  const stopStatusStream = () => {
    statusStreamConsumers = Math.max(0, statusStreamConsumers - 1);
    if (statusStreamConsumers > 0) return;

    if (statusEventSource) {
      statusEventSource.close();
      statusEventSource = null;
    }
  };

  return {
    tasks,
    total,
    loading,
    page,
    pageSize,
    noMore,
    init,
    fetchTasks,
    loadMoreTasks,
    refreshTasks,
    stopStatusStream,
    runTask,
    stopTask,
    pauseTask,
    resumeTask,
    deleteTask,
    toggleEnable,
    pinTask,
    unpinTask,
    batchRun,
    batchStop,
    batchPause,
    batchResume,
    batchEnable,
    batchDisable,
    batchDelete,
    batchPin,
    batchUnpin,
  };
});
