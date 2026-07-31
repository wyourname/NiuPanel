<template>
  <div class="relative min-h-0 flex-1 overflow-hidden">
    <div class="h-full min-h-0 overflow-y-auto pb-24 custom-scrollbar">
      <div
        v-if="items.length === 0 && !loading"
        class="h-[50vh] flex flex-col items-center justify-center select-none"
      >
        <div class="mb-4 h-14 w-14 rounded-md border border-light/40 bg-soft/50 flex-center">
          <div :class="emptyStateIcon" class="text-3xl text-muted/20"></div>
        </div>
        <p class="text-sm font-medium text-muted/50">
          {{ searchQuery ? "没有结果" : "空文件夹" }}
        </p>
        <p class="text-xs text-muted/30 mt-1">
          {{ searchQuery ? "换关键词试试" : "点击 + 创建" }}
        </p>
      </div>

      <div v-else class="divide-y divide-light/70 border-b border-light/70 bg-card">
        <article
          v-for="row in items"
          :key="row.path"
          class="group relative transition-colors"
          :class="isSelected(row) ? 'bg-soft/60' : 'hover:bg-soft/30'"
          @click="emit('item-click', row)"
          @touchstart="emit('touch-start', row)"
          @touchend="emit('touch-end')"
          @touchmove="emit('touch-move')"
        >
          <div
            v-if="isSelected(row)"
            class="absolute bottom-0 left-0 top-0 w-[2px] bg-primary"
          ></div>

          <div class="flex items-center gap-3 px-4 py-2.5">
            <div
              class="h-10 w-10 shrink-0 rounded-md flex-center"
              :class="getFileIconBgClass(row)"
            >
              <div :class="getFileIconClass(row)" class="text-[20px]"></div>
            </div>

            <div class="flex-1 min-w-0 pointer-events-none">
              <span class="block font-medium text-[14px] text-default truncate leading-tight">
                {{ row.name }}
              </span>
              <div class="flex items-center gap-1.5 mt-0.5">
                <span v-if="!row.is_dir" class="text-[11px] font-mono text-muted/45 tabular-nums">
                  {{ formatFileSize(row.size) }}
                </span>
                <span
                  v-if="row.is_dir"
                  class="text-[10px] font-medium text-amber-600/70 dark:text-amber-300/70"
                >
                  文件夹
                </span>
                <span v-if="row.mtime" class="text-[10.5px] text-muted/35">
                  {{ formatRelativeFileDate(row.mtime) }}
                </span>
              </div>
            </div>

            <el-dropdown trigger="click" @command="handleCommand($event, row)">
              <button
                type="button"
                class="h-8 w-8 shrink-0 rounded-md text-muted flex-center transition-colors hover:bg-soft hover:text-default"
                title="文件操作"
                aria-label="文件操作"
                @click.stop
              >
                <div class="i-ep-more-filled text-sm"></div>
              </button>
              <template #dropdown>
                <el-dropdown-menu class="modern-dropdown">
                  <el-dropdown-item v-if="!row.is_dir && isEditableFile(row.name)" command="edit">
                    <div class="i-ep-edit mr-2"></div>
                    编辑
                  </el-dropdown-item>
                  <el-dropdown-item v-if="!row.is_dir" command="download">
                    <div class="i-ep-download mr-2"></div>
                    下载
                  </el-dropdown-item>
                  <el-dropdown-item v-if="!row.is_dir && isArchiveFile(row.name)" command="extract">
                    <div class="i-ep-box mr-2"></div>
                    解压
                  </el-dropdown-item>
                  <el-dropdown-item command="copy">
                    <div class="i-ep-copy-document mr-2"></div>
                    复制
                  </el-dropdown-item>
                  <el-dropdown-item command="cut">
                    <div class="i-ep-scissor mr-2"></div>
                    剪切
                  </el-dropdown-item>
                  <el-dropdown-item divided command="rename">
                    <div class="i-ep-edit-pen mr-2"></div>
                    重命名
                  </el-dropdown-item>
                  <el-dropdown-item command="move">
                    <div class="i-ep-position mr-2"></div>
                    移动
                  </el-dropdown-item>
                  <el-dropdown-item command="delete" class="!text-rose-500">
                    <div class="i-ep-delete mr-2"></div>
                    删除
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
        </article>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { formatFileSize } from "../../../../utils/format";
import type { FileItem } from "../../../../composables/useFileOperations";
import {
  formatRelativeFileDate,
  getFileIconBgClass,
  getFileIconClass,
  isArchiveFile,
  isEditableFile,
} from "../utils/fileDisplay";
import { isFileCommand, type FileCommand } from "../utils/fileActions";

const props = defineProps<{
  items: FileItem[];
  loading: boolean;
  searchQuery: string;
  selectedPaths: string[];
}>();

const emit = defineEmits<{
  (event: "command", command: FileCommand, row: FileItem): void;
  (event: "item-click", row: FileItem): void;
  (event: "touch-end"): void;
  (event: "touch-move"): void;
  (event: "touch-start", row: FileItem): void;
}>();

const emptyStateIcon = computed(() =>
  props.searchQuery ? "i-ep-search" : "i-ep-folder-opened",
);

const isSelected = (row: FileItem) => props.selectedPaths.includes(row.path);

const handleCommand = (command: unknown, row: FileItem) => {
  if (isFileCommand(command)) emit("command", command, row);
};
</script>
