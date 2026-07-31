<template>
  <div v-if="stationStats && !stationStats.isConfigured"
    class="flex flex-1 flex-col items-center justify-center p-5 text-center sm:p-8">
    <div class="accent-subtle mb-4 h-12 w-12 rounded-md flex-center">
      <div class="i-ep-connection text-2xl"></div>
    </div>
    <h3 class="mb-1 text-[16px] font-bold text-default">
      云端资源未配置
    </h3>
    <p class="mb-5 max-w-sm text-xs leading-5 text-muted">
      请初始化与中转站的连接，以管理云端分发内容并监控全局存储状态。
    </p>
    <div class="w-full max-w-md border-t border-light pt-5 text-left">
      <el-form :model="configForm" label-position="top">
        <el-form-item>
          <template #label>
            <span class="label-xs">节点地址 URL</span>
          </template>
          <el-input
            v-model="configForm.url"
            placeholder="https://station.example.com"
            class="modern-input"
          />
        </el-form-item>
        <el-form-item>
          <template #label>
            <span class="label-xs">管理员密钥 Token</span>
          </template>
          <el-input
            v-model="configForm.token"
            type="password"
            show-password
            placeholder="Auth credentials"
            class="modern-input"
          />
        </el-form-item>
        <el-button
          type="primary"
          class="mt-2 w-full !h-9 font-bold"
          :loading="savingConfig"
          @click="emit('save-config')"
        >
          建立连接
        </el-button>
      </el-form>
    </div>
  </div>

  <template v-else>
    <div
      v-if="stationStats"
      class="z-10 flex items-center justify-between border-b border-light bg-card px-4 py-3 md:px-6"
    >
      <div class="flex items-center gap-8 flex-1">
        <div class="flex flex-col gap-1.5 min-w-[160px]">
          <div class="flex justify-between items-end">
            <span class="text-[9px] font-bold text-muted">空间占用率</span>
            <span class="text-[10px] font-mono font-bold text-primary">
              {{ stationStats.usagePercent }}
            </span>
          </div>
          <div class="h-1.5 w-full bg-base rounded-full overflow-hidden border border-light">
            <div
              class="bg-primary h-full transition-all duration-1000"
              :style="{ width: stationStats.usagePercent }"
            ></div>
          </div>
        </div>
        <div class="hidden sm:flex flex-col gap-0.5">
          <span class="text-[9px] font-bold text-muted">容量分配</span>
          <span class="text-[10px] font-mono font-bold text-secondary">
            {{ formatSize(stationStats.currentUsageBytes) }}
            <span class="opacity-30">/</span>
            {{ formatSize(stationStats.maxUsageBytes) }}
          </span>
        </div>
      </div>
      <div class="flex items-center gap-4">
        <div class="hidden font-mono text-[9px] font-bold text-muted opacity-60 md:block">
          节点：{{ configForm.url || "默认" }}
        </div>
        <button class="btn-icon !w-8 !h-8 config-trigger" @click="emit('configure')">
          <div class="i-ep-setting"></div>
        </button>
      </div>
    </div>

    <PullToRefresh
      :on-refresh="onRefresh"
      :disabled="!isMobile"
      class="flex-1 flex flex-col min-h-0"
    >
      <div class="flex-1 flex flex-col bg-subtle dark:bg-subtle min-h-0">
        <ShareManageTable
          :data="stationList"
          :loading="loadingList"
          @copy-link="emit('copy-link', $event)"
          @edit="emit('edit', $event)"
          @delete="emit('delete', $event)"
          @update-content="emit('update-content', $event)"
        />
      </div>
    </PullToRefresh>
  </template>
</template>

<script setup lang="ts">
import PullToRefresh from "@/components/common/PullToRefresh.vue";
import type {
  StationConfigPayload,
  StationFile,
  StationStats,
} from "@/types";
import ShareManageTable from "./ShareManageTable.vue";

defineProps<{
  configForm: StationConfigPayload;
  isMobile: boolean;
  loadingList: boolean;
  onRefresh: () => Promise<unknown> | unknown;
  savingConfig: boolean;
  stationList: StationFile[];
  stationStats: StationStats | null;
}>();

const emit = defineEmits<{
  (event: "configure"): void;
  (event: "copy-link", token: string): void;
  (event: "delete", row: StationFile): void;
  (event: "edit", row: StationFile): void;
  (event: "save-config"): void;
  (event: "update-content", row: StationFile): void;
}>();

const formatSize = (bytes: number) => {
  if (!bytes) return "0 B";
  const unit = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const index = Math.floor(Math.log(bytes) / Math.log(unit));
  return `${parseFloat((bytes / Math.pow(unit, index)).toFixed(2))} ${sizes[index]}`;
};
</script>

<style scoped>
.modern-input :deep(.el-input__wrapper) {
  border-radius: 6px !important;
  padding: 8px 12px;
  background-color: var(--bg-soft) !important;
  box-shadow: none !important;
  border: 1px solid var(--border-light) !important;
  transition: border-color 0.2s, background-color 0.2s;
}

.modern-input :deep(.el-input__wrapper):hover {
  border-color: var(--el-color-primary-light-5) !important;
}

.modern-input :deep(.el-input__wrapper).is-focus {
  border-color: var(--el-color-primary) !important;
  background-color: var(--bg-card) !important;
  box-shadow: 0 0 0 1px var(--el-color-primary) !important;
}

.config-trigger {
  color: var(--text-secondary);
  transition: color 0.2s;
}

.config-trigger:hover {
  color: var(--el-color-primary);
}
</style>
