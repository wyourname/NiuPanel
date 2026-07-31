<template>
  <div class="full relative flex bg-white dark:bg-[#0e1621]">
    <div
      class="flex-1 relative flex flex-col h-full overflow-hidden bg-white dark:bg-[#0e1621]"
    >
      <div
        v-if="selectedRunId"
        class="absolute inset-x-0 top-0 z-20 mx-auto w-max rounded-b-md border border-t-0 border-amber-500 bg-amber-500 px-5 py-1.5 text-[10px] font-bold text-white"
      >
        历史只读模式 - 运行 #{{ selectedRunId }}
      </div>

      <LogViewer
        ref="logViewerRef"
        :is-mobile="false"
        class="w-full h-full"
        @ui-event="emit('ui-event', $event)"
      />

      <div
        class="absolute right-[70px] top-6 bottom-[80px] w-[160px] pointer-events-none z-30 flex flex-col justify-start"
      >
        <TaskRunTimeline
          :runs="runs"
          :selected-run-id="selectedRunId"
          :loading="timelineLoading"
          :page="timelinePage"
          :has-more="timelineHasMore"
          :task-status="taskStatus"
          variant="desktop"
          @load-more="emit('load-more')"
          @refresh="emit('refresh')"
          @select="emit('select', $event)"
        />
      </div>

      <div
        class="absolute top-6 right-[260px] z-20 flex flex-col gap-4 pointer-events-none"
      >
        <transition name="el-zoom-in-center">
          <div
            v-if="showLogProgress"
            class="pointer-events-auto rounded-md border border-light bg-card p-3"
          >
            <el-progress
              type="circle"
              :percentage="logProgressValue"
              :width="70"
              :stroke-width="6"
              color="#3b82f6"
            />
          </div>
        </transition>
        <transition name="el-zoom-in-center">
          <div
            v-if="logQrCodeData"
            class="pointer-events-auto cursor-zoom-in rounded-md border border-light bg-white p-2"
            @click="emit('expand-qr')"
          >
            <img
              :src="logQrCodeData"
              class="h-24 w-28 rounded-md object-contain"
            />
          </div>
        </transition>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import LogViewer from "../common/LogViewer.vue";
import TaskRunTimeline from "./TaskRunTimeline.vue";
import { useTaskLogViewerBridge } from "../../composables/useTaskLogViewerBridge";
import type {
  TaskLogUiEvent,
  TaskLogViewerRef,
  TaskRunTimelineItem,
} from "../../composables/taskPageTypes";

defineProps<{
  logProgressValue: number;
  logQrCodeData: string | null;
  runs: TaskRunTimelineItem[];
  selectedRunId: number | null;
  showLogProgress: boolean;
  taskStatus?: string;
  timelineHasMore: boolean;
  timelineLoading: boolean;
  timelinePage: number;
}>();

const emit = defineEmits<{
  (event: "expand-qr"): void;
  (event: "load-more"): void;
  (event: "refresh"): void;
  (event: "select", runId: number | null): void;
  (event: "ui-event", payload: TaskLogUiEvent): void;
}>();

const logViewerRef = ref<TaskLogViewerRef | null>(null);
const logViewerBridge = useTaskLogViewerBridge(logViewerRef);

defineExpose(logViewerBridge);
</script>
