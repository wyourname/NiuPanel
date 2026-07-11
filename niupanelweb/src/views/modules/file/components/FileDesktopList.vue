<template>
  <div
    v-loading="loading"
    class="h-full min-h-0 flex-1 overflow-hidden"
  >
    <div class="h-full min-h-0 overflow-auto p-2 custom-scrollbar">
      <div
        v-if="items.length === 0 && !loading"
        class="h-full min-h-[280px] flex-col text-muted flex-center"
      >
        <div class="mb-2 h-14 w-14 rounded-md border border-light bg-soft/50 text-muted/40 flex-center">
          <div :class="emptyStateIcon" class="text-[26px]"></div>
        </div>
        <div class="text-[13px] font-black">
          {{ searchQuery ? "没有匹配结果" : "当前目录为空" }}
        </div>
      </div>

      <div v-else-if="viewMode === 'detail'" class="min-w-[720px]">
        <div
          class="sticky top-0 z-10 grid h-8 grid-cols-[minmax(260px,1fr)_104px_104px_144px_36px] items-center border-b border-light bg-card px-2.5 text-[10px] font-bold text-muted"
        >
          <div>名称</div>
          <div>类型</div>
          <div class="text-right">大小</div>
          <div class="text-right">修改时间</div>
          <div></div>
        </div>

        <div>
          <div
            v-for="row in items"
            :key="row.path"
            class="group relative grid min-h-10 cursor-pointer grid-cols-[minmax(260px,1fr)_104px_104px_144px_36px] items-center border-b border-light/70 px-2.5 transition-colors"
            :class="rowClass(row)"
            draggable="true"
            @click="handleRowClick(row, $event)"
            @contextmenu.prevent="emit('context-menu', row, $event)"
            @dblclick="handleRowDoubleClick(row)"
            @dragend="emit('item-drag-end')"
            @dragstart="emit('item-drag-start', row, $event)"
          >
            <div
              v-if="isSelected(row)"
              class="absolute bottom-2 left-0 top-2 w-[3px] rounded-r-full bg-primary"
            ></div>

            <div class="min-w-0 pr-4">
              <div class="flex min-w-0 items-center gap-2.5">
                <button
                  type="button"
                  class="h-[18px] w-[18px] shrink-0 cursor-pointer rounded border flex-center transition-colors"
                  :class="isSelected(row) ? 'border-primary bg-primary text-white' : 'border-slate-900/12 bg-white/50 text-transparent group-hover:text-muted dark:border-white/12 dark:bg-white/6'"
                  title="选择"
                  @click.stop="emit('toggle-selection', row)"
                >
                  <div class="i-ep-check text-[12px]"></div>
                </button>
                <div
                  class="h-8 w-8 shrink-0 rounded-md flex-center"
                  :class="getFileIconBgClass(row)"
                >
                  <div :class="getFileIconClass(row)" class="text-[16px]"></div>
                </div>
                <div class="min-w-0 truncate text-[13px] font-black leading-5 text-default">
                  {{ row.name }}
                </div>
              </div>
            </div>

            <div class="text-[11px] font-black text-secondary">
              {{ getFileTypeLabel(row) }}
            </div>

            <div class="text-right text-[11px] font-bold tabular-nums text-muted">
              {{ row.is_dir ? "-" : formatFileSize(row.size) }}
            </div>

            <div class="text-right text-[11px] font-bold tabular-nums text-muted">
              {{ row.mtime ? formatFullFileDate(row.mtime) : "-" }}
            </div>

            <div class="flex justify-end" @click.stop>
              <el-dropdown trigger="click" @command="handleCommand($event, row)">
                <button
                  type="button"
                  class="h-7 w-7 cursor-pointer rounded-md text-muted/55 flex-center opacity-0 transition-colors hover:bg-soft hover:text-default group-hover:opacity-100"
                  title="更多"
                >
                  <div class="i-ep-more-filled text-[13px]"></div>
                </button>
                <template #dropdown>
                  <el-dropdown-menu class="modern-dropdown">
                    <el-dropdown-item v-if="!row.is_dir && isEditableFile(row.name)" command="edit">
                      <div class="i-ep-edit mr-2"></div>
                      编辑
                    </el-dropdown-item>
                    <el-dropdown-item v-if="!row.is_dir && isImageFile(row.name)" command="preview">
                      <div class="i-ep-picture mr-2"></div>
                      预览
                    </el-dropdown-item>
                    <el-dropdown-item v-if="!row.is_dir" command="download">
                      <div class="i-ep-download mr-2"></div>
                      下载
                    </el-dropdown-item>
                    <el-dropdown-item v-if="!row.is_dir && isArchiveFile(row.name)" command="extract">
                      <div class="i-ep-box mr-2"></div>
                      解压
                    </el-dropdown-item>
                    <el-dropdown-item divided command="rename">
                      <div class="i-ep-edit-pen mr-2"></div>
                      重命名
                    </el-dropdown-item>
                    <el-dropdown-item command="move">
                      <div class="i-ep-position mr-2"></div>
                      移动
                    </el-dropdown-item>
                    <el-dropdown-item command="copy">
                      <div class="i-ep-copy-document mr-2"></div>
                      复制
                    </el-dropdown-item>
                    <el-dropdown-item command="cut">
                      <div class="i-ep-scissor mr-2"></div>
                      剪切
                    </el-dropdown-item>
                    <el-dropdown-item divided command="delete" class="!text-rose-500">
                      <div class="i-ep-delete mr-2"></div>
                      删除
                    </el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </div>
          </div>
        </div>
      </div>

      <div
        v-else
        class="grid grid-cols-[repeat(auto-fill,minmax(146px,1fr))] gap-2"
      >
        <div
          v-for="row in items"
          :key="row.path"
          class="group relative min-h-[108px] cursor-pointer rounded-md border p-2.5 transition-colors"
          :class="rowClass(row)"
          draggable="true"
          @click="handleRowClick(row, $event)"
          @contextmenu.prevent="emit('context-menu', row, $event)"
          @dblclick="handleRowDoubleClick(row)"
          @dragend="emit('item-drag-end')"
          @dragstart="emit('item-drag-start', row, $event)"
        >
          <div class="relative flex h-10 items-center justify-center">
            <div
              class="h-10 w-10 rounded-md flex-center"
              :class="getFileIconBgClass(row)"
            >
              <div :class="getFileIconClass(row)" class="text-[20px]"></div>
            </div>
          </div>

          <div class="mt-2 min-w-0 text-center">
            <div class="line-clamp-2 min-h-9 break-all text-center text-[12px] font-black leading-[18px] text-default">
              {{ row.name }}
            </div>
            <div class="mt-1 flex min-w-0 items-center justify-center gap-1.5 text-center text-[10px] font-bold text-muted">
              <span>{{ getFileTypeLabel(row) }}</span>
              <span class="text-muted/45">·</span>
              <span>{{ row.is_dir ? "-" : formatFileSize(row.size) }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onUnmounted } from "vue";
