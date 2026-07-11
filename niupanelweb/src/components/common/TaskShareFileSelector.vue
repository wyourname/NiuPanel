<template>
  <div v-if="!currentEditingTask" class="task-master-view">
    <div v-if="tasks.length > 1" class="mb-4">
      <el-alert
        title="请为每个任务选择要分享的文件"
        type="info"
        :closable="false"
        show-icon
      />
    </div>

    <div class="border border-base rounded-lg bg-base p-2">
      <el-scrollbar max-height="400px">
        <div
          v-for="task in tasks"
          :key="task.id"
          class="bg-card border border-base rounded p-3 mb-2 flex justify-between items-center transition-all hover:border-primary/30"
          :class="{ 'border-l-4 border-l-primary': hasFiles(task.id) }"
        >
          <div class="flex-1 overflow-hidden mr-4">
            <div class="flex items-center mb-1.5">
              <span class="font-bold text-sm mr-2 text-default truncate">
                {{ task.name }}
              </span>
              <el-tag size="small" :type="hasFiles(task.id) ? 'primary' : 'info'">
                {{ getTaskFileCount(task.id) }} 文件
              </el-tag>
            </div>
            <div class="text-xs text-secondary truncate">
              {{
                hasFiles(task.id)
                  ? getTaskFilesPreviewText(task.id)
                  : "未关联文件"
              }}
            </div>
          </div>
          <div class="shrink-0">
            <el-button
              type="primary"
              size="small"
              :icon="Edit"
              @click="emit('edit', task)"
            >
              可勾选文件依赖
            </el-button>
          </div>
        </div>
      </el-scrollbar>
    </div>
  </div>

  <div v-else class="flex flex-col h-full">
    <div class="flex justify-between items-center pb-2 mb-2 border-b border-light">
      <el-button link :icon="ArrowLeft" @click="emit('stop-edit')">返回</el-button>
      <span class="font-bold text-default text-sm flex-1 text-center truncate">
        选择文件: {{ currentEditingTask.name }}
      </span>
      <el-button link type="primary" size="small" @click="emit('select-all')">
        全选
      </el-button>
    </div>

    <div class="px-2 pb-2 border-b border-light mb-2 flex items-center gap-2">
      <span class="text-xs text-secondary shrink-0">主执行文件:</span>
      <el-select
        :model-value="currentMainFile"
        placeholder="请先勾选文件"
        size="small"
        class="flex-1"
        @update:model-value="updateCurrentMainFile"
      >
        <template #default>
          <el-option
            v-for="item in currentCheckedFiles"
            :key="item.value"
            :label="item.label"
            :value="item.value"
          />
        </template>
      </el-select>
    </div>

    <div
      class="h-[400px] overflow-y-auto border border-base rounded p-2 custom-scrollbar"
      v-loading="loadingFileTree"
    >
      <el-tree
        ref="fileTreeRef"
        :data="currentTreeData"
        node-key="path"
        :props="treeProps"
        show-checkbox
        highlight-current
        check-on-click-node
        empty-text="暂无文件"
        class="bg-transparent"
        @check="emit('tree-check')"
      >
        <template #default="{ node, data }">
          <div class="flex items-center flex-1 overflow-hidden text-sm">
            <el-icon
              v-if="data.is_dir"
              class="mr-1.5 text-yellow-500 text-base"
            >
              <Folder />
            </el-icon>
            <el-icon v-else class="mr-1.5 text-blue-400 text-base">
              <Document />
            </el-icon>
            <span class="truncate flex-1 text-default" :title="node.label">
              {{ node.label }}
            </span>
            <span v-if="!data.is_dir && data.size" class="text-xs text-muted ml-2">
              {{ formatFileSize(data.size) }}
            </span>
          </div>
        </template>
      </el-tree>
    </div>
    <div v-if="isMobile" class="mt-4">
      <el-button type="primary" class="w-full" @click="emit('stop-edit')">
        确认选择
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { ArrowLeft, Document, Edit, Folder } from "@element-plus/icons-vue";
import { formatFileSize } from "../../utils/format";
import type { FileNode } from "@/types";
import type { Task } from "../../stores/tasks";
import type {
  CheckedFileOption,
  TaskShareFileSelection,
} from "./taskShareDialogTypes";

type FileTreeInstance = {
  getCheckedNodes: (leafOnly?: boolean, includeHalfChecked?: boolean) => FileNode[];
  setCheckedKeys: (keys: string[]) => void;
  setCheckedNodes: (nodes: FileNode[]) => void;
};

const props = defineProps<{
  currentCheckedFiles: CheckedFileOption[];
  currentEditingTask: Task | null;
  currentMainFile: string;
  currentTreeData: FileNode[];
  isMobile: boolean;
  loadingFileTree: boolean;
  taskFiles: Record<number, TaskShareFileSelection>;
  tasks: Task[];
}>();

const emit = defineEmits<{
  (event: "edit", task: Task): void;
  (event: "select-all"): void;
  (event: "stop-edit"): void;
  (event: "tree-check"): void;
  (event: "update:currentMainFile", value: string): void;
}>();

const fileTreeRef = ref<FileTreeInstance | null>(null);

const treeProps = {
  label: "name",
  children: "children",
  isLeaf: (data: FileNode) => !data.is_dir,
};

const getFileName = (path: string) => path.split("/").pop();

const getTaskFileCount = (taskId: number) => {
  const data = props.taskFiles[taskId];
  if (!data) return 0;
  return (data.main ? 1 : 0) + (data.deps ? data.deps.size : 0);
};

const hasFiles = (taskId: number) => getTaskFileCount(taskId) > 0;

const getTaskFilesPreviewText = (taskId: number) => {
  const data = props.taskFiles[taskId];
  if (!data || !data.main) return "未设置主文件";
  const mainName = getFileName(data.main);
  const count = data.deps ? data.deps.size : 0;
  return `主文件: ${mainName}` + (count > 0 ? ` + ${count} 个依赖` : "");
};

const updateCurrentMainFile = (value: string) => {
  emit("update:currentMainFile", value);
};

defineExpose({
  getCheckedNodes: () => fileTreeRef.value?.getCheckedNodes(false, false) || [],
  setCheckedKeys: (keys: string[]) => {
    fileTreeRef.value?.setCheckedKeys(keys);
  },
  setCheckedNodes: (nodes: FileNode[]) => {
    fileTreeRef.value?.setCheckedNodes(nodes);
  },
});
</script>
