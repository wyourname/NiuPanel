<template>
  <div
    class="px-4 h-12 flex items-center justify-between border-b border-[var(--editor-border)] shrink-0 relative z-20"
  >
    <div class="flex items-center gap-1.5 min-w-0 flex-1">
      <button
        type="button"
        class="h-10 w-10 rounded-md bg-transparent text-secondary flex-center transition-colors hover:bg-soft touch-manipulation"
        aria-label="返回任务列表"
        @click="emit('close')"
      >
        <div class="i-ep-back text-[22px]"></div>
      </button>

      <div class="w-10 h-10 flex-center shrink-0">
        <div :class="getEnvIcon(task)" class="text-3xl"></div>
      </div>

      <div class="flex flex-col min-w-0 ml-2 justify-center">
        <span
          class="truncate text-[15px] font-bold leading-tight text-default"
          >{{ task?.name }}</span
        >
        <span
          class="text-[13px] text-primary truncate leading-tight mt-0.5"
          :class="task?.status === 'Running' ? 'animate-pulse' : 'text-secondary'"
        >
          {{ task?.status === "Running" ? "正在运行" : task?.status === "Paused" ? "已暂停" : "未运行" }}
        </span>
      </div>
    </div>

    <div class="flex items-center pr-2 shrink-0 gap-1">
      <button
        type="button"
        class="h-9 w-9 rounded-md border-none bg-transparent text-secondary flex-center transition-colors hover:bg-soft touch-manipulation"
        :class="showTimeline ? 'bg-[var(--accent-subtle-bg)] text-[var(--accent-subtle-text)]' : 'text-secondary'"
        title="运行历史"
        aria-label="显示运行历史"
        @click="emit('toggle-timeline')"
      >
        <div class="i-ep-clock text-[20px]"></div>
      </button>

      <el-dropdown trigger="click">
        <button
          type="button"
          class="h-9 w-9 rounded-md border-none bg-transparent text-secondary flex-center transition-colors hover:bg-soft touch-manipulation"
          title="更多操作"
          aria-label="更多日志操作"
        >
          <div class="i-ep-more-filled text-[22px] transform rotate-90"></div>
        </button>
        <template #dropdown>
          <el-dropdown-menu class="modern-dropdown w-48">
            <el-dropdown-item v-if="task" @click="emit('edit', task)">
              <div class="flex items-center gap-3">
                <div class="i-ep-edit text-lg"></div>
                编辑任务
              </div>
            </el-dropdown-item>
            <el-dropdown-item
              v-if="task"
              @click="emit('edit-variables', task.id)"
            >
              <div class="flex items-center gap-3">
                <div class="i-ep-key text-lg text-emerald-500"></div>
                环境变量
              </div>
            </el-dropdown-item>
            <el-dropdown-item v-if="task" @click="emit('edit-cron', task)">
              <div class="flex items-center gap-3">
                <div class="i-ep-clock text-lg text-orange-400"></div>
                定时规则
              </div>
            </el-dropdown-item>
            <el-dropdown-item v-if="task" @click="emit('edit-script', task)">
              <div class="flex items-center gap-3">
                <div class="i-ep-document text-lg text-purple-500"></div>
                编辑脚本
              </div>
            </el-dropdown-item>
            <el-dropdown-item v-if="task" @click="emit('share', task)">
              <div class="flex items-center gap-3">
                <div class="i-ep-share text-lg text-purple-500"></div>
                分享资源
              </div>
            </el-dropdown-item>
            <div class="h-px bg-light/50 my-1 mx-2"></div>
            <el-dropdown-item @click="emit('clear')">
              <div class="flex items-center gap-3">
                <div class="i-ep-delete text-lg text-rose-500"></div>
                清空日志
              </div>
            </el-dropdown-item>
            <el-dropdown-item @click="emit('download-logs')">
              <div class="flex items-center gap-3">
                <div class="i-ep-download text-lg text-blue-500"></div>
                下载日志
              </div>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </div>
</template>

<script setup lang="ts">
import { getEnvIcon } from "../../composables/useTaskPresentation";
import type { Task } from "@/types";

defineProps<{
  showTimeline: boolean;
  task?: Task;
}>();

const emit = defineEmits<{
  (event: "clear"): void;
  (event: "close"): void;
  (event: "download-logs"): void;
  (event: "edit", task: Task): void;
  (event: "edit-cron", task: Task): void;
  (event: "edit-script", task: Task): void;
  (event: "edit-variables", taskId: number): void;
  (event: "share", task: Task): void;
  (event: "toggle-timeline"): void;
}>();
</script>
