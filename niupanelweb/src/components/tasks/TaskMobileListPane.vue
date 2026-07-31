<template>
  <div class="flex-1 flex flex-col overflow-hidden relative">
    <div class="flex flex-col shrink-0 bg-card border-b border-base z-20">
      <div class="flex h-12 items-center justify-between gap-2 px-3">
        <label
          class="flex h-9 min-w-0 flex-1 items-center rounded-md border border-light bg-base px-2.5 transition-colors focus-within:border-primary/50 focus-within:bg-card"
        >
          <span class="i-ep-search mr-2 shrink-0 text-xs text-muted"></span>
          <input
            v-model="searchValue"
            type="search"
            inputmode="search"
            aria-label="搜索任务名称或脚本"
            placeholder="搜索任务名称或脚本"
            class="min-w-0 flex-1 appearance-none border-none bg-transparent text-xs text-default outline-none placeholder:text-muted/60"
          />
          <span
            v-if="loading && totalTasks > 0"
            class="i-ep-loading ml-2 shrink-0 animate-spin text-primary"
          ></span>
        </label>

        <button
          v-if="!selectionMode"
          type="button"
          class="accent-subtle h-9 w-9 shrink-0 rounded-md flex-center transition-[filter,box-shadow] duration-200 hover:brightness-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/25"
          title="新建任务"
          aria-label="新建任务"
          @click="emit('open-create-sheet')"
        >
          <span class="i-ep-plus text-[17px]"></span>
        </button>
      </div>
      <div class="flex items-center gap-1 overflow-x-auto no-scrollbar w-full px-3 pb-2">
        <button
          v-for="item in statusPills"
          :key="item.value"
          class="h-7 shrink-0 px-2.5 rounded-md text-[10px] font-semibold transition-colors border-none outline-none"
          :class="
            statusValue === item.value
              ? 'accent-subtle'
              : 'text-muted hover:bg-soft hover:text-secondary'
          "
          @click="statusValue = item.value"
        >
          {{
            {
              all: "全部",
              Running: "活跃",
              Paused: "暂停",
              Stopped: "停止",
              Failed: "失败",
            }[item.value] || item.label
          }}
          <span class="ml-1 opacity-60">{{ statusCount(item.value) }}</span>
        </button>
      </div>
    </div>

    <div class="flex-1 relative flex flex-col min-h-0">
      <div v-if="loading && totalTasks === 0" class="h-full p-4">
        <el-skeleton animated :loading="true" :count="6" />
      </div>
      <div v-else class="h-full">
        <PullToRefresh :on-refresh="refreshTasks" :disabled="false">
          <TaskCardList
            :tasks="tasks"
            :selected-tasks="selectedTasks"
            :selection-mode="selectionMode"
            :loading="loading"
            :no-more="noMore"
            @load-more="emit('load-more')"
            @selection-change="(task, selected) => emit('selection-change', task, selected)"
            @run="emit('run', $event)"
            @stop="emit('stop', $event)"
            @pause="emit('pause', $event)"
            @resume="emit('resume', $event)"
            @enable="emit('enable', $event)"
            @disable="emit('disable', $event)"
            @delete="emit('delete', $event)"
            @pin="emit('pin', $event)"
            @unpin="emit('unpin', $event)"
            @edit="emit('edit', $event)"
            @share="emit('share', $event)"
            @logs="emit('logs', $event)"
            @toggle-enable="(task, enabled) => emit('toggle-enable', task, enabled)"
            @edit-variables="emit('edit-variables', $event)"
            @more-actions="emit('more-actions', $event)"
            @enter-selection="emit('enter-selection', $event)"
            @edit-cron="emit('edit-cron', $event)"
          />
        </PullToRefresh>
      </div>
    </div>

    <BulkActionBar
      :count="selectedIds.length"
      :show-select-all="true"
      :is-all-selected="isAllSelected"
      @cancel="emit('cancel-selection')"
      @delete="emit('bulk-delete')"
      @select-all="emit('select-all')"
      @command="handleBulkCommand"
    >
      <template #actions>
        <el-button
          link
          type="primary"
          class="!px-2 !py-1"
          @click="emit('bulk-run')"
        >
          <div class="flex flex-col items-center gap-0.5">
            <div class="i-ep-video-play text-[22px]"></div>
            <span class="text-[10px] opacity-80 mt-0.5">运行</span>
          </div>
        </el-button>
        <el-button
          link
          type="warning"
          class="!px-2 !py-1"
          @click="emit('bulk-pause')"
        >
          <div class="flex flex-col items-center gap-0.5">
            <div class="i-ep-video-pause text-[22px]"></div>
            <span class="text-[10px] opacity-80 mt-0.5">暂停</span>
          </div>
        </el-button>
        <el-button
          link
          type="danger"
          class="!px-2 !py-1"
          @click="emit('bulk-stop')"
        >
          <div class="flex flex-col items-center gap-0.5">
            <div class="i-ep-switch-button text-[20px]"></div>
            <span class="text-[10px] opacity-80 mt-0.5">停止</span>
          </div>
        </el-button>
      </template>
      <template #more>
        <el-dropdown-item command="resume">
          <div class="flex items-center gap-2">
            <div class="i-ep-refresh text-blue-500"></div>
            恢复运行
          </div>
        </el-dropdown-item>
        <el-dropdown-item command="share">
          <div class="flex items-center gap-2 text-purple-500">
            <div class="i-ep-share"></div>
            分享选中
          </div>
        </el-dropdown-item>
        <el-dropdown-item command="pin">
          <div class="flex items-center gap-2 text-yellow-500">
            <div class="i-ep-top"></div>
            批量置顶
          </div>
        </el-dropdown-item>
        <el-dropdown-item command="unpin">
          <div class="flex items-center gap-2 text-orange-500">
            <div class="i-ep-bottom"></div>
            取消置顶
          </div>
        </el-dropdown-item>
        <el-dropdown-item command="enable">
          <div class="flex items-center gap-2 text-emerald-500">
            <div class="i-ep-check"></div>
            批量启用
          </div>
        </el-dropdown-item>
        <el-dropdown-item command="disable">
          <div class="flex items-center gap-2 text-rose-500">
            <div class="i-ep-close"></div>
            批量禁用
          </div>
        </el-dropdown-item>
      </template>
    </BulkActionBar>

  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Task } from "@/types";
