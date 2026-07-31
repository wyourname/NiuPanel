<template>
  <el-dialog
    v-model="visible"
    title="选择文件"
    :width="isMobile ? '95%' : '600px'"
    class="om-dialog"
  >
    <div class="space-y-3">
      <div
        class="flex items-center justify-between rounded-lg border border-light bg-subtle p-2.5"
      >
        <div class="flex items-center gap-2 overflow-hidden">
          <div class="i-ep-folder-opened text-primary"></div>
          <span class="text-[11px] font-mono font-bold text-default truncate">
            {{ currentPath }}
          </span>
        </div>
        <el-button v-if="currentPath !== '/'" size="small" circle @click="$emit('back')">
          <div class="i-ep-back"></div>
        </el-button>
      </div>
      <div class="max-h-[400px] overflow-hidden overflow-y-auto rounded-lg border border-light">
        <el-table
          :data="serverFiles"
          size="small"
          class="cursor-pointer om-table"
          highlight-current-row
          @row-click="$emit('rowClick', $event)"
        >
          <el-table-column width="40" align="center">
            <template #default="{ row }">
              <div
                :class="[
                  row.is_dir ? 'i-ep-folder text-blue-500' : getFileIconClass(row.name),
                  'text-base',
                ]"
              ></div>
            </template>
          </el-table-column>
          <el-table-column prop="name" label="名称">
            <template #default="{ row }">
              <span
                :class="[
                  'font-mono text-[11px] font-bold',
                  row.is_dir ? 'text-default' : 'text-muted',
                ]"
              >
                {{ row.name }}
              </span>
            </template>
          </el-table-column>
          <el-table-column
            v-if="!isMobile"
            prop="size"
            label="大小"
            width="80"
            align="right"
          >
            <template #default="{ row }">
              <span class="font-mono text-[10px] text-muted">
                {{ row.is_dir ? "--" : formatSize(row.size) }}
              </span>
            </template>
          </el-table-column>
        </el-table>
      </div>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import type { FileItem } from "@/types";

defineProps<{
  currentPath: string;
  formatSize: (bytes: number) => string;
  getFileIconClass: (name: string) => string;
  isMobile: boolean;
  serverFiles: FileItem[];
}>();

defineEmits<{
  (e: "back"): void;
  (e: "rowClick", row: FileItem): void;
}>();

const visible = defineModel<boolean>("visible", { required: true });
</script>

<style scoped>
.om-table :deep(.el-table__row) {
  transition: background-color 0.2s;
}

.om-table :deep(.el-table__body tr.current-row > td) {
  background-color: var(--el-color-primary-light-9) !important;
}

@media (max-width: 768px) {
  .om-dialog :deep(.el-dialog__body) {
    padding: 15px !important;
  }

  .om-table :deep(.el-table__cell) {
    padding: 8px 0 !important;
  }

  .om-dialog :deep(.el-dialog__header) {
    margin-right: 0;
    padding-bottom: 10px;
  }
}
</style>
