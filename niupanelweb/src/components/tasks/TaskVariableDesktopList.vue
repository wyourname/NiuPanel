<template>
  <div class="flex-1 min-h-0 overflow-y-auto overflow-x-hidden" v-loading="loading">
    <div class="desktop-table min-w-0">
      <div class="desktop-table__header">
        <div class="desktop-table__cell desktop-table__cell--drag"></div>
        <div class="desktop-table__cell desktop-table__cell--selection"></div>
        <div class="desktop-table__cell desktop-table__cell--id">ID</div>
        <div class="desktop-table__cell desktop-table__cell--key">键 (Key)</div>
        <div class="desktop-table__cell desktop-table__cell--value">值 (Value)</div>
        <div class="desktop-table__cell desktop-table__cell--remarks">备注</div>
        <div class="desktop-table__cell desktop-table__cell--state">状态</div>
        <div class="desktop-table__cell desktop-table__cell--actions">操作</div>
      </div>

      <TaskVariableDesktopRow
        v-for="(row, index) in variables"
        :key="row.id ?? `new-${index}`"
        :can-sort="canSort"
        :index="index"
        :is-dragging="dragIndex === index"
        :is-drop-target="dragOverIndex === index && dragIndex !== index"
        :row="row"
        :selected-ids="selectedIds"
        @delete="forwardDelete"
        @selection-change="forwardSelectionChange"
        @start-drag="forwardStartDrag"
        @status-change="forwardStatusChange"
      />

      <div
        v-if="variables.length === 0"
        class="py-20 flex flex-col items-center justify-center text-muted opacity-50"
      >
        <div class="i-ep-box text-5xl mb-2"></div>
        <span>暂无环境变量</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { TaskVariableRow } from "../../composables/taskVariableEditorHelpers";
import TaskVariableDesktopRow from "./TaskVariableDesktopRow.vue";

defineProps<{
  canSort: boolean;
  dragIndex: number | null;
  dragOverIndex: number | null;
  loading: boolean;
  selectedIds: number[];
  variables: TaskVariableRow[];
}>();

const emit = defineEmits<{
  (event: "delete", row: TaskVariableRow, index: number): void;
  (event: "selection-change", id: number, checked: boolean): void;
  (event: "start-drag", index: number, pointerEvent: PointerEvent): void;
  (event: "status-change", row: TaskVariableRow, enabled: boolean): void;
}>();

const forwardDelete = (row: TaskVariableRow, index: number) => {
  emit("delete", row, index);
};

const forwardSelectionChange = (id: number, checked: boolean) => {
  emit("selection-change", id, checked);
};

const forwardStartDrag = (index: number, pointerEvent: PointerEvent) => {
  emit("start-drag", index, pointerEvent);
};

const forwardStatusChange = (row: TaskVariableRow, enabled: boolean) => {
  emit("status-change", row, enabled);
};
</script>

<style scoped>
.desktop-table {
  min-width: 0;
}

.desktop-table__header {
  display: grid;
  grid-template-columns: 30px 36px 52px minmax(0, 1.25fr) minmax(0, 2fr) minmax(0, 0.55fr) 66px 52px;
  align-items: center;
  gap: 0;
  padding: 0 8px;
  position: sticky;
  top: 0;
  z-index: 2;
  min-height: 42px;
  background: var(--el-bg-color);
  border-bottom: 1px solid var(--el-border-color-light);
  color: var(--el-text-color-secondary);
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.01em;
}

.desktop-table__cell {
  min-width: 0;
  display: flex;
  align-items: center;
  padding: 10px 8px;
}

.desktop-table__cell--drag,
.desktop-table__cell--selection,
.desktop-table__cell--state,
.desktop-table__cell--actions {
  justify-content: center;
}

.desktop-table__cell--drag,
.desktop-table__cell--selection,
.desktop-table__cell--actions {
  padding-left: 2px;
  padding-right: 2px;
}

.desktop-table__cell--id {
  justify-content: flex-start;
  padding-left: 4px;
  padding-right: 4px;
}
</style>
