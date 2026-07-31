<template>
  <section class="mx-auto max-w-3xl">
    <header class="flex items-start gap-4 border-b border-light pb-5">
      <div class="h-12 w-12 shrink-0 rounded-lg bg-primary text-[20px] font-bold text-white flex-center">
        N
      </div>
      <div class="min-w-0">
        <h1 class="m-0 text-[20px] font-bold leading-7 text-default">NiuPanel</h1>
        <p class="mt-1 text-[12px] leading-5 text-secondary">
          轻量、可扩展的任务与服务管理面板
        </p>
      </div>
    </header>

    <dl class="grid border-b border-light sm:grid-cols-3">
      <div class="border-b border-light py-4 sm:border-b-0 sm:border-r sm:pr-4">
        <dt class="text-[11px] font-semibold text-muted">Core 版本</dt>
        <dd class="mt-1 font-mono text-[16px] font-bold text-default">
          {{ formatVersion(currentVersion) }}
        </dd>
      </div>
      <div class="border-b border-light py-4 sm:border-b-0 sm:border-r sm:px-4">
        <dt class="text-[11px] font-semibold text-muted">Web 版本</dt>
        <dd class="mt-1 font-mono text-[16px] font-bold text-default">
          {{ formatVersion(FRONTEND_VERSION) }}
        </dd>
      </div>
      <div class="py-4 sm:pl-4">
        <dt class="text-[11px] font-semibold text-muted">技术架构</dt>
        <dd class="mt-1 text-[16px] font-bold text-default">Rust + Vue 3</dd>
      </div>
    </dl>

    <div class="border-b border-light py-5">
      <div class="mb-3 flex items-center justify-between gap-3">
        <div>
          <h2 class="m-0 text-[14px] font-bold text-default">更新通道</h2>
          <p class="mt-1 text-[11px] text-muted">
            正式通道使用稳定发布，预览通道包含 prerelease 版本。
          </p>
        </div>
        <span
          class="shrink-0 rounded-md px-2 py-1 text-[11px] font-semibold"
          :class="
            updateChannel === 'preview'
              ? 'bg-amber-500/10 text-amber-600 dark:text-amber-300'
              : 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-300'
          "
        >
          {{ updateChannel === "preview" ? "预览版本" : "正式版本" }}
        </span>
      </div>
      <el-radio-group
        :model-value="updateChannel"
        :disabled="updatingUpdateChannel || checkingUpdate"
        class="update-channel-group w-full"
        @change="emit('update-channel', $event as UpdateChannel)"
      >
        <el-radio-button label="stable" value="stable">正式</el-radio-button>
        <el-radio-button label="preview" value="preview">预览</el-radio-button>
      </el-radio-group>
    </div>

    <div class="flex flex-col gap-2 pt-5 sm:flex-row sm:flex-wrap">
      <el-button
        type="primary"
        class="!mx-0 !h-10 !rounded-md sm:flex-1"
        :loading="checkingUpdate"
        @click="emit('check-update')"
      >
        <span class="i-ep-search mr-2 text-[14px]"></span>
        检查 Core 更新
      </el-button>
      <el-button
        plain
        class="!mx-0 !h-10 !rounded-md sm:flex-1"
        :loading="uploadingUpdate"
        @click="emit('upload')"
      >
        <span class="i-ep-upload mr-2 text-[14px]"></span>
        上传 Core 包
      </el-button>
      <a
        href="https://github.com/wyourname/NiuPanel"
        target="_blank"
        rel="noreferrer"
        class="no-underline sm:flex-1"
      >
        <el-button class="!mx-0 !h-10 !w-full !rounded-md">
          <span class="i-carbon-logo-github mr-2 text-[14px]"></span>
          GitHub
        </el-button>
      </a>
    </div>
  </section>
</template>

<script setup lang="ts">
import type { UpdateChannel } from "@/types";
import { FRONTEND_VERSION, formatVersion } from "@/version";

defineProps<{
  checkingUpdate: boolean;
  currentVersion: string;
  updateChannel: UpdateChannel;
  updatingUpdateChannel: boolean;
  uploadingUpdate: boolean;
}>();

const emit = defineEmits<{
  (event: "check-update"): void;
  (event: "update-channel", channel: UpdateChannel): void;
  (event: "upload"): void;
}>();
</script>

<style scoped>
.update-channel-group :deep(.el-radio-button) {
  flex: 1;
}

.update-channel-group :deep(.el-radio-button__inner) {
  width: 100%;
}
</style>
