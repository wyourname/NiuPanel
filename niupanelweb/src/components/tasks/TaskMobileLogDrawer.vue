<template>
  <OverlayDrawer
    v-model:visible="drawerVisible"
    :title="task?.name || '任务日志'"
    variant="workspace"
    content-preset="workspace"
    destroy-on-close
    append-to-body
    :lock-scroll="false"
  >
    <template #title>
      <div class="flex min-w-0 items-center gap-2">
        <span
          :class="getEnvIcon(task)"
          class="h-8 w-8 shrink-0 text-xl flex-center"
          aria-hidden="true"
        ></span>
        <div class="min-w-0">
          <div class="truncate text-[13px] font-bold leading-tight text-default">
            {{ task?.name || "任务日志" }}
          </div>
          <div
            class="mt-0.5 truncate text-[10px] font-semibold leading-tight"
            :class="task?.status === 'Running' ? 'text-primary' : 'text-muted'"
          >
            {{ task?.status === "Running" ? "正在运行" : task?.status === "Paused" ? "已暂停" : "未运行" }}
          </div>
        </div>
      </div>
    </template>

    <template #header-actions>
      <button
        type="button"
        class="mobile-touch-target cursor-pointer rounded-md text-secondary flex-center transition-colors hover:bg-soft hover:text-default"
        :class="showTimeline ? 'accent-subtle' : ''"
        title="运行历史"
        aria-label="显示运行历史"
        @click="toggleTimeline"
      >
        <span class="i-ep-clock text-[18px]" aria-hidden="true"></span>
      </button>

      <el-dropdown trigger="click">
        <button
          type="button"
          class="mobile-touch-target cursor-pointer rounded-md text-secondary flex-center transition-colors hover:bg-soft hover:text-default"
          title="更多操作"
          aria-label="更多日志操作"
        >
          <span class="i-ep-more-filled rotate-90 text-[20px]" aria-hidden="true"></span>
        </button>
        <template #dropdown>
          <el-dropdown-menu class="modern-dropdown w-48">
            <el-dropdown-item v-if="task" @click="emit('edit', task)">
              <span class="flex items-center gap-3"><span class="i-ep-edit text-lg"></span>编辑任务</span>
            </el-dropdown-item>
            <el-dropdown-item v-if="task" @click="emit('edit-variables', task.id)">
              <span class="flex items-center gap-3"><span class="i-ep-key text-lg text-emerald-500"></span>环境变量</span>
            </el-dropdown-item>
            <el-dropdown-item v-if="task" @click="emit('edit-cron', task)">
              <span class="flex items-center gap-3"><span class="i-ep-clock text-lg text-orange-400"></span>定时规则</span>
            </el-dropdown-item>
            <el-dropdown-item v-if="task" @click="emit('edit-script', task)">
              <span class="flex items-center gap-3"><span class="i-ep-document text-lg text-purple-500"></span>编辑脚本</span>
            </el-dropdown-item>
            <el-dropdown-item v-if="task" @click="emit('share', task)">
              <span class="flex items-center gap-3"><span class="i-ep-share text-lg text-purple-500"></span>分享资源</span>
            </el-dropdown-item>
            <div class="mx-2 my-1 h-px bg-light/50"></div>
            <el-dropdown-item @click="clear">
              <span class="flex items-center gap-3"><span class="i-ep-delete text-lg text-rose-500"></span>清空日志</span>
            </el-dropdown-item>
            <el-dropdown-item @click="emit('download-logs')">
              <span class="flex items-center gap-3"><span class="i-ep-download text-lg text-blue-500"></span>下载日志</span>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </template>

    <div class="flex-1 flex flex-col bg-[var(--editor-bg)] overflow-hidden">
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
  </OverlayDrawer>
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
import OverlayDrawer from "../common/OverlayDrawer.vue";
import { getEnvIcon } from "../../composables/useTaskPresentation";
import TaskRunTimeline from "./TaskRunTimeline.vue";
import TaskMobileLogFooter from "./TaskMobileLogFooter.vue";

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
