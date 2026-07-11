<template>
  <div class="relative flex h-full min-h-0 flex-col overflow-hidden bg-base">
    <div class="relative min-h-0 flex-1 overflow-hidden">
      <LogViewer
        ref="logViewerRef"
        :is-mobile="isMobile"
        class="min-h-0 flex-1"
      />

      <aside
        v-if="historyPanelVisible"
        class="absolute inset-2 z-20 flex flex-col overflow-hidden rounded-lg border border-light bg-card shadow-sm sm:left-auto sm:w-[310px]"
      >
        <div class="grid h-9 shrink-0 grid-cols-[minmax(0,1fr)_auto] items-center border-b border-slate-900/8 px-3 dark:border-white/8">
          <div class="text-[12px] font-black text-default">历史记录</div>
          <div class="flex items-center gap-1">
            <button
              type="button"
              class="h-7 w-7 cursor-pointer rounded-md text-muted flex-center transition-colors hover:bg-subtle hover:text-default"
              title="刷新历史"
              aria-label="刷新历史"
              @click="fetchHistory(true)"
            >
              <div class="i-ep-refresh text-[13px]" :class="{ 'animate-spin': historyLoading }"></div>
            </button>
            <button
              type="button"
              class="h-7 w-7 cursor-pointer rounded-md text-muted flex-center transition-colors hover:bg-subtle hover:text-default"
              title="关闭"
              aria-label="关闭历史记录"
              @click="historyPanelVisible = false"
            >
              <div class="i-ep-close text-[13px]"></div>
            </button>
          </div>
        </div>

        <div class="min-h-0 flex-1 overflow-y-auto p-2 no-scrollbar">
          <button
            type="button"
            class="mb-1 flex w-full cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-left transition-colors"
            :class="historyRowClass(!activeRunId)"
            @click="selectLiveLog"
          >
            <div
              class="h-2.5 w-2.5 shrink-0 rounded-full"
              :class="liveStatusDotClass"
            ></div>
            <div class="min-w-0 flex-1">
              <div class="truncate text-[12px] font-black">
                {{ liveModeLabel }}
              </div>
              <div
                class="truncate text-[10px] font-bold"
                :class="historyMetaClass(!activeRunId)"
              >
                {{ getStatusLabel(currentTask.status) }}
              </div>
            </div>
          </button>

          <div v-loading="historyLoading && historyPage === 1" class="space-y-1.5">
            <button
              v-for="run in historyRuns"
              :key="run.id"
              type="button"
              class="flex w-full cursor-pointer items-start gap-2 rounded-md px-2.5 py-2 text-left transition-colors"
              :class="historyRowClass(activeRunId === run.id)"
              @click="selectHistoryRun(run.id)"
            >
              <div
                class="mt-1 h-2.5 w-2.5 shrink-0 rounded-full"
                :class="runStatusDotClass(run.status)"
              ></div>
              <div class="min-w-0 flex-1">
                <div class="flex items-center justify-between gap-2">
                  <span class="truncate text-[12px] font-black">
                    #{{ run.id }} {{ getStatusLabel(run.status) }}
                  </span>
                  <span
                    class="shrink-0 text-[10px] font-semibold"
                    :class="historyMetaClass(activeRunId === run.id)"
                  >
                    {{ formatRunDuration(run) }}
                  </span>
                </div>
                <div
                  class="mt-0.5 truncate text-[10px] font-bold"
                  :class="historyMetaClass(activeRunId === run.id)"
                >
                  {{ formatDate(run.started_at) }}
                </div>
                <div
                  v-if="run.pid"
                  class="mt-1 inline-flex rounded px-1.5 py-0.5 text-[10px] font-semibold"
                  :class="activeRunId === run.id ? 'bg-[var(--accent-subtle-bg)] text-[var(--accent-subtle-text)]' : 'bg-subtle text-muted'"
                >
                  PID {{ run.pid }}
                </div>
              </div>
            </button>
          </div>

          <div
            v-if="historyRuns.length === 0 && !historyLoading"
            class="h-28 flex-center text-[11px] font-bold text-muted"
          >
            暂无历史
          </div>
        </div>

        <div
          v-if="historyHasMore"
          class="shrink-0 border-t border-slate-900/8 p-2 dark:border-white/8"
        >
          <button
            type="button"
            class="h-8 w-full cursor-pointer rounded-md bg-subtle text-xs font-semibold text-secondary transition-colors hover:text-default"
            @click="fetchHistory(false)"
          >
            加载更多
          </button>
        </div>
      </aside>
    </div>

    <footer class="shrink-0 border-t border-light bg-card px-3 py-2">
      <div
        class="flex min-h-10 flex-wrap items-center gap-2"
      >
        <button
          type="button"
          class="flex h-9 min-w-[150px] cursor-pointer items-center gap-2 rounded-md px-2 text-left transition-colors hover:bg-subtle"
          :title="currentTask.name"
          @click="selectLiveLog"
        >
          <div class="h-2.5 w-2.5 shrink-0 rounded-full" :class="statusDotClass"></div>
          <div class="min-w-0 flex-1">
            <div class="truncate text-[12px] font-black leading-4 text-default">
              {{ activeModeLabel }}
            </div>
            <div class="truncate text-[10px] font-bold leading-3 text-muted">
              {{ getStatusLabel(currentTask.status) }}
            </div>
          </div>
        </button>

        <div class="flex shrink-0 items-center gap-1 border-l border-light pl-2">
          <button
            v-if="showRunButton"
            type="button"
            class="h-8 w-8 cursor-pointer rounded-md text-emerald-600 flex-center transition-colors hover:bg-subtle dark:text-emerald-300"
            title="运行"
            aria-label="运行任务"
            @click="handleRun"
          >
            <div class="i-ep-video-play text-[15px]"></div>
          </button>
          <button
            v-if="currentTask.status === 'Running'"
            type="button"
            class="h-8 w-8 cursor-pointer rounded-md text-amber-600 flex-center transition-colors hover:bg-subtle dark:text-amber-300"
            title="暂停"
            aria-label="暂停任务"
            @click="handlePause"
          >
            <div class="i-ep-video-pause text-[15px]"></div>
          </button>
          <button
            v-if="currentTask.status === 'Paused'"
            type="button"
            class="h-8 w-8 cursor-pointer rounded-md text-emerald-600 flex-center transition-colors hover:bg-subtle dark:text-emerald-300"
            title="恢复"
            aria-label="恢复任务"
            @click="handleResume"
          >
            <div class="i-ep-refresh text-[15px]"></div>
          </button>
          <button
            v-if="currentTask.status === 'Running' || currentTask.status === 'Paused'"
            type="button"
            class="h-8 w-8 cursor-pointer rounded-md text-rose-600 flex-center transition-colors hover:bg-rose-500/10 dark:text-rose-300"
            title="停止"
            aria-label="停止任务"
            @click="handleStop"
          >
            <div class="i-ep-switch-button text-[15px]"></div>
          </button>
        </div>

        <div class="flex min-w-0 flex-1 items-center justify-end gap-1">
          <el-input
            v-if="searchVisible || searchQuery"
            ref="searchInputRef"
            v-model="searchQuery"
            clearable
            placeholder="搜索日志"
            size="small"
            class="!w-36 sm:!w-[180px]"
            @clear="searchVisible = false"
          >
            <template #prefix>
              <div class="i-ep-search text-[12px] text-muted"></div>
            </template>
          </el-input>

          <button
            type="button"
            class="h-8 w-8 shrink-0 cursor-pointer rounded-md text-muted flex-center transition-colors hover:bg-subtle hover:text-default"
            title="搜索"
            aria-label="搜索日志"
            @click="openSearch"
          >
            <div class="i-ep-search text-[14px]"></div>
          </button>
          <button
            type="button"
            class="h-8 w-8 shrink-0 cursor-pointer rounded-md flex-center transition-colors hover:bg-subtle"
            :class="onlyShowMatches ? 'text-primary' : 'text-muted hover:text-default'"
            title="只看匹配"
            :aria-pressed="onlyShowMatches"
            aria-label="只显示匹配日志"
            @click="toggleOnlyShowMatches"
          >
            <div class="i-ep-view text-[14px]"></div>
          </button>
          <button
            type="button"
            class="h-8 w-8 shrink-0 cursor-pointer rounded-md flex-center transition-colors hover:bg-subtle"
            :class="wrapEnabled ? 'text-primary' : 'text-muted hover:text-default'"
            title="换行"
            :aria-pressed="wrapEnabled"
            aria-label="切换日志换行"
            @click="toggleWrap"
          >
            <div class="i-ep-sort text-[14px]"></div>
          </button>
          <button
            type="button"
            class="h-8 w-8 shrink-0 cursor-pointer rounded-md text-muted flex-center transition-colors hover:bg-subtle hover:text-default"
            title="滚动到底部"
            aria-label="滚动到日志底部"
            @click="logViewerRef?.scrollToBottom?.()"
          >
            <div class="i-ep-d-arrow-right rotate-90 text-[14px]"></div>
          </button>
          <button
            type="button"
            class="h-8 w-8 shrink-0 cursor-pointer rounded-md text-muted flex-center transition-colors hover:bg-subtle hover:text-default"
            title="下载日志"
            aria-label="下载日志"
            @click="downloadLogs"
          >
            <div class="i-ep-download text-[14px]"></div>
          </button>
          <button
            type="button"
            class="h-8 w-8 shrink-0 cursor-pointer rounded-md text-muted flex-center transition-colors hover:bg-rose-500/10 hover:text-rose-600 dark:hover:text-rose-300"
            title="清空当前窗口"
            aria-label="清空当前窗口"
            @click="logViewerRef?.clear?.()"
          >
            <div class="i-ep-delete text-[14px]"></div>
          </button>
          <button
            type="button"
            class="h-8 w-8 shrink-0 cursor-pointer rounded-md flex-center transition-colors hover:bg-subtle"
            :class="historyPanelVisible ? 'text-primary' : 'text-muted hover:text-default'"
            title="历史记录"
            :aria-pressed="historyPanelVisible"
            aria-label="切换历史记录"
            @click="toggleHistoryPanel"
          >
            <div class="i-ep-clock text-[14px]"></div>
          </button>
        </div>
      </div>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { ElMessage } from "element-plus";
