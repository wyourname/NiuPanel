<template>
  <header :aria-label="task.name" class="task-detail-header" :class="{ 'task-detail-header--mobile': isMobile }">
    <div class="task-detail-heading">
      <button v-if="isMobile" type="button" class="task-detail-action task-detail-back" aria-label="返回任务列表" @click="emit('back')">
        <span class="i-ep-arrow-left" aria-hidden="true"></span>
      </button>
    <nav class="task-detail-tabs" aria-label="任务详情导航">
      <button v-for="tab in detailTabs" :key="tab.value" type="button" :class="{ 'is-active': activeTab === tab.value }" :aria-pressed="activeTab === tab.value" @click="emit('update:activeTab', tab.value as TaskDetailTab)">
        <span :class="tab.value === 'log' ? 'i-carbon-terminal' : tab.icon" aria-hidden="true"></span>
        {{ tabLabels[tab.value] || tab.label }}
      </button>
    </nav>
      <div class="task-detail-actions">
        <button v-if="activeTab === 'log'" type="button" class="task-detail-action" :class="{ 'is-active': showSearch }" title="搜索日志" aria-label="搜索日志" :aria-pressed="showSearch" @click="emit('toggle-search')">
          <span class="i-ep-search" aria-hidden="true"></span>
        </button>
        <el-dropdown trigger="click" @command="handleCommand">
          <button type="button" class="task-detail-action" title="更多任务操作" aria-label="更多任务操作"><span class="i-ep-more-filled" aria-hidden="true"></span></button>
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
              <el-dropdown-item :command="task.enabled ? 'disable' : 'enable'">
                <div class="flex items-center gap-2">
                  <div :class="task.enabled ? 'i-ep-turn-off text-amber-500' : 'i-ep-open text-emerald-500'"></div>
                  {{ task.enabled ? '禁用任务' : '启用任务' }}
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

const tabLabels: Record<string, string> = { log: "控制台", var: "变量", script: "编辑器", info: "详情" };

const emit = defineEmits<{
  (event: "back"): void;
  (event: "command", command: TaskDetailMoreCommand): void;
  (event: "toggle-search"): void;
  (event: "update:activeTab", tab: TaskDetailTab): void;
}>();

const handleCommand = (command: unknown) => {
  if (isTaskDetailMoreCommand(command)) {
    emit("command", command);
  }
};
</script>

<style scoped>
.task-detail-header { position: sticky; top: 0; z-index: 30; flex-shrink: 0; min-width: 0; container-type: inline-size; border-bottom: 1px solid var(--border-light); background: var(--bg-card); }
.task-detail-heading { display: flex; align-items: center; gap: 12px; min-height: 56px; padding: 8px 16px; }
.task-detail-tabs { display: flex; flex: 0 1 auto; min-width: 0; gap: 4px; padding: 3px; margin-right: auto; border: 1px solid var(--border-light); border-radius: 8px; background: var(--surface-inset); }
.task-detail-tabs button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; min-width: 70px; min-height: 36px; padding: 0 10px; border-radius: 5px; color: var(--text-secondary); font-size: 12px; font-weight: 600; white-space: nowrap; transition: color .16s, background-color .16s; }
.task-detail-tabs button > span { font-size: 15px; }
.task-detail-tabs button:hover { color: var(--el-color-primary); }
.task-detail-tabs button.is-active { color: var(--text-default); background: var(--bg-card); box-shadow: var(--shadow-sm); }
.task-detail-actions { display: flex; align-items: center; flex-shrink: 0; gap: 6px; }
.task-detail-action { display: grid; place-items: center; width: 36px; height: 36px; flex-shrink: 0; border: 1px solid var(--border-light); border-radius: var(--radius-sm); background: var(--bg-card); color: var(--text-secondary); transition: background-color .16s, color .16s, border-color .16s; }
.task-detail-action:hover, .task-detail-action.is-active { background: var(--accent-subtle-bg); color: var(--accent-subtle-text); border-color: var(--accent-subtle-border); }
.task-detail-header button { cursor: pointer; }
.task-detail-header button:focus-visible { outline: 2px solid var(--el-color-primary); outline-offset: -2px; }
.task-detail-header--mobile .task-detail-action { width: 44px; height: 44px; }
.task-detail-header--mobile .task-detail-tabs { flex: 1; padding: 0; border: none; background: transparent; gap: 0; }
.task-detail-header--mobile .task-detail-tabs button { flex: 1; min-width: 44px; min-height: 44px; padding: 0 4px; }
@container (max-width: 520px) {
  .task-detail-heading { gap: 2px; padding: 6px 4px; }
  .task-detail-tabs { gap: 0; }
  .task-detail-tabs button { min-width: 44px; padding: 0 4px; }
  .task-detail-tabs button > span { display: none; }
  .task-detail-actions { gap: 0; }
}
</style>
