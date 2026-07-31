<template>
  <header
    class="h-14 px-4 flex items-center justify-between bg-card border-b border-base z-30 shrink-0 sticky top-0"
  >
    <div class="flex items-center gap-3 min-w-0">
      <button
        v-if="isMobile"
        class="w-8 h-8 flex-center rounded-md bg-base text-muted hover:text-primary transition-colors cursor-pointer"
        @click="emit('back')"
      >
        <div class="i-ep-arrow-left"></div>
      </button>

      <div class="flex flex-col min-w-0">
        <div class="flex items-center gap-2">
          <h2
            class="text-[14px] font-bold text-default truncate max-w-[45vw] md:max-w-[320px]"
          >
            {{ task.name }}
          </h2>

          <div class="flex items-center gap-1.5 shrink-0">
            <span
              class="flex h-1.5 w-1.5 rounded-full"
              :class="{
                'bg-emerald-500': task.status === 'Running',
                'bg-rose-500': task.status === 'Failed',
                'bg-muted': task.status === 'Stopped' || !task.status,
                'bg-amber-500': task.status === 'Paused'
              }"
            ></span>
            <span class="text-[9px] font-mono text-muted">#{{ task.id }}</span>
            <span
              v-if="task.cron_schedule || task.random_config"
              class="flex items-center gap-1 px-1.5 py-0.5 rounded bg-soft text-[9px] font-semibold text-secondary"
            >
              <div
                :class="task.random_config ? 'i-ep-opportunity' : 'i-ep-timer'"
                class="text-[10px]"
              ></div>
              {{ task.random_config ? "Random" : "Cron" }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <nav class="hidden lg:flex items-center gap-0.5 rounded-md border border-light bg-base p-0.5">
        <button
          v-for="t in detailTabs"
          :key="t.value"
          class="h-8 flex items-center gap-1.5 px-3 rounded text-[11px] font-semibold transition-colors cursor-pointer select-none"
          :class="
            activeTab === t.value
              ? 'bg-card text-primary shadow-sm'
              : 'text-muted hover:text-default hover:bg-soft'
          "
          @click="emit('update:activeTab', t.value as TaskDetailTab)"
        >
          <div :class="t.icon" class="text-xs"></div>
          {{
            {
              log: "控制台",
              var: "变量",
              script: "编辑器",
              info: "详情",
            }[t.value] || t.label
          }}
        </button>
      </nav>

      <div class="lg:hidden flex items-center gap-0.5 rounded-md border border-light bg-base p-0.5">
        <button
          v-for="t in detailTabs"
          :key="t.value"
          class="w-8 h-8 flex-center rounded transition-colors cursor-pointer"
          :class="activeTab === t.value ? 'bg-card text-primary shadow-sm' : 'text-muted'"
          @click="emit('update:activeTab', t.value as TaskDetailTab)"
        >
          <div :class="t.icon"></div>
        </button>
      </div>

      <div class="flex items-center gap-1">
        <button
          v-if="activeTab === 'log'"
          class="w-8 h-8 flex-center rounded-md transition-colors cursor-pointer"
          :class="
            showSearch
              ? 'bg-primary text-white '
              : 'text-secondary hover:bg-soft hover:text-primary bg-card border border-light'
          "
          title="搜索日志"
          @click="emit('toggle-search')"
        >
          <div class="i-ep-search"></div>
        </button>

        <button
          v-if="activeTab === 'log' && !isMobile"
          class="w-8 h-8 flex-center rounded-md text-secondary hover:bg-soft hover:text-primary transition-colors bg-card border border-light cursor-pointer"
          title="打开日志窗口"
          @click="emit('open-log-window')"
        >
          <div class="i-ep-copy-document"></div>
        </button>

        <el-dropdown trigger="click" @command="handleCommand">
          <button
            type="button"
            class="w-8 h-8 flex-center rounded-md text-secondary hover:bg-soft hover:text-primary transition-colors bg-card border border-light outline-none cursor-pointer"
            title="更多任务操作"
            aria-label="更多任务操作"
          >
            <div class="i-ep-more-filled"></div>
          </button>
          <template #dropdown>
            <el-dropdown-menu class="modern-dropdown">
              <el-dropdown-item command="edit_config">
                <div class="flex items-center gap-2 text-primary">
                  <div class="i-ep-edit"></div>
                  任务设置
                </div>
              </el-dropdown-item>
              <el-dropdown-item command="edit_script">
                <div class="flex items-center gap-2">
                  <div class="i-ep-document"></div>
                  编辑脚本
                </div>
              </el-dropdown-item>
              <el-dropdown-item command="share">
                <div class="flex items-center gap-2">
                  <div class="i-ep-share"></div>
                  分享资源
                </div>
              </el-dropdown-item>
              <el-dropdown-item divided command="download_log">
                <div class="flex items-center gap-2">
                  <div class="i-ep-download"></div>
                  导出日志
                </div>
              </el-dropdown-item>
              <el-dropdown-item command="clear_screen">
                <div class="flex items-center gap-2">
                  <div class="i-ep-delete"></div>
                  清空控制台
                </div>
              </el-dropdown-item>
              <el-dropdown-item divided command="delete_task" class="!text-rose-500">
                <div class="flex items-center gap-2">
                  <div class="i-ep-delete"></div>
                  物理删除任务
                </div>
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
import type { Task } from "@/types";
import {
  isTaskDetailMoreCommand,
  type TaskDetailMoreCommand,
} from "../../composables/taskPageTypes";
import { detailTabs } from "../../composables/useTaskPresentation";

type TaskDetailTab = "log" | "script" | "var" | "info";

defineProps<{
  task: Task;
  activeTab: TaskDetailTab;
  isMobile: boolean;
  showSearch: boolean;
}>();

const emit = defineEmits<{
  (event: "back"): void;
  (event: "command", command: TaskDetailMoreCommand): void;
  (event: "open-log-window"): void;
  (event: "toggle-search"): void;
  (event: "update:activeTab", tab: TaskDetailTab): void;
}>();

const handleCommand = (command: unknown) => {
  if (isTaskDetailMoreCommand(command)) {
    emit("command", command);
  }
};
</script>