import { formatFileSize } from "../../../../utils/format";
import type { FileItem } from "../../../../composables/useFileOperations";
import {
  formatFullFileDate,
  getFileIconBgClass,
  getFileIconClass,
  getFileTypeLabel,
  isArchiveFile,
  isEditableFile,
  isImageFile,
} from "../utils/fileDisplay";
import { isFileCommand, type FileCommand } from "../utils/fileActions";

type FileViewMode = "detail" | "grid";

const props = defineProps<{
  items: FileItem[];
  loading: boolean;
  searchQuery: string;
  selectedPaths: string[];
  viewMode: FileViewMode;
}>();

const emit = defineEmits<{
  (event: "command", command: FileCommand, row: FileItem): void;
  (event: "context-menu", row: FileItem, mouseEvent: MouseEvent): void;
  (event: "item-drag-end"): void;
  (event: "item-drag-start", row: FileItem, dragEvent: DragEvent): void;
  (event: "item-click", row: FileItem): void;
  (event: "toggle-selection", row: FileItem): void;
}>();

let clickTimer: ReturnType<typeof setTimeout> | null = null;

const emptyStateIcon = computed(() =>
  props.searchQuery ? "i-ep-search" : "i-ep-folder-opened",
);

const isSelected = (row: FileItem) => props.selectedPaths.includes(row.path);

const rowClass = (row: FileItem) =>
  isSelected(row)
    ? "border-primary/20 bg-soft/70"
    : "border-light/70 bg-card hover:bg-soft/35";

const handleRowClick = (row: FileItem, event: MouseEvent) => {
  if (clickTimer) clearTimeout(clickTimer);
  const additive = event.ctrlKey || event.metaKey;

  if (additive) {
    emit("toggle-selection", row);
    return;
  }

  clickTimer = setTimeout(() => {
    emit("item-click", row);
  }, 160);
};

const handleRowDoubleClick = (row: FileItem) => {
  if (clickTimer) clearTimeout(clickTimer);
  clickTimer = null;
  emit("item-click", row);
};

const handleCommand = (command: unknown, row: FileItem) => {
  if (isFileCommand(command)) emit("command", command, row);
};

onUnmounted(() => {
  if (clickTimer) clearTimeout(clickTimer);
});
</script>
