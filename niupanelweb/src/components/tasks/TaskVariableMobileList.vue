<template>
  <div
    class="flex-1 overflow-y-auto px-4 py-4 flex flex-col gap-3 bg-card"
    v-loading="loading"
  >
    <div
      v-for="(item, index) in variables"
      :key="item.id ?? `mobile-${index}`"
      class="mobile-variable-card"
      :class="{
        'is-dragging': dragIndex === index,
        'is-drop-target': dragOverIndex === index && dragIndex !== index,
        'is-selected': item.id !== null && selectedIds.includes(item.id),
      }"
      :data-variable-index="index"
    >
      <div class="flex items-center gap-3">
        <button
          type="button"
          class="drag-handle shrink-0"
          :class="{ 'is-disabled': !canSort }"
          title="拖拽排序"
          @pointerdown="emit('start-drag', index, $event)"
        >
          <div class="i-ep-rank text-sm"></div>
        </button>

        <el-checkbox
          :model-value="item.id !== null && selectedIds.includes(item.id)"
          :disabled="item.id === null"
          size="large"
          class="!mr-0"
          @change="handleSelectionChange(item, $event)"
        />

        <span class="id-badge">#{{ item.id ?? "NEW" }}</span>

        <div class="ml-auto flex items-center gap-2">
          <el-switch
            v-model="item.enabled"
            size="small"
            :loading="item.statusLoading"
            @change="emit('status-change', item, Boolean($event))"
          />
          <button
            class="p-1 text-muted hover:text-rose-500 transition-colors"
            @click="emit('delete', item, index)"
          >
            <div class="i-ep-delete text-lg"></div>
          </button>
        </div>
      </div>

      <el-input v-model="item.key" placeholder="变量名 (Key)" class="font-mono" />

      <el-input
        v-model="item.value"
        type="textarea"
        :rows="4"
        placeholder="变量值 (Value)"
        class="font-mono"
      />

      <el-input
        v-model="item.remarks"
        placeholder="备注"
        size="small"
        class="italic"
      />
    </div>

    <div
      v-if="variables.length === 0"
      class="flex flex-col items-center justify-center py-20 text-muted opacity-50"
    >
      <div class="i-ep-box text-5xl mb-2"></div>
      <span>暂无环境变量</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { CheckboxValueType } from "element-plus";
import type { TaskVariableRow } from "../../composables/taskVariableEditorHelpers";

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

const handleSelectionChange = (
  row: TaskVariableRow,
  value: CheckboxValueType,
) => {
  if (row.id === null) return;
  emit("selection-change", row.id, Boolean(value));
};
</script>

<style scoped>
.mobile-variable-card {
  display: flex;
  flex-direction: column;
  gap: 12px;
  border: 1px solid var(--el-border-color-light);
  border-radius: 16px;
  background: var(--el-bg-color);
  padding: 14px;
  transition: box-shadow 0.2s ease, border-color 0.2s ease, opacity 0.2s ease;
}

.mobile-variable-card.is-selected {
  border-color: rgba(59, 130, 246, 0.35);
  box-shadow: 0 0 0 1px rgba(59, 130, 246, 0.16);
}

.mobile-variable-card.is-dragging {
  opacity: 0.72;
}

.mobile-variable-card.is-drop-target {
  box-shadow: inset 0 0 0 1px rgba(59, 130, 246, 0.35);
}

.id-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 40px;
  height: 22px;
  padding: 0 7px;
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
</style>
