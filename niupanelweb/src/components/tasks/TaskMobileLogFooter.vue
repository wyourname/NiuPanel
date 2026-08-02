<template>
  <div
    class="task-mobile-log-footer relative z-20 flex shrink-0 items-center justify-between gap-2 border-t border-light/50 bg-card px-3 pt-2"
  >
    <template v-if="task?.status && ['Running', 'Paused'].includes(task.status)">
      <el-button
        size="default"
        :type="task.status === 'Running' ? 'warning' : 'success'"
        class="!m-0 !min-h-11 flex-1 !rounded-md text-[13px] font-bold"
        @click="emit('action', task.status === 'Running' ? 'pause' : 'resume')"
      >
        <div
          :class="task.status === 'Running' ? 'i-ep-video-pause' : 'i-ep-refresh'"
          class="mr-1.5 text-base font-normal"
        ></div>
        {{ task.status === "Running" ? "暂停" : "恢复" }}
      </el-button>
      <el-button
        type="danger"
        size="default"
        class="!m-0 !min-h-11 flex-1 !rounded-md text-[13px] font-bold"
        @click="emit('action', 'stop')"
      >
        <div class="i-ep-switch-button mr-1.5 text-base font-normal"></div>
        停止
      </el-button>
    </template>
    <el-button
      v-else
      type="primary"
      size="default"
      class="!m-0 !min-h-11 w-full !rounded-md text-[13px] font-bold"
      @click="emit('action', 'run')"
    >
      <div class="i-ep-video-play mr-1.5 text-base font-normal"></div>
      立即运行
    </el-button>
  </div>
</template>

<script setup lang="ts">
import type { Task } from "@/types";

defineProps<{
  task?: Task;
}>();

const emit = defineEmits<{
  (event: "action", action: string): void;
}>();
</script>

<style scoped>
.task-mobile-log-footer {
  padding-bottom: calc(8px + env(safe-area-inset-bottom, 0px));
}
</style>
