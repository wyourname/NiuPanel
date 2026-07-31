<template>
  <el-drawer
    v-model="drawerVisible"
    size="100%"
    :with-header="false"
    direction="btt"
    destroy-on-close
    append-to-body
    class="log-modal"
    :lock-scroll="false"
  >
    <div class="flex-1 flex flex-col bg-[var(--editor-bg)] overflow-hidden">
      <TaskMobileLogHeader
        :show-timeline="showTimeline"
        :task="task"
        @clear="clear"
        @close="drawerVisible = false"
        @download-logs="emit('download-logs')"
        @edit="emit('edit', $event)"
        @edit-cron="emit('edit-cron', $event)"
        @edit-script="emit('edit-script', $event)"
        @edit-variables="emit('edit-variables', $event)"
        @share="emit('share', $event)"
        @toggle-timeline="toggleTimeline"
      />

      <div class="flex-1 overflow-hidden relative z-10">
        <div
          v-show="showTimeline"
          class="absolute right-4 top-4 bottom-4 w-[160px] pointer-events-none z-30 flex flex-col justify-start touch-pan-y"
        >
          <TaskRunTimeline
            :runs="runs"
            :selected-run-id="selectedRunId"
            :loading="timelineLoading"
            :page="timelinePage"
            :has-more="timelineHasMore"
            :task-status="task?.status"
            variant="mobile"
            @load-more="emit('load-more-timeline')"
            @refresh="emit('refresh-timeline')"
            @select="handleTimelineSelect"
          />
        </div>

        <div
          ref="mobileWidgetRef"
          class="absolute z-20 flex flex-col gap-4"
          :style="{
            left: `${mobileWidgetX}px`,
            top: `${mobileWidgetY}px`,
            touchAction: 'none',
          }"
        >
          <transition name="el-zoom-in-center">
            <div
              v-if="showLogProgress"
              class="pointer-events-auto cursor-move rounded-md border border-light bg-card p-2"
            >
              <el-progress
                type="circle"
                :percentage="logProgressValue"
                :width="54"
                :stroke-width="5"
                color="#3b82f6"
              />
            </div>
          </transition>
          <transition name="el-zoom-in-center">
            <div
              v-if="logQrCodeData"
              class="pointer-events-auto rounded-md border border-light bg-white p-2"
            >
              <img
                :src="logQrCodeData"
                class="h-20 w-20 cursor-zoom-in rounded-md object-contain"
                @click="emit('expand-qr')"
              />
              <div class="absolute inset-0 cursor-move" style="z-index: -1"></div>
            </div>
          </transition>
        </div>

        <MobileLogViewer
          ref="mobileLogViewerRef"
          :searchText="logSearchQuery"
          @ui-event="emit('ui-event', $event)"
        />
      </div>

      <TaskMobileLogFooter :task="task" @action="emit('action', $event)" />
    </div>
  </el-drawer>
</template>

<script setup lang="ts">
import { ref, toRef } from "vue";
import MobileLogViewer from "../common/MobileLogViewer.vue";
import { useTaskLogViewerBridge } from "../../composables/useTaskLogViewerBridge";
import { useTaskMobileLogDrawerState } from "../../composables/useTaskMobileLogDrawerState";
import type {
  TaskLogUiEvent,
  TaskLogViewerRef,
  TaskRunTimelineItem,
} from "../../composables/taskPageTypes";
import type { Task } from "@/types";
import TaskRunTimeline from "./TaskRunTimeline.vue";
import TaskMobileLogFooter from "./TaskMobileLogFooter.vue";
import TaskMobileLogHeader from "./TaskMobileLogHeader.vue";

const props = defineProps<{
  logProgressValue: number;
  logQrCodeData: string | null;
  logSearchQuery: string;
  modelValue: boolean;
  runs: TaskRunTimelineItem[];
  selectedRunId: number | null;
  showLogProgress: boolean;
  task?: Task;
  timelineHasMore: boolean;
  timelineLoading: boolean;
  timelinePage: number;
}>();

const emit = defineEmits<{
  (event: "action", action: string): void;
  (event: "download-logs"): void;
  (event: "edit", task: Task): void;
  (event: "edit-cron", task: Task): void;
  (event: "edit-script", task: Task): void;
  (event: "edit-variables", taskId: number): void;
  (event: "expand-qr"): void;
  (event: "load-more-timeline"): void;
  (event: "refresh-timeline"): void;
  (event: "select-timeline", runId: number | null): void;
  (event: "share", task: Task): void;
  (event: "ui-event", payload: TaskLogUiEvent): void;
  (event: "update:modelValue", value: boolean): void;
}>();

const {
  drawerVisible,
  handleTimelineSelect,
  mobileWidgetRef,
  mobileWidgetX,
  mobileWidgetY,
  showTimeline,
  toggleTimeline,
} = useTaskMobileLogDrawerState({
  modelValue: toRef(props, "modelValue"),
  onRefreshTimeline: () => emit("refresh-timeline"),
  onSelectTimeline: (runId) => emit("select-timeline", runId),
  onUpdateVisible: (value) => emit("update:modelValue", value),
});

const mobileLogViewerRef = ref<TaskLogViewerRef | null>(null);
const logViewerBridge = useTaskLogViewerBridge(mobileLogViewerRef);
const { clear } = logViewerBridge;

defineExpose(logViewerBridge);
</script>
