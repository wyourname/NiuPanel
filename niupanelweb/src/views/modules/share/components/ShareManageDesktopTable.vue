<template>
  <div class="flex-1 overflow-hidden">
    <el-table
      :data="data"
      height="100%"
      style="width: 100%"
      stripe
      v-loading="loading"
      :header-cell-style="{
        background: 'var(--bg-base)',
        color: 'var(--text-secondary)',
        fontWeight: '600',
      }"
      :row-style="{ background: 'var(--bg-card)' }"
    >
      <el-table-column label="中转信息 / 资源链接" min-width="350">
        <template #default="{ row }">
          <div class="flex flex-col justify-center gap-1.5 py-1">
            <div class="flex items-center gap-2">
              <span class="font-bold text-sm text-default truncate max-w-[200px]">
                {{ getShareDisplayName(row) }}
              </span>
              <el-tag
                v-if="isExpired(row)"
                type="danger"
                size="small"
                effect="light"
                round
                class="border-0 font-medium"
              >
                已过期
              </el-tag>
              <el-tag
                v-else-if="isDeleteOnDownload(row)"
                type="warning"
                size="small"
                effect="light"
                round
                class="border-0 font-medium"
              >
                阅后即焚
              </el-tag>
              <el-tag
                v-else
                type="success"
                size="small"
                effect="light"
                round
                class="border-0 font-medium"
              >
                正常
              </el-tag>
            </div>

            <div
              class="flex items-center gap-2 text-xs text-[#8C8C8C] font-mono group cursor-pointer"
              @click="emit('copy-link', row.token)"
            >
              <span class="opacity-60 shrink-0">#{{ row.token }}</span>
              <span
                v-if="row.size"
                class="opacity-40 shrink-0 border-l border-base pl-2"
              >
                {{ formatShareSize(row.size) }}
              </span>
              <div
                class="i-ep-copy-document text-[10px] text-muted opacity-0 group-hover:opacity-100 transition-opacity"
                title="复制链接"
              ></div>
            </div>
          </div>
        </template>
      </el-table-column>

      <el-table-column label="剩余下载" width="120">
        <template #default="{ row }">
          <div class="flex flex-col gap-0.5">
            <span class="font-mono text-xs text-default font-bold">
              {{
                getDownloadsRemaining(row) === -1
                  ? "无限制"
                  : getDownloadsRemaining(row)
              }}
            </span>
          </div>
        </template>
      </el-table-column>

      <el-table-column label="上传时间" width="160">
        <template #default="{ row }">
          <span class="text-xs text-secondary">
            {{ row.uploaded_at ? formatShareDate(row.uploaded_at) : "-" }}
          </span>
        </template>
      </el-table-column>

      <el-table-column label="操作" width="200" align="right" fixed="right">
        <template #default="{ row }">
          <div class="flex justify-end gap-2 pr-2">
            <el-button
              size="small"
              type="warning"
              link
              @click="emit('update-content', row)"
            >
              <div class="i-ep-refresh mr-1"></div>
              更新
            </el-button>

            <el-tooltip content="复制链接" placement="top" :show-after="500">
              <el-button
                size="small"
                link
                class="!p-1"
                @click="emit('copy-link', row.token)"
              >
                <div
                  class="i-ep-link text-lg text-gray-400 hover:text-primary transition-colors"
                ></div>
              </el-button>
            </el-tooltip>

            <el-tooltip content="编辑" placement="top" :show-after="500">
              <el-button
                size="small"
                link
                class="!p-1"
                @click="emit('edit', row)"
              >
                <div
                  class="i-ep-edit text-lg text-gray-400 hover:text-primary transition-colors"
                ></div>
              </el-button>
            </el-tooltip>

            <el-tooltip content="删除" placement="top" :show-after="500">
              <el-button
                size="small"
                type="danger"
                link
                class="!p-1"
                @click="emit('delete', row)"
              >
                <div class="i-ep-delete text-lg"></div>
              </el-button>
            </el-tooltip>
          </div>
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<script setup lang="ts">
import type { StationFile } from "@/types";
import {
  formatShareDate,
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
</script>

<style scoped>
:deep(.el-table) {
  --el-table-border-color: var(--border-light);
  border-radius: 12px;
  overflow: hidden;
}

:deep(.el-table__row) {
  transition: background-color 0.2s ease;
}

:deep(.el-table__row):hover {
  background-color: var(--bg-soft) !important;
}
</style>
