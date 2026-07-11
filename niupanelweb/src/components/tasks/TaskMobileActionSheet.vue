<template>
  <el-drawer
    v-model="visibleValue"
    direction="btt"
    size="auto"
    :with-header="false"
    class="action-sheet-drawer"
    append-to-body
  >
    <div class="rounded-t-md bg-card px-4 pb-8 pt-3 text-center">
      <div class="mx-auto mb-4 h-1 w-9 rounded-full bg-muted/30"></div>
      <div class="mb-4 flex items-center gap-3 border-b border-light pb-4 text-left">
        <div
          class="accent-subtle h-10 w-10 rounded-md text-xl flex-center"
        >
          <div :class="task ? getEnvIcon(task) : 'i-ep-document'"></div>
        </div>
        <div class="flex flex-col min-w-0">
          <span class="text-base font-bold text-default truncate">
            {{ task?.name }}
          </span>
          <span class="text-[10px] font-medium text-muted">
            任务 ID：#{{ task?.id }}
          </span>
        </div>
      </div>
      <div class="grid grid-cols-3 gap-2">
        <button
          v-for="act in actions"
          :key="act.command"
          type="button"
          class="flex min-h-[72px] flex-col items-center justify-center gap-2 rounded-md border border-light bg-base px-2 py-3 transition-colors hover:bg-soft"
          @click="selectAction(act.command)"
        >
          <div
            class="h-8 w-8 rounded-md bg-soft text-lg flex-center"
            :class="act.color"
          >
            <div :class="act.icon"></div>
          </div>
          <span class="text-[10px] font-semibold leading-tight text-secondary">
            {{ act.label }}
          </span>
        </button>
      </div>
    </div>
  </el-drawer>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Task } from "@/types";
import type { TaskMobileActionCommand } from "../../composables/taskPageTypes";
import { getEnvIcon } from "../../composables/useTaskPresentation";

const props = defineProps<{
  task: Task | null;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "command", command: TaskMobileActionCommand): void;
  (event: "update:visible", visible: boolean): void;
}>();

const visibleValue = computed({
  get: () => props.visible,
  set: (value: boolean) => emit("update:visible", value),
});

const actions = computed(() => [
  {
    label: "查看日志",
    command: "logs" as const,
    icon: "i-ep-document",
    color: "text-blue-500",
  },
  {
    label: "编辑任务",
    command: "edit" as const,
    icon: "i-ep-edit",
    color: "text-amber-500",
  },
  {
    label: "编辑脚本",
    command: "script" as const,
    icon: "i-ep-document",
    color: "text-purple-500",
  },
  {
    label: "定时规则",
    command: "cron" as const,
    icon: "i-ep-clock",
    color: "text-orange-400",
  },
  {
    label: "环境变量",
    command: "variables" as const,
    icon: "i-ep-key",
    color: "text-emerald-500",
  },
  {
    label: "分享资源",
    command: "share" as const,
    icon: "i-ep-share",
    color: "text-purple-500",
  },
  {
    label: props.task?.is_pinned ? "取消置顶" : "置顶任务",
    command: props.task?.is_pinned ? ("unpin" as const) : ("pin" as const),
    icon: props.task?.is_pinned ? "i-ep-bottom" : "i-ep-top",
    color: "text-orange-500",
  },
  {
    label: "复制路径",
    command: "copy" as const,
    icon: "i-ep-copy-document",
    color: "text-blue-400",
  },
  {
    label: "删除任务",
    command: "delete" as const,
    icon: "i-ep-delete",
    color: "text-rose-500",
  },
]);

const selectAction = (command: TaskMobileActionCommand) => {
  emit("command", command);
  emit("update:visible", false);
};
</script>
