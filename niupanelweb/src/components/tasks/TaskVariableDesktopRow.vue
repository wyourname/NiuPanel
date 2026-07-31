<template>
  <div
    class="desktop-table__row"
    :class="{
      'is-dragging': isDragging,
      'is-drop-target': isDropTarget,
    }"
    :data-variable-index="index"
  >
    <div class="desktop-table__cell desktop-table__cell--drag">
      <button
        type="button"
        class="drag-handle"
        :class="{ 'is-disabled': !canSort }"
        title="拖拽排序"
        @pointerdown="emit('start-drag', index, $event)"
      >
        <div class="i-ep-rank text-sm"></div>
      </button>
    </div>

    <div class="desktop-table__cell desktop-table__cell--selection">
      <el-checkbox
        :model-value="row.id !== null && selectedIds.includes(row.id)"
        :disabled="row.id === null"
        @change="handleSelectionChange"
      />
    </div>

    <div class="desktop-table__cell desktop-table__cell--id">
      <div class="flex flex-col items-start gap-1">
        <span class="id-badge">#{{ row.id ?? "NEW" }}</span>
        <span
          v-if="(row.task_ids?.length ?? 0) > 1"
          class="rounded bg-amber-500/10 px-1 text-[9px] font-semibold text-amber-600 dark:text-amber-400"
          :title="`该变量被 ${row.task_ids?.length} 个任务共享`"
        >
          共享 {{ row.task_ids?.length }}
        </span>
      </div>
    </div>

    <div class="desktop-table__cell desktop-table__cell--key">
      <el-input
        v-model="row.key"
        placeholder="KEY"
        class="font-mono font-bold custom-input-filled w-full min-w-0"
      />
    </div>

    <div class="desktop-table__cell desktop-table__cell--value">
      <el-input
        v-model="row.value"
        type="textarea"
        :rows="2"
        placeholder="VALUE"
        class="font-mono text-xs custom-input-filled w-full min-w-0"
      />
    </div>

    <div class="desktop-table__cell desktop-table__cell--remarks">
      <el-input
        v-model="row.remarks"
        placeholder="备注..."
        class="custom-input-filled italic text-muted w-full min-w-0"
      />
    </div>

    <div class="desktop-table__cell desktop-table__cell--state">
      <el-switch
        v-model="row.enabled"
        size="small"
        :loading="row.statusLoading"
        @change="emit('status-change', row, Boolean($event))"
      />
    </div>

    <div class="desktop-table__cell desktop-table__cell--actions">
      <button
        class="rounded-md p-1.5 text-muted transition-colors hover:bg-rose-500/10 hover:text-rose-500"
        @click="emit('delete', row, index)"
      >
        <div class="i-ep-delete text-base"></div>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { CheckboxValueType } from "element-plus";
import type { TaskVariableRow } from "../../composables/taskVariableEditorHelpers";

const props = defineProps<{
  canSort: boolean;
  index: number;
  isDragging: boolean;
  isDropTarget: boolean;
  row: TaskVariableRow;
  selectedIds: number[];
}>();

const emit = defineEmits<{
  (event: "delete", row: TaskVariableRow, index: number): void;
  (event: "selection-change", id: number, checked: boolean): void;
  (event: "start-drag", index: number, pointerEvent: PointerEvent): void;
  (event: "status-change", row: TaskVariableRow, enabled: boolean): void;
}>();

const handleSelectionChange = (value: CheckboxValueType) => {
  if (props.row.id === null) return;
  emit("selection-change", props.row.id, Boolean(value));
};
</script>

<style scoped>
.desktop-table__row {
  display: grid;
  grid-template-columns: 30px 36px 52px minmax(0, 1.25fr) minmax(0, 2fr) minmax(0, 0.55fr) 66px 52px;
  align-items: center;
  gap: 0;
  padding: 0 8px;
  min-height: 74px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  background: var(--el-bg-color);
  transition: background-color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease;
}

.desktop-table__row:hover {
  background: var(--el-fill-color-light);
}

.desktop-table__row.is-dragging {
  opacity: 0.72;
}

.desktop-table__row.is-drop-target {
  box-shadow: inset 0 0 0 1px rgba(59, 130, 246, 0.35);
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

.desktop-table__cell--key,
.desktop-table__cell--value,
.desktop-table__cell--remarks {
  overflow: hidden;
}

.id-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 36px;
  height: 22px;
  padding: 0 6px;
  border-radius: 999px;
  background: var(--el-fill-color-light);
  color: var(--el-text-color-secondary);
  font-size: 10px;
  line-height: 1;
  font-family: ui-monospace, SFMono-Regular, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
}

.drag-handle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 6px;
  background: var(--el-fill-color-blank);
  color: var(--el-text-color-secondary);
  cursor: grab;
  touch-action: none;
  transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease;
}

.drag-handle:hover {
  background: var(--el-fill-color-light);
  color: var(--el-color-primary);
  border-color: var(--el-color-primary-light-7);
}

.drag-handle:active {
  cursor: grabbing;
}

.drag-handle.is-disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.custom-input-filled :deep(.el-input__wrapper),
.custom-input-filled :deep(.el-textarea__inner) {
  box-shadow: none !important;
  background-color: transparent;
  border: 1px solid transparent;
  transition: all 0.2s;
  padding: 4px 8px;
}

.custom-input-filled :deep(.el-input__wrapper:hover),
.custom-input-filled :deep(.el-textarea__inner:hover) {
  background-color: var(--el-fill-color-light);
}

.custom-input-filled :deep(.el-input__wrapper.is-focus),
.custom-input-filled :deep(.el-textarea__inner:focus) {
  background-color: var(--el-bg-color);
  border-color: var(--el-color-primary-light-5);
  box-shadow: 0 0 0 1px var(--el-color-primary-light-8) !important;
}
</style>
