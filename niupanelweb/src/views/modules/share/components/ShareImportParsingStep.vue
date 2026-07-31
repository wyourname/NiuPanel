<template>
  <div
    class="py-8 flex flex-col items-center justify-center animate-fade-in min-h-[260px]"
  >
    <div class="relative w-24 h-24 flex items-center justify-center">
      <el-progress
        type="circle"
        :percentage="status?.progress || 0"
        :status="status?.state === 'error' ? 'exception' : ''"
        :width="96"
        :stroke-width="8"
      />
      <div
        v-if="status?.state !== 'error'"
        class="absolute inset-0 flex items-center justify-center"
      >
        <div class="i-ep-loading animate-spin text-2xl text-primary opacity-50"></div>
      </div>
    </div>
    <p class="mt-6 text-default font-bold text-base">
      {{ status?.message || "正在请求资源..." }}
    </p>
    <p class="text-xs text-muted mt-1">请稍候，正在同步远程任务配置</p>

    <div v-if="status?.state === 'error'" class="mt-6 flex gap-3">
      <el-button @click="emit('reset')">重新输入</el-button>
      <el-button type="primary" @click="emit('retry')">重试</el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ImportStatus } from "@/types";

defineProps<{
  status: ImportStatus | null;
}>();

const emit = defineEmits<{
  (event: "reset"): void;
  (event: "retry"): void;
}>();
</script>

<style scoped>
.animate-fade-in {
  animation: fadeIn 0.3s ease-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(5px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
