<template>
  <OverlayDrawer
    v-model:visible="visibleValue"
    title="任务操作"
    variant="sheet"
    content-preset="list"
    append-to-body
  >
    <template #title>
      <div class="flex min-w-0 items-center gap-2">
        <div class="accent-subtle h-8 w-8 shrink-0 rounded-md text-base flex-center">
          <span :class="task ? getEnvIcon(task) : 'i-ep-document'"></span>
        </div>
        <div class="min-w-0">
          <div class="truncate text-[13px] font-bold text-default">{{ task?.name || "任务操作" }}</div>
          <div class="truncate text-[10px] font-medium text-muted">任务 ID：#{{ task?.id }}</div>
        </div>
      </div>
    </template>

    <div class="grid grid-cols-3 gap-2">
      <button
        v-for="act in actions"
        :key="act.command"
        type="button"
        class="flex min-h-[72px] cursor-pointer flex-col items-center justify-center gap-2 rounded-md border border-light bg-base px-2 py-3 transition-colors hover:bg-soft focus-visible:outline-2 focus-visible:outline-primary"
        @click="selectAction(act.command)"
      >
        <span
          class="h-8 w-8 rounded-md bg-soft text-lg flex-center"
          :class="act.color"
        >
          <span :class="act.icon"></span>
        </span>
        <span class="text-[10px] font-semibold leading-tight text-secondary">
          {{ act.label }}
        </span>
      </button>
    </div>
  </OverlayDrawer>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Task } from "@/types";
import type { TaskMobileActionCommand } from "../../composables/taskPageTypes";
import { getEnvIcon } from "../../composables/useTaskPresentation";
import OverlayDrawer from "../common/OverlayDrawer.vue";

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
