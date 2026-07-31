<template>
  <div class="task-variable-editor h-full flex flex-col bg-card overflow-hidden">
    <div
      v-if="!hideToolbar"
      class="flex justify-between items-center px-4 py-3 border-b border-base shrink-0"
    >
      <div class="flex items-center gap-2 min-w-0">
        <div class="i-ep-key text-primary text-lg"></div>
        <h3 class="text-sm font-bold text-default">变量管理</h3>
        <el-tag
          size="small"
          type="info"
          effect="plain"
          class="font-mono"
        >
          {{ variables.length }}
        </el-tag>
      </div>
      <div class="flex gap-2">
        <el-tooltip content="源码模式 (批量编辑)" placement="top">
          <el-button size="small" @click="openSourceMode">
            <div
              class="i-ep-document"
              :class="{ 'mr-1': !appStore.isMobile }"
            ></div>
            <span v-if="!appStore.isMobile">源码模式</span>
          </el-button>
        </el-tooltip>
        <el-button type="primary" size="small" @click="addVariableRow">
          <div class="i-ep-plus" :class="{ 'mr-1': !appStore.isMobile }"></div>
          <span v-if="!appStore.isMobile">新增</span>
        </el-button>
      </div>
    </div>

    <TaskVariableDesktopList
      v-if="!appStore.isMobile"
      :can-sort="canSort"
      :drag-index="dragIndex"
      :drag-over-index="dragOverIndex"
      :loading="loading"
      :selected-ids="selectedIds"
      :variables="variables"
      @delete="handleRowDelete"
      @selection-change="updateSelection"
      @start-drag="startDrag"
      @status-change="handleStatusChange"
    />

    <TaskVariableMobileList
      v-else
      :can-sort="canSort"
      :drag-index="dragIndex"
      :drag-over-index="dragOverIndex"
      :loading="loading"
      :selected-ids="selectedIds"
      :variables="variables"
      @delete="handleRowDelete"
      @selection-change="updateSelection"
      @start-drag="startDrag"
      @status-change="handleStatusChange"
    />

    <div
      class="flex shrink-0 flex-wrap items-center justify-between gap-3 border-t border-base bg-base px-4 py-3"
    >
      <div class="flex items-center min-w-0">
        <transition name="el-fade-in-linear" mode="out-in">
          <div
            v-if="selectedIds.length === 0"
            class="text-xs text-muted font-medium"
          >
            共 {{ variables.length }} 个变量
          </div>
          <div v-else class="flex items-center gap-1 text-xs">
            <span class="font-bold text-primary mr-2">已选 {{ selectedIds.length }}</span>
            <el-button link type="danger" size="small" @click="batchDelete">删除</el-button>
            <div class="w-px h-3 bg-border-base mx-1"></div>
            <el-button link type="warning" size="small" @click="batchToggle(false)">禁用</el-button>
            <div class="w-px h-3 bg-border-base mx-1"></div>
            <el-button link type="success" size="small" @click="batchToggle(true)">启用</el-button>
          </div>
        </transition>
      </div>

      <div class="flex gap-2 ml-auto">
        <el-button size="small" @click="cancel">取消</el-button>
        <el-button
          type="primary"
          size="small"
          @click="saveAll"
          :loading="saving"
          :disabled="!hasChanges"
          class="!px-6"
        >
          <div class="i-ep-check mr-1"></div>
          保存全部
        </el-button>
      </div>
    </div>

    <el-dialog
      v-model="sourceModeVisible"
      title="源码模式"
      width="90%"
      append-to-body
      destroy-on-close
    >
      <div
        class="mb-3 text-xs text-muted bg-base p-3 rounded-lg flex items-start gap-2 leading-relaxed"
      >
        <div class="i-ep-info-filled text-primary shrink-0 mt-0.5"></div>
        <span>
          您可以直接粘贴 <b>KEY=VALUE</b>。含空格、换行或等号的值请使用双引号，
          例如 <b>CONFIG="line1\nline=2"</b>；从列表生成的内容可无损往返。
        </span>
      </div>
      <el-input
        v-model="rawSource"
        type="textarea"
        :rows="15"
        placeholder="APP_ID=12345&#10;SECRET=abcde..."
        class="font-mono text-sm"
      />
      <template #footer>
        <el-button @click="sourceModeVisible = false">取消</el-button>
        <el-button type="primary" @click="applySource">解析并应用</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, watch } from "vue";
import { useAppStore } from "../../stores/app";
import type { TaskVariableRow } from "../../composables/taskVariableEditorHelpers";
import { useTaskVariableEditorRows } from "../../composables/useTaskVariableEditorRows";
import { useTaskVariableReorder } from "../../composables/useTaskVariableReorder";
import TaskVariableDesktopList from "./TaskVariableDesktopList.vue";
import TaskVariableMobileList from "./TaskVariableMobileList.vue";

const appStore = useAppStore();

const props = withDefaults(
  defineProps<{
    hideToolbar?: boolean;
    taskId: number | string;
  }>(),
  {
    hideToolbar: false,
  },
);

const emit = defineEmits<{
  (e: "success"): void;
  (e: "cancel"): void;
}>();

const taskId = computed(() => Number(props.taskId));

const {
  addVariableRow,
  applySource,
  batchDelete,
  batchToggle,
  cancelNewVariable,
  fetchVariables,
  handleDelete,
  handleStatusChange,
  hasChanges,
  hasDraftRows,
  loading,
  openSourceMode,
  rawSource,
  saveAll,
  saving,
  selectedIds,
  sourceModeVisible,
  updateSelection,
  variables,
} = useTaskVariableEditorRows({
  onSuccess: () => emit("success"),
  taskId,
});

const { canSort, dragIndex, dragOverIndex, startDrag } = useTaskVariableReorder({
  hasDraftRows,
  loading,
  saving,
  taskId,
  variables,
});

const handleRowDelete = (row: TaskVariableRow, index: number) => {
  if (row.isNew) {
    cancelNewVariable(index);
    return;
  }

  handleDelete(row);
};

const cancel = () => emit("cancel");

watch(
  taskId,
  (newId) => {
    if (newId) {
      fetchVariables();
    }
  },
  { immediate: true },
);
</script>
