<template>
  <div class="min-h-0 flex-1 overflow-y-auto custom-scrollbar">
    <div
      v-if="data.length === 0 && !loading"
      class="h-60 flex flex-center text-muted flex-col opacity-50"
    >
      <div class="i-ep-share text-5xl mb-2"></div>
      <span>暂无资源</span>
    </div>

    <div v-else class="divide-y divide-light/70 border-b border-light/70 bg-card">
      <article
        v-for="item in data"
        :key="item.token"
        class="p-4 transition-colors hover:bg-soft/30"
        @click="emit('edit', item)"
      >
        <div class="flex justify-between items-start mb-3">
          <div class="flex flex-col min-w-0">
            <span class="font-bold text-default truncate">
              {{ getShareDisplayName(item) }}
            </span>
            <span class="text-[10px] text-muted font-mono mt-0.5">
              Token: {{ item.token }}
            </span>
          </div>
          <el-tag
            v-if="isExpired(item)"
            type="danger"
            size="small"
            effect="plain"
          >
            过期
          </el-tag>
          <el-tag
            v-else-if="isDeleteOnDownload(item)"
            type="warning"
            size="small"
            effect="plain"
          >
            阅后即焚
          </el-tag>
          <el-tag v-else type="success" size="small" effect="plain">
            正常
          </el-tag>
        </div>

        <div
          class="mb-4 flex cursor-pointer items-center justify-between rounded-md border border-base/30 bg-base/50 p-3 transition-colors active:bg-base copy-badge"
          @click.stop="handleCopy(item.token)"
        >
          <div class="flex items-center gap-2 overflow-hidden">
            <div class="i-ep-link text-primary shrink-0"></div>
            <span class="truncate font-mono text-xs text-secondary">
              /share/{{ item.token }}
            </span>
          </div>
          <div class="i-ep-copy-document text-primary text-sm ml-2 shrink-0"></div>
        </div>

        <div class="flex flex-col gap-4">
          <div class="flex justify-between items-center text-xs">
            <div class="flex items-center gap-1.5 text-muted">
              <div class="i-ep-guide opacity-60"></div>
              <span>剩余次数 / 大小</span>
            </div>
            <div class="font-bold text-default">
              {{
                getDownloadsRemaining(item) === -1
                  ? "∞"
                  : getDownloadsRemaining(item)
              }}
              <span class="text-muted font-normal">
                / {{ formatShareSize(item.size) }}
              </span>
            </div>
          </div>

          <div class="flex items-center gap-2">
            <el-button
              size="small"
              type="warning"
              class="!rounded-md"
              @click.stop="emit('update-content', item)"
            >
              <div class="i-ep-refresh mr-1"></div>
              更新
            </el-button>
            <el-button
              size="small"
              class="flex-1 !rounded-md"
              @click.stop="emit('edit', item)"
            >
              <div class="i-ep-edit mr-1"></div>
              编辑
            </el-button>
            <el-button
              size="small"
              type="danger"
              class="flex-1 !rounded-md"
              @click.stop="emit('delete', item)"
            >
              <div class="i-ep-delete mr-1"></div>
              删除
            </el-button>
          </div>
        </div>
      </article>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useHaptics } from "../../../../composables/useHaptics";
import type { StationFile } from "@/types";
import {
  formatShareSize,
  getDownloadsRemaining,
  getShareDisplayName,
  isDeleteOnDownload,
  isExpired,
} from "./shareManageTableUtils";

defineProps<{
  data: StationFile[];
  loading?: boolean;
}>();

const emit = defineEmits<{
  (event: "copy-link", token: string): void;
  (event: "delete", row: StationFile): void;
  (event: "edit", row: StationFile): void;
  (event: "update-content", row: StationFile): void;
}>();

const haptics = useHaptics();

const handleCopy = (token: string) => {
  haptics.impact();
  emit("copy-link", token);
};
</script>

<style scoped>
.copy-badge {
  transition: all 0.2s ease;
}

.copy-badge:hover {
  background-color: var(--bg-soft) !important;
  border-color: var(--el-color-primary-light-5) !important;
}
</style>