import * as taskApi from "@/api/tasks";
import { useTaskLogDownload } from "@/composables/useTaskLogDownload";
import {
  formatDuration,
  getStatusLabel,
} from "@/composables/useTaskPresentation";
import LogViewer from "@/components/common/LogViewer.vue";
import { useTaskStore } from "@/stores/tasks";
import { useWorkspaceLogSessionStore } from "@/stores/workspaceLogSessions";
import type { Task, TaskRunHistoryItem, TaskStatus } from "@/types";
import type { TaskLogViewerRef } from "@/composables/taskPageTypes";
import type { TaskLogWindowPayload } from "@/types/workspace";
import { formatDate } from "@/utils/format";

const props = defineProps<{
  payload: TaskLogWindowPayload;
  isMobile?: boolean;
}>();

const taskStore = useTaskStore();
const sessionStore = useWorkspaceLogSessionStore();
const logViewerRef = ref<TaskLogViewerRef | null>(null);
const searchInputRef = ref<{ focus?: () => void } | null>(null);
const activeRunId = ref<number | null>(props.payload.runId);
const historyPanelVisible = ref(Boolean(props.payload.runId));
const historyRuns = ref<TaskRunHistoryItem[]>([]);
const historyLoading = ref(false);
const historyPage = ref(1);
const historyTotal = ref(0);
const searchVisible = ref(false);
const searchQuery = ref("");
const onlyShowMatches = ref(false);
const wrapEnabled = ref(true);

