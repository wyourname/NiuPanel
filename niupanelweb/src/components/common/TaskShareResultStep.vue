<template>
  <div class="mt-5">
    <div class="text-center mb-5 text-success">
      <el-icon class="text-5xl mb-2"><CircleCheckFilled /></el-icon>
      <h3 class="text-lg font-bold">分享链接已生成</h3>
    </div>

    <div class="w-full">
      <p class="mb-2 font-medium text-default text-xs text-muted">
        下载链接:
      </p>
      <el-input :model-value="shareLink" readonly class="mb-4">
        <template #append>
          <el-button type="primary" @click="emit('copy')">复制链接</el-button>
        </template>
      </el-input>
    </div>

    <div
      class="rounded-lg border border-emerald-100 bg-emerald-50 p-4 text-sm text-emerald-600 dark:border-emerald-800/30 dark:bg-emerald-900/20 dark:text-emerald-400"
    >
      <div class="flex items-start gap-3">
        <div class="i-ep-info-filled mt-0.5 shrink-0"></div>
        <div class="flex-1">
          <p>
            请将此链接分享给接收方。对方可在“导入记录”中通过链接直接获取资源。
          </p>
          <p v-if="sharePassword" class="text-orange-500 font-bold mt-2">
            <el-icon class="mr-1"><Lock /></el-icon>
            此分享已加密，请同时提供提取密码。
          </p>
          <p v-if="burnAfterReading" class="text-rose-500 font-bold mt-1">
            <el-icon class="mr-1"><Warning /></el-icon>
            阅后即焚：资源被导入一次后将自动失效。
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { CircleCheckFilled, Lock, Warning } from "@element-plus/icons-vue";

defineProps<{
  burnAfterReading: boolean;
  shareLink: string;
  sharePassword: string;
}>();

const emit = defineEmits<{
  (event: "copy"): void;
}>();
</script>
