<template>
  <div class="relative min-h-0 flex-1 overflow-hidden">
    <div v-if="loading" class="absolute inset-0 z-10 bg-card/70 flex-center" role="status" aria-label="正在加载文件"><span class="i-ep-loading animate-spin text-xl text-primary" /></div>
    <div class="h-full min-h-0 overflow-y-auto custom-scrollbar">
      <div
        v-if="items.length === 0 && !loading"
        class="h-[50vh] flex flex-col items-center justify-center select-none"
      >
        <div class="mb-4 h-14 w-14 rounded-md border border-light/40 bg-soft/50 flex-center">
          <div :class="emptyStateIcon" class="text-3xl text-muted"></div>
        </div>
        <p class="text-sm font-medium text-muted">
          {{ searchQuery ? "没有结果" : "空文件夹" }}
        </p>
        <p class="mt-1 text-xs text-muted">
          {{ searchQuery ? "换关键词试试" : "点击 + 创建" }}
        </p>
      </div>

      <div
        v-else
        class="divide-y divide-light/70 border-b border-light/70 bg-card"
        :inert="loading"
        :class="selectedPaths.length > 0
          ? 'pb-[calc(var(--mobile-dock-clearance)+128px)]'
          : 'pb-[var(--mobile-dock-clearance)]'"
      >
        <article
          v-for="row in items"
          :key="row.path"
          class="group relative transition-colors"
          :class="isSelected(row) ? 'bg-soft/60' : 'hover:bg-soft/30'"
          @contextmenu.prevent
          @touchstart="emit('touch-start', row)"
          @touchend="emit('touch-end')"
          @touchmove="emit('touch-move')"
          @touchcancel="emit('touch-move'); emit('touch-end')"
        >
          <div
            v-if="isSelected(row)"
            class="absolute bottom-0 left-0 top-0 w-[2px] bg-primary"
          ></div>

          <div class="flex min-h-[64px] items-center gap-2 px-2 py-1.5">
            <button
              type="button"
              class="h-11 w-11 shrink-0 cursor-pointer rounded-md flex-center transition-colors"
              :class="isSelected(row)
                ? 'bg-primary text-white'
                : getFileIconBgClass(row)"
              :aria-label="isSelected(row) ? `取消选择 ${row.name}` : `选择 ${row.name}`"
              :aria-pressed="isSelected(row)"
              @click.stop="emit('toggle-selection', row)"
              @touchstart.stop
              @touchend.stop
              @touchmove.stop
            >
              <div :class="isSelected(row) ? 'i-ep-check' : getFileIconClass(row)" class="text-[20px]" aria-hidden="true"></div>
            </button>

            <button type="button" class="min-h-11 min-w-0 flex-1 cursor-pointer text-left" :aria-label="`${row.is_dir ? '打开目录' : '打开文件'} ${row.name}`" @click="emit('item-click', row)">
              <span class="line-clamp-2 break-all text-[13px] font-medium leading-5 text-default">
                {{ row.name }}
              </span>
              <span v-if="searchQuery" class="block truncate text-[10px] text-muted">{{ row.path }}</span>
              <span class="mt-0.5 flex min-w-0 items-center gap-2 truncate">
                <span v-if="!row.is_dir" class="font-mono text-[11px] text-muted tabular-nums">
                  {{ formatFileSize(row.size) }}
                </span>
                <span
                  v-if="row.is_dir"
                  class="text-[11px] font-medium text-amber-700 dark:text-amber-300"
                >
                  文件夹
                </span>
                <span v-if="row.mtime" class="text-[11px] text-muted">
                  {{ formatRelativeFileDate(row.mtime) }}
                </span>
              </span>
            </button>

            <el-dropdown trigger="click" @command="handleCommand($event, row)">
              <button
                type="button"
                class="h-11 w-11 shrink-0 rounded-md text-muted flex-center transition-colors hover:bg-soft hover:text-default"
                title="文件操作"
                aria-label="文件操作"
                @click.stop
                @touchstart.stop
                @touchend.stop
                @touchmove.stop
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
  (event: "toggle-selection", row: FileItem): void;
}>();

const emptyStateIcon = computed(() =>
  props.searchQuery ? "i-ep-search" : "i-ep-folder-opened",
);

const isSelected = (row: FileItem) => props.selectedPaths.includes(row.path);

const handleCommand = (command: unknown, row: FileItem) => {
  if (isFileCommand(command)) emit("command", command, row);
};
</script>