let unsubscribe: (() => void) | null = null;
const historyPageSize = 20;

const currentTask = computed<Task>(() => {
  return (
    taskStore.tasks.find((task) => task.id === props.payload.taskId) ??
    props.payload.task
  );
});

const streamRunId = computed(() =>
  typeof currentTask.value.run_id === "number" ? currentTask.value.run_id : null,
);

const sessionKey = computed(
  () => `task-log:${props.payload.taskId}:${activeRunId.value ?? "live"}`,
);

const liveModeLabel = computed(() =>
  currentTask.value.status === "Running" ? "实时日志" : "最近日志",
);

const activeModeLabel = computed(() =>
  activeRunId.value ? `运行 #${activeRunId.value}` : liveModeLabel.value,
);

const showRunButton = computed(
  () =>
    currentTask.value.status !== "Running" &&
    currentTask.value.status !== "Paused",
);

const historyHasMore = computed(
  () => historyRuns.value.length < historyTotal.value,
);

const statusColorMap: Record<TaskStatus, string> = {
  Pending: "bg-slate-400",
  Running: "bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]",
  Finished: "bg-blue-500",
  Failed: "bg-rose-500",
  Cancelled: "bg-muted",
  Stopped: "bg-muted",
  Paused: "bg-amber-500",
  Idle: "bg-muted",
};

const statusDotClass = computed(
  () => statusColorMap[currentTask.value.status] ?? "bg-muted",
);

const historyRowClass = (selected: boolean) =>
  selected
    ? "bg-soft text-primary"
    : "text-secondary hover:bg-subtle hover:text-default";

const historyMetaClass = (selected: boolean) =>
  selected ? "text-primary/75" : "text-muted";

const liveStatusDotClass = computed(() => {
  if (currentTask.value.status === "Running") {
    return !activeRunId.value
      ? "bg-emerald-500 animate-pulse ring-2 ring-emerald-500/20"
      : "bg-emerald-500 animate-pulse";
  }

  return !activeRunId.value ? "bg-primary" : "bg-slate-400";
});

