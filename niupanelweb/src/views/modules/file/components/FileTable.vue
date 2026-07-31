<template>
  <el-table
    ref="tableRef"
    :data="data"
    height="100%"
    style="width: 100%"
    highlight-current-row
    row-key="path"
    lazy
    :load="loadNode"
    :tree-props="{ children: 'children', hasChildren: 'is_dir' }"
    @row-click="handleRowClick"
    @selection-change="handleSelectionChange"
    @row-contextmenu="handleContextMenu"
    :header-cell-style="{
      background: 'transparent',
      color: 'var(--el-text-color-secondary)',
      fontWeight: 'bold',
      fontSize: '10px',
      borderBottom: '1px solid var(--el-border-color-lighter)',
    }"
    :row-style="{ background: 'transparent' }"
    class="file-table"
  >
    <el-table-column type="selection" width="50" />

    <el-table-column prop="name" label="名称" min-width="300" sortable>
      <template #default="{ row }">
        <FileNameCell :get-icon-class="getIconClass" :row="row" />
      </template>
    </el-table-column>

    <el-table-column label="类型" width="160" v-if="!isMobile">
      <template #default="{ row }">
        <span
          class="text-[10px] font-bold text-muted/60"
        >
          {{
            row.is_dir ? "目录" : row.name.split(".").pop() || "文件"
          }}
        </span>
      </template>
    </el-table-column>

    <el-table-column prop="size" label="大小" width="120" align="right" sortable v-if="!isMobile">
      <template #default="{ row }">
        <span
          v-if="!row.is_dir"
          class="text-[11px] font-mono font-bold text-secondary"
          >{{ formatFileSize(row.size) }}</span
        >
        <span v-else class="text-muted/30 font-bold">-</span>
      </template>
    </el-table-column>

    <el-table-column prop="mtime" label="修改时间" width="140" sortable v-if="!isMobile">
      <template #default="{ row }">
        <span v-if="row.mtime" class="text-[11px] font-mono text-muted whitespace-nowrap">
          {{ new Date(row.mtime * 1000).toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) }}
        </span>
        <span v-else class="text-muted/30 font-bold">-</span>
      </template>
    </el-table-column>

    <el-table-column label="操作" width="140" align="right" fixed="right">
      <template #default="{ row }">
        <FileRowActions
          :is-editable="isEditable"
          :row="row"
          @delete="emit('delete', $event)"
          @download="emit('download', $event)"
          @move="emit('move', $event)"
          @open="emit('row-click', $event)"
          @rename="emit('rename', $event)"
        />
      </template>
    </el-table-column>
  </el-table>
</template>

<script setup lang="ts">
import { ref } from "vue";
import type {
  FileItem,
  FileTableRef,
} from "../../../../composables/useFileOperations";
import FileNameCell from "./FileNameCell.vue";
import FileRowActions from "./FileRowActions.vue";

type FileTableLoadNode = (
  row: FileItem,
  treeNode: unknown,
  resolve: (data: FileItem[]) => void,
) => Promise<void> | void;

type FileTableExpose = FileTableRef & {
  toggleRowExpansion?: (row: FileItem) => void;
};

const props = withDefaults(defineProps<{
  data: FileItem[];
  isEditable: (name: string) => boolean;
  getIconClass: (row: FileItem) => string;
  formatFileSize: (size: number) => string;
  isMobile?: boolean;
  loadNode: FileTableLoadNode;
}>(), {
  isMobile: false,
});

const emit = defineEmits<{
  (event: "row-click", row: FileItem): void;
  (event: "selection-change", rows: FileItem[]): void;
  (event: "context-menu", row: FileItem, column: unknown, mouseEvent: MouseEvent): void;
  (event: "download", row: FileItem): void;
  (event: "rename", row: FileItem): void;
  (event: "move", row: FileItem): void;
  (event: "delete", row: FileItem): void;
}>();
const tableRef = ref<FileTableExpose | null>(null);

const handleSelectionChange = (rows: FileItem[]) => {
  emit("selection-change", rows);
};

const handleContextMenu = (
  row: FileItem,
  column: unknown,
  event: MouseEvent,
) => {
  emit("context-menu", row, column, event);
};

const handleRowClick = (row: FileItem) => {
  // If it's a directory, toggle expansion natively
  if (row.is_dir) {
    tableRef.value?.toggleRowExpansion?.(row);
  } else {
    emit("row-click", row);
  }
};

defineExpose({
  clearSelection: () => tableRef.value?.clearSelection?.(),
  toggleRowSelection: (row: FileItem, selected: boolean) =>
    tableRef.value?.toggleRowSelection?.(row, selected),
  setCurrentRow: (row: FileItem | null) => tableRef.value?.setCurrentRow?.(row),
});
</script>

<style scoped>
.file-table :deep(.el-table__row) {
  transition: all 0.2s;
  height: 48px;
  cursor: pointer;
}
.file-table :deep(.el-table__row:hover) {
  background-color: var(--el-fill-color-light) !important;
}
.file-table :deep(.el-table__row.current-row) {
  background-color: var(--el-color-primary-light-9) !important;
}
.file-table :deep(.el-table__cell) {
  padding: 0;
  border-bottom: 1px solid var(--el-border-color-lighter) !important;
}
.file-table :deep(.el-table__inner-wrapper::before) {
  display: none;
}
.file-table :deep(.el-checkbox__inner) {
  border-radius: 4px;
}
.file-table :deep(.el-table__cell > .cell) {
  display: flex !important;
  align-items: center;
}
.file-table :deep(.el-table__expand-icon) {
  height: 22px;
  line-height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-right: 4px;
}
.file-table :deep(.el-table__placeholder) {
  display: inline-block;
  width: 24px;
}
</style>
