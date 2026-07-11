<template>
  <div
    :class="containerClass"
    v-infinite-scroll="() => emit('load-more')"
    :infinite-scroll-disabled="loading || !hasMore"
    :infinite-scroll-distance="20"
  >
    <div :class="headerClass">
      <h3 :class="titleClass">时间轴</h3>
      <button
        type="button"
        class="h-6 w-6 rounded-md text-muted flex-center transition-colors hover:bg-light/50 hover:text-primary"
        title="刷新运行记录"
        aria-label="刷新运行记录"
        @click.stop="emit('refresh')"
      >
        <div class="i-ep-refresh text-[12px]"></div>
      </button>
    </div>

    <div class="relative pl-5" v-loading="loading && page === 1">
      <div
        class="absolute left-0 top-3 bottom-0 w-[1.5px] bg-black/10 dark:bg-white/10"
      ></div>

      <div class="relative pb-6 cursor-pointer group" @click="emit('select', null)">
        <div
          class="absolute -left-[27px] top-1 w-[11px] h-[11px] rounded-full border-[2.5px] border-white dark:border-[#121212] z-10 transition-colors"
          :class="
            !selectedRunId
              ? 'bg-primary'
              : inactiveLiveDotClass
          "
        ></div>

        <div
          class="mb-0.5 mt-[1px] flex items-center gap-1 font-mono text-[9px] font-bold"
          :class="
            !selectedRunId
              ? 'text-primary'
              : inactiveLiveMetaClass
          "
        >
          <div
            v-if="taskStatus === 'Running'"
            class="w-1.5 h-1.5 rounded-full bg-success animate-pulse"
          ></div>
          {{ taskStatus === "Running" ? "实时日志" : "最新日志" }}
        </div>
        <div
          class="text-[11px] font-bold transition-colors leading-tight"
          :class="!selectedRunId ? 'text-default' : inactiveLiveTitleClass"
        >
          实时日志监控
        </div>
      </div>

      <div
        v-for="(run, index) in runs"
        :key="run.id"
        class="relative pb-6 cursor-pointer group"
        @click="emit('select', run.id)"
      >
        <div
          v-if="index === runs.length - 1 && !hasMore"
          class="absolute -left-[5px] bottom-0 top-4 z-0 w-3 bg-base"
        ></div>
        <div
          class="absolute -left-[27px] top-1 w-[11px] h-[11px] rounded-full border-[2.5px] border-white dark:border-[#121212] z-10 transition-colors"
          :class="
            selectedRunId === run.id
              ? 'bg-warning'
              : inactiveRunDotClass
          "
        ></div>
        <div
          class="text-[9px] font-mono mb-0.5 mt-[1px] transition-colors"
          :class="
            selectedRunId === run.id
              ? 'text-warning font-bold'
              : inactiveRunMetaClass
          "
        >
          {{ run.started_at ? formatDate(run.started_at).split(" ")[0] : "-" }}
          <span class="opacity-60 text-[8px]">
            {{ run.started_at ? formatDate(run.started_at).split(" ")[1] : "" }}
          </span>
        </div>
        <div
          class="text-[11px] font-bold transition-colors leading-tight mb-0.5 flex items-center gap-1.5 break-words"
          :class="[
            selectedRunId === run.id ? 'text-default' : inactiveRunTitleClass,
          ]"
        >
          {{ getRunStatusLabel(run.status) }}
          <div
            v-if="run.status === 'Failed'"
            class="i-ep-warning-filled text-danger text-sm"
          ></div>
        </div>
        <div
          class="text-[9px] font-mono opacity-80"
          :class="
            selectedRunId === run.id ? 'text-warning/80' : 'text-muted'
          "
        >
          耗时：
          {{
            run.started_at && run.ended_at
              ? formatDuration(
                  Math.round(
                    (new Date(run.ended_at).getTime() -
                      new Date(run.started_at).getTime()) /
                      1000,
                  ),
                )
              : "-"
          }}
        </div>
      </div>

      <div v-if="runs.length === 0 && !loading" class="pt-2 text-center opacity-50">
        <span :class="emptyClass">暂无记录</span>
      </div>
      <div v-if="loading && page > 1" class="pt-2 pb-4 text-center">
        <div
          class="i-ep-loading animate-spin text-primary inline-block text-sm"
        ></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { formatDate } from "../../utils/format";
import { formatDuration } from "../../composables/useTaskPresentation";
import type { TaskRunTimelineItem } from "../../composables/taskPageTypes";

const props = withDefaults(
  defineProps<{
    hasMore: boolean;
    loading: boolean;
    page: number;
    runs: TaskRunTimelineItem[];
    selectedRunId: number | null;
    taskStatus?: string;
    variant?: "desktop" | "mobile";
  }>(),
  {
    taskStatus: undefined,
    variant: "desktop",
  },
);

const emit = defineEmits<{
  (event: "load-more"): void;
  (event: "refresh"): void;
  (event: "select", runId: number | null): void;
}>();

const isMobile = computed(() => props.variant === "mobile");

const containerClass = computed(() =>
  isMobile.value
    ? "w-full h-auto max-h-full bg-card border border-light rounded-md p-3 pointer-events-auto overflow-y-auto custom-scrollbar flex flex-col transition-colors"
    : "w-full h-auto max-h-full bg-card/90 border border-light rounded-md p-3 pointer-events-auto overflow-y-auto custom-scrollbar flex flex-col transition-colors",
);

const headerClass = computed(() =>
  isMobile.value
    ? "flex items-center justify-between mb-4 sticky top-0 bg-card z-10 px-1 py-1 -mx-1 -mt-1"
    : "flex items-center justify-between mb-4 sticky top-0 bg-card/90 z-10 px-1",
);

const titleClass = computed(() =>
  isMobile.value
    ? "text-[11px] font-semibold text-default"
    : "label-xs",
);

const emptyClass = computed(() =>
  isMobile.value
    ? "text-[9px] font-semibold text-muted"
    : "label-micro",
);

const inactiveLiveDotClass = computed(() =>
  isMobile.value
    ? "bg-gray-300 dark:bg-gray-600"
    : "bg-gray-300 dark:bg-gray-600 group-hover:bg-primary/50",
);

const inactiveLiveMetaClass = computed(() =>
  isMobile.value
    ? "text-muted"
    : "text-muted group-hover:text-primary/70",
);

const inactiveLiveTitleClass = computed(() =>
  isMobile.value ? "text-muted" : "text-muted group-hover:text-default",
);

const inactiveRunDotClass = computed(() =>
  isMobile.value
    ? "bg-gray-300 dark:bg-gray-600"
    : "bg-gray-300 dark:bg-gray-600 group-hover:bg-warning/50",
);

const inactiveRunMetaClass = computed(() =>
  isMobile.value
    ? "text-muted"
    : "text-muted group-hover:text-warning/70",
);

const inactiveRunTitleClass = computed(() =>
  isMobile.value ? "text-muted" : "text-muted group-hover:text-default",
);

const getRunStatusLabel = (status?: string) => {
  if (status === "Finished") return "执行完成";
  if (status === "Failed") return "执行出错";
  return status || "-";
};
</script>
