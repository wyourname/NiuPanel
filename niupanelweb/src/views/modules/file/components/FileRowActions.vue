<template>
  <div class="flex justify-end items-center gap-1.5 pr-2" @click.stop>
    <el-tooltip
      :content="row.is_dir ? '打开' : isEditable(row.name) ? '编辑' : '查看'"
      placement="top"
      :show-after="600"
    >
      <button
        type="button"
        class="h-8 w-8 rounded-md text-muted flex-center transition-colors hover:bg-hover hover:text-primary"
        @click.stop="emit('open', row)"
      >
        <div :class="getRowActionIcon(row)" class="text-base"></div>
      </button>
    </el-tooltip>

    <el-dropdown trigger="click" @command="handleCommand($event, row)">
      <button
        type="button"
        class="h-8 w-8 rounded-md text-muted flex-center transition-colors hover:bg-hover hover:text-primary"
        title="更多操作"
        aria-label="更多文件操作"
      >
        <div class="i-ep-more-filled text-base"></div>
      </button>
      <template #dropdown>
        <el-dropdown-menu class="modern-dropdown">
          <el-dropdown-item v-if="!row.is_dir" command="download">
            <div class="flex items-center gap-2">
              <div class="i-ep-download"></div>
              下载
            </div>
          </el-dropdown-item>
          <el-dropdown-item v-if="!row.is_dir && isArchiveFile(row.name)" command="extract">
            <div class="flex items-center gap-2">
              <div class="i-ep-box"></div>
              解压
            </div>
          </el-dropdown-item>
          <el-dropdown-item command="rename">
            <div class="flex items-center gap-2">
              <div class="i-ep-edit-pen"></div>
              重命名
            </div>
          </el-dropdown-item>
          <el-dropdown-item command="move">
            <div class="flex items-center gap-2">
              <div class="i-ep-position"></div>
              移动到...
            </div>
          </el-dropdown-item>
          <div class="h-px bg-light/50 my-1 mx-2"></div>
          <el-dropdown-item command="delete" class="!text-rose-500">
            <div class="flex items-center gap-2">
              <div class="i-ep-delete"></div>
              彻底删除
            </div>
          </el-dropdown-item>
        </el-dropdown-menu>
      </template>
    </el-dropdown>
  </div>
</template>

<script setup lang="ts">
import type { FileItem } from "../../../../composables/useFileOperations";
import { isFileCommand, type FileCommand } from "../utils/fileActions";
import { isArchiveFile } from "../utils/fileDisplay";

type FileTableCommand = Extract<
  FileCommand,
  "delete" | "download" | "extract" | "move" | "rename"
>;

const props = defineProps<{
  isEditable: (name: string) => boolean;
  row: FileItem;
}>();

const emit = defineEmits<{
  (event: "delete", row: FileItem): void;
  (event: "download", row: FileItem): void;
  (event: "extract", row: FileItem): void;
  (event: "move", row: FileItem): void;
  (event: "open", row: FileItem): void;
  (event: "rename", row: FileItem): void;
}>();

const handleCommand = (cmd: unknown, row: FileItem) => {
  if (!isFileCommand(cmd) || !isFileTableCommand(cmd)) return;
  if (cmd === "download") emit("download", row);
  if (cmd === "extract") emit("extract", row);
  if (cmd === "rename") emit("rename", row);
  if (cmd === "move") emit("move", row);
  if (cmd === "delete") emit("delete", row);
};

const isFileTableCommand = (cmd: FileCommand): cmd is FileTableCommand =>
  cmd === "download" ||
  cmd === "extract" ||
  cmd === "rename" ||
  cmd === "move" ||
  cmd === "delete";

const getRowActionIcon = (row: FileItem) => {
  if (row.is_dir) return "i-ep-folder-opened";
  return props.isEditable(row.name) ? "i-ep-edit" : "i-ep-view";
};
</script>