import {
  isTaskBulkCommand,
  type TaskBulkCommand,
} from "../../composables/taskPageTypes";
import { statusPills } from "../../composables/useTaskPresentation";
import BulkActionBar from "../common/BulkActionBar.vue";
import PullToRefresh from "../common/PullToRefresh.vue";
import TaskCardList from "./TaskCardList.vue";

const props = defineProps<{
  allTasks: Task[];
  isAllSelected: boolean;
  loading: boolean;
  noMore: boolean;
  refreshTasks: () => unknown;
  searchQuery: string;
  selectedIds: number[];
  selectedTasks: Task[];
  selectionMode: boolean;
  statusFilter: string;
  tasks: Task[];
  totalTasks: number;
}>();

const emit = defineEmits<{
  (event: "bulk-command", command: TaskBulkCommand): void;
  (event: "bulk-delete"): void;
  (event: "bulk-pause"): void;
  (event: "bulk-run"): void;
  (event: "bulk-stop"): void;
  (event: "cancel-selection"): void;
  (event: "create"): void;
  (event: "delete", id: number): void;
  (event: "disable", id: number): void;
  (event: "edit", task: Task): void;
  (event: "edit-cron", task: Task): void;
  (event: "edit-variables", taskId: number): void;
  (event: "enable", id: number): void;
  (event: "enter-selection", task: Task): void;
  (event: "load-more"): void;
  (event: "logs", task: Task): void;
  (event: "more-actions", task: Task): void;
  (event: "open-create-sheet"): void;
  (event: "pause", id: number): void;
  (event: "pin", id: number): void;
  (event: "resume", id: number): void;
  (event: "run", id: number): void;
  (event: "select-all"): void;
  (event: "selection-change", task: Task, selected: boolean): void;
  (event: "share", task: Task): void;
  (event: "stop", id: number): void;
  (event: "toggle-enable", task: Task, enabled: boolean): void;
  (event: "unpin", id: number): void;
  (event: "update:searchQuery", value: string): void;
  (event: "update:statusFilter", value: string): void;
}>();

const handleBulkCommand = (command: unknown) => {
  if (isTaskBulkCommand(command)) {
    emit("bulk-command", command);
  }
};

const searchValue = computed({
  get: () => props.searchQuery,
  set: (value: string) => emit("update:searchQuery", value),
});

const statusValue = computed({
  get: () => props.statusFilter,
  set: (value: string) => emit("update:statusFilter", value),
});

const statusCount = (status: string) =>
  status === "all" ? props.allTasks.length : props.allTasks.filter((task) => task.status === status).length;
</script>
