<template>
  <div
    class="px-2 py-2 flex items-center gap-1 overflow-x-auto no-scrollbar border-b border-light/50 bg-base/5"
  >
    <button
      class="shrink-0 px-2 py-1.5 rounded-lg label-micro transition-all text-primary hover:bg-primary/10 flex items-center gap-1"
      @click="emit('bulk-run')"
    >
      <div class="i-ep-video-play"></div>
      运行
    </button>
    <button
      class="shrink-0 px-2 py-1.5 rounded-lg label-micro transition-all text-rose-500 hover:bg-rose-50 flex items-center gap-1"
      @click="emit('bulk-stop')"
    >
      <div class="i-ep-video-pause"></div>
      停止
    </button>
    <button
      class="shrink-0 px-2 py-1.5 rounded-lg label-micro transition-all text-orange-400 hover:bg-orange-50 flex items-center gap-1"
      @click="emit('bulk-pause')"
    >
      <div class="i-ep-warning"></div>
      暂停
    </button>
    <button
      class="shrink-0 px-2 py-1.5 rounded-lg label-micro transition-all text-emerald-500 hover:bg-emerald-50 flex items-center gap-1"
      @click="emit('bulk-enable')"
    >
      <div class="i-ep-check"></div>
      启用
    </button>
    <button
      class="shrink-0 px-2 py-1.5 rounded-lg label-micro transition-all text-gray-400 hover:bg-gray-50 flex items-center gap-1"
      @click="emit('bulk-disable')"
    >
      <div class="i-ep-minus"></div>
      禁用
    </button>
    <button
      class="shrink-0 px-2 py-1.5 rounded-lg label-micro transition-all text-blue-500 hover:bg-blue-50 flex items-center gap-1"
      @click="emit('bulk-pin')"
    >
      <div class="i-ep-top"></div>
      置顶
    </button>
    <button
      class="shrink-0 px-2 py-1.5 rounded-lg label-micro transition-all text-purple-500 hover:bg-purple-50 flex items-center gap-1"
      @click="emit('bulk-share')"
    >
      <div class="i-ep-share"></div>
      分享
    </button>
    <el-dropdown trigger="click" @command="handleBulkMoreCommand">
      <button
        class="shrink-0 px-2 py-1.5 rounded-lg label-micro transition-all text-muted hover:bg-base flex items-center gap-1"
      >
        <div class="i-ep-more-filled"></div>
        更多
      </button>
      <template #dropdown>
        <el-dropdown-menu class="modern-dropdown">
          <el-dropdown-item command="resume">
            <div class="flex items-center gap-2">
              <div class="i-ep-refresh"></div>
              恢复运行
            </div>
          </el-dropdown-item>
          <el-dropdown-item command="unpin">
            <div class="flex items-center gap-2">
              <div class="i-ep-bottom"></div>
              取消置顶
            </div>
          </el-dropdown-item>
          <el-dropdown-item command="delete" class="!text-rose-500">
            <div class="flex items-center gap-2">
              <div class="i-ep-delete"></div>
              删除选中
            </div>
          </el-dropdown-item>
        </el-dropdown-menu>
      </template>
    </el-dropdown>
  </div>
</template>

<script setup lang="ts">
import {
  isTaskBulkMoreCommand,
  type TaskBulkMoreCommand,
} from "../../composables/taskPageTypes";

const emit = defineEmits<{
  (event: "bulk-disable"): void;
  (event: "bulk-enable"): void;
  (event: "bulk-more-command", command: TaskBulkMoreCommand): void;
  (event: "bulk-pause"): void;
  (event: "bulk-pin"): void;
  (event: "bulk-run"): void;
  (event: "bulk-share"): void;
  (event: "bulk-stop"): void;
}>();

const handleBulkMoreCommand = (command: unknown) => {
  if (isTaskBulkMoreCommand(command)) {
    emit("bulk-more-command", command);
  }
};
</script>
