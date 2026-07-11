<template>
  <div
    v-show="activeTab === 'log' || activeTab === 'info'"
    class="h-14 px-4 bg-card border-t border-base shrink-0 flex items-center justify-between z-30"
  >
    <div class="flex items-center gap-3">
      <div class="flex items-center gap-2">
        <div class="w-1.5 h-1.5 rounded-full" :class="getStatusDotClass(task)"></div>
        <span class="text-[10px] font-semibold text-secondary">
          {{ getStatusLabel(task.status) }}
        </span>
      </div>
      <div
        v-if="task.status === 'Running' && activeTab === 'log'"
        class="flex items-center gap-1.5 text-primary animate-fade-in"
      >
        <div class="i-ep-loading animate-spin text-sm"></div>
        <span class="text-[10px] font-semibold">
          实时日志
        </span>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <template v-if="activeTab === 'log'">
        <template v-if="['Running', 'Paused'].includes(task.status)">
          <el-button
            :type="task.status === 'Running' ? 'warning' : 'success'"
            class="!h-8 !px-3 !rounded-md text-[11px] font-semibold"
            @click="emit('action', task.status === 'Running' ? 'pause' : 'resume')"
          >
            {{ task.status === "Running" ? "暂停" : "恢复" }}
          </el-button>
          <el-button
            type="danger"
            class="!h-8 !px-3 !rounded-md text-[11px] font-semibold"
            @click="emit('action', 'stop')"
          >
            终止任务
          </el-button>
        </template>
        <template v-else>
          <el-button
            type="primary"
            class="!h-8 !px-4 !rounded-md text-[11px] font-semibold"
            @click="emit('action', 'run')"
          >
            <div class="i-ep-video-play mr-2 text-xl"></div>
            立即运行任务
          </el-button>
        </template>
      </template>

      <template v-if="activeTab === 'info'">
        <el-button
          class="!h-8 !px-3 !rounded-md text-[11px] font-semibold"
          @click="emit('edit')"
        >
          <div class="i-ep-edit mr-2"></div>
          编辑任务设置
        </el-button>
        <el-button
          type="primary"
          class="!h-8 !px-3 !rounded-md text-[11px] font-semibold"
          :disabled="task.status === 'Running'"
          @click="emit('action', 'run')"
        >
          <div class="i-ep-video-play mr-2"></div>
          运行
        </el-button>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Task } from "@/types";
import type { TaskFooterAction } from "../../composables/taskPageTypes";
import {
  getStatusDotClass,
  getStatusLabel,
} from "../../composables/useTaskPresentation";

type TaskDetailTab = "log" | "script" | "var" | "info";

defineProps<{
  task: Task;
  activeTab: TaskDetailTab;
}>();

const emit = defineEmits<{
  (event: "action", action: TaskFooterAction): void;
  (event: "edit"): void;
}>();
</script>