const { downloadLogs } = useTaskLogDownload({
  activeLogTask: () => currentTask.value,
  selectedHistoryRunId: activeRunId,
});

const teardown = () => {
  unsubscribe?.();
  unsubscribe = null;
};

const loadReadonlyLog = () => {
  const taskId = props.payload.taskId;
  const runId = activeRunId.value;

  logViewerRef.value?.reset?.();
  logViewerRef.value?.init?.(async (offset: number, limit: number) => {
    const res = runId
      ? await taskApi.getTaskRunLog(taskId, runId, offset, limit)
      : await taskApi.getLatestLog(taskId, offset, limit);
    return res.data;
  });
};

const connect = async () => {
  await nextTick();
  teardown();

  const viewer = logViewerRef.value;
  if (!viewer) return;

  if (activeRunId.value || currentTask.value.status !== "Running") {
    loadReadonlyLog();
    return;
  }

  viewer.reset?.();

  unsubscribe = sessionStore.subscribe(
    sessionKey.value,
    props.payload.taskId,
    streamRunId.value,
    (content) => viewer.write?.(content),
  );
  sessionStore.connectLive(
    sessionKey.value,
    props.payload.taskId,
    streamRunId.value,
  );
};

const fetchHistory = async (reset = true) => {
  if (historyLoading.value) return;

  historyLoading.value = true;
  const nextPage = reset ? 1 : historyPage.value + 1;

  try {
    const res = await taskApi.getTaskHistory(
      props.payload.taskId,
      nextPage,
      historyPageSize,
    );
    historyRuns.value = reset
      ? res.data.items
      : [...historyRuns.value, ...res.data.items];
    historyTotal.value = res.data.total;
    historyPage.value = nextPage;
  } catch {
    ElMessage.error("加载历史失败");
  } finally {
    historyLoading.value = false;
  }
};

const selectLiveLog = () => {
  activeRunId.value = null;
};

const selectHistoryRun = (runId: number) => {
  activeRunId.value = runId;
};

const toggleHistoryPanel = () => {
  historyPanelVisible.value = !historyPanelVisible.value;
  if (historyPanelVisible.value && historyRuns.value.length === 0) {
    void fetchHistory(true);
  }
};

const openSearch = async () => {
  searchVisible.value = true;
  await nextTick();
  searchInputRef.value?.focus?.();
};

const toggleOnlyShowMatches = () => {
  onlyShowMatches.value = !onlyShowMatches.value;
};

const toggleWrap = () => {
  wrapEnabled.value = !wrapEnabled.value;
  logViewerRef.value?.toggleWrap?.();
};

const refreshTaskState = async () => {
  await taskStore.refreshTasks(true);
  void fetchHistory(true);
};

const handleRun = async () => {
  activeRunId.value = null;
  await taskStore.runTask(currentTask.value.id);
  await refreshTaskState();
};

const handlePause = async () => {
  await taskStore.pauseTask(currentTask.value.id);
  await refreshTaskState();
};

const handleResume = async () => {
  activeRunId.value = null;
  await taskStore.resumeTask(currentTask.value.id);
  await refreshTaskState();
};

const handleStop = async () => {
  await taskStore.stopTask(currentTask.value.id);
  await refreshTaskState();
};

const runStatusDotClass = (status: TaskStatus) => {
  if (status === "Running") return "bg-emerald-500 animate-pulse";
  if (status === "Finished") return "bg-blue-500";
  if (status === "Failed") return "bg-rose-500";
  if (status === "Paused") return "bg-amber-500";
  return "bg-slate-400";
};

const formatRunDuration = (run: TaskRunHistoryItem) => {
  if (!run.started_at || !run.ended_at) return "-";
  const seconds = Math.max(
    0,
    Math.round(
      (new Date(run.ended_at).getTime() - new Date(run.started_at).getTime()) /
        1000,
    ),
  );
  return formatDuration(seconds);
};

watch(
  () => [props.payload.taskId, props.payload.runId] as const,
  ([taskId, runId], [oldTaskId]) => {
    activeRunId.value = runId;
    if (taskId !== oldTaskId) void fetchHistory(true);
  },
);

watch(
  () => [
    props.payload.taskId,
    activeRunId.value,
    currentTask.value.status,
    currentTask.value.run_id,
  ],
  () => {
    void connect();
  },
);

watch(
  () => [searchQuery.value, onlyShowMatches.value] as const,
  ([query, filter]) => {
    logViewerRef.value?.setSearch?.(query, filter);
  },
);

onMounted(() => {
  void fetchHistory(true);
  void connect();
});

onUnmounted(teardown);
</script>
