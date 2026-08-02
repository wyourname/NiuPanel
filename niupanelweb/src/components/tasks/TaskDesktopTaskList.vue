<template>
  <div
    class="flex-1 overflow-y-auto custom-scrollbar"
    @scroll.passive="handleScroll"
  >
    <div v-if="loading && totalTasks === 0" class="p-3 space-y-2">
      <el-skeleton animated :loading="true" :count="6">
        <template #template>
          <div class="flex items-center gap-3 p-2.5 bg-white/5 rounded-md">
            <el-skeleton-item
              variant="circle"
              style="width: 40px; height: 40px"
            />
            <div class="flex-1 space-y-2">
              <el-skeleton-item variant="text" style="width: 40%" />
              <el-skeleton-item variant="text" style="width: 70%" />
            </div>
          </div>
        </template>
      </el-skeleton>
    </div>

    <div v-else>
      <div
        v-if="tasks.length === 0"
        class="flex-center flex-col px-6 py-20 text-center text-muted"
      >
        <div class="h-11 w-11 rounded-md bg-soft flex-center mb-3">
          <div class="i-ep-search text-[20px]"></div>
        </div>
        <span class="text-[13px] font-bold text-default">没有符合条件的任务</span>
        <span class="mt-1 text-[11px]">可以调整搜索内容或状态筛选</span>
      </div>

      <el-dropdown
        v-for="task in tasks"
        :key="task.id"
        trigger="contextmenu"
        placement="bottom-end"
        @command="(command: unknown) => handleContextCommand(command, task)"
        class="block w-full"
      >
        <TaskCardItem
          :task="task"
          :is-selected="isTaskSelected(task)"
          :selection-mode="selectionMode"
          @selection-change="(selectedTask, selected) => emit('selection-change', selectedTask, selected)"
          @logs="emit('select-task', task)"
          @more-actions="emit('more-actions', $event)"
          @delete="emit('delete', $event)"
          @run="emit('run', $event)"
          @stop="emit('stop', $event)"
          @enable="emit('toggle-enable', task, true)"
          @disable="emit('toggle-enable', task, false)"
        />
        <template #dropdown>
          <el-dropdown-menu class="modern-dropdown w-48">
            <el-dropdown-item command="run" v-if="task.status !== 'Running'">
              <div class="flex items-center gap-3">
                <div class="i-ep-video-play text-lg text-primary"></div>
                立即运行
              </div>
            </el-dropdown-item>
            <el-dropdown-item command="stop" v-else>
              <div class="flex items-center gap-3">
                <div class="i-ep-video-pause text-lg text-rose-500"></div>
                停止执行
              </div>
            </el-dropdown-item>
            <el-dropdown-item command="edit">
              <div class="flex items-center gap-3">
                <div class="i-ep-edit text-lg"></div>
                编辑任务
              </div>
            </el-dropdown-item>
            <el-dropdown-item command="script">
              <div class="flex items-center gap-3">
                <div class="i-ep-document text-lg text-purple-500"></div>
                编辑代码
              </div>
            </el-dropdown-item>
            <el-dropdown-item command="vars">
              <div class="flex items-center gap-3">
                <div class="i-ep-key text-lg text-emerald-500"></div>
                环境变量
              </div>
            </el-dropdown-item>
            <el-dropdown-item command="cron">
              <div class="flex items-center gap-3">
                <div class="i-ep-clock text-lg text-orange-400"></div>
                定时规则
              </div>
            </el-dropdown-item>
            <el-dropdown-item command="share">
              <div class="flex items-center gap-3">
                <div class="i-ep-share text-lg text-purple-500"></div>
                分享任务
              </div>
            </el-dropdown-item>
            <div class="h-px bg-light/50 my-1 mx-2"></div>
            <el-dropdown-item command="pin">
              <div class="flex items-center gap-3">
                <div class="i-ep-top text-lg text-blue-500"></div>
                {{ task.is_pinned ? "取消置顶" : "置顶任务" }}
              </div>
            </el-dropdown-item>
            <el-dropdown-item command="delete">
              <div class="flex items-center gap-3">
                <div class="i-ep-delete text-lg text-rose-500"></div>
                删除任务
              </div>
            </el-dropdown-item>
            <div class="h-px bg-light/50 my-1 mx-2"></div>
            <el-dropdown-item command="select">
              <div class="flex items-center gap-3">
                <div class="i-ep-finished text-lg text-primary"></div>
                多选该任务
              </div>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>

      <div v-if="loading && totalTasks > 0" class="py-3 text-center">
        <div class="i-ep-loading animate-spin text-primary inline-block"></div>
      </div>
      <div
        v-if="noMore && totalTasks > 0"
        class="py-4 text-center label-micro text-muted"
      >
        共 {{ totalTasks }} 个任务
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Task } from "@/types";
import {
  isTaskContextCommand,
  type TaskContextCommand,
} from "../../composables/taskPageTypes";
import TaskCardItem from "./TaskCardItem.vue";

const props = defineProps<{
  loading: boolean;
  noMore: boolean;
  selectedIds: number[];
  selectedTaskId: number | null;
  selectionMode: boolean;
  tasks: Task[];
  totalTasks: number;
}>();

const emit = defineEmits<{
  (event: "context-command", command: TaskContextCommand, task: Task): void;
  (event: "delete", id: number): void;
  (event: "load-more"): void;
  (event: "more-actions", task: Task): void;
  (event: "run", id: number): void;
  (event: "select-task", task: Task): void;
  (event: "selection-change", task: Task, value: boolean): void;
  (event: "stop", id: number): void;
  (event: "toggle-enable", task: Task, enabled: boolean): void;
}>();

const handleContextCommand = (command: unknown, task: Task) => {
  if (isTaskContextCommand(command)) {
    emit("context-command", command, task);
  }
};

const handleScroll = (event: Event) => {
  if (props.loading || props.noMore) return;

  const target = event.currentTarget as HTMLElement;
  const remaining = target.scrollHeight - target.scrollTop - target.clientHeight;
  if (remaining <= 20) emit("load-more");
};

const isTaskSelected = (task: Task) => {
  return props.selectionMode
    ? props.selectedIds.includes(task.id)
    : props.selectedTaskId === task.id;
};
</script>
