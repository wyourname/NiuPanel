<template>
  <div class="relative select-none touch-pan-y overflow-hidden bg-transparent group">
    <TaskCardSwipeActions
      :is-mobile="appStore.isMobile"
      :max-left="maxLeft"
      :max-right="maxRight"
      :offset="offset"
      :task="task"
      @action="handleAction"
    />

    <div
      ref="cardRef"
      class="relative z-10 transition-all duration-200 ease-out"
      :class="[
        appStore.isMobile
          ? 'bg-card border-b border-light'
          : isSelected
            ? 'bg-soft border-l-2 border-l-primary border-b border-light'
            : 'bg-card border-l-2 border-l-transparent border-b border-light hover:bg-subtle',
        isSwiping ? '!transition-none' : '',
      ]"
      :style="{ transform: `translate3d(${offset}px, 0, 0)` }"
      @click="handleCardClick"
      @touchstart="startLongPress"
      @touchend="cancelLongPress"
      @touchmove="cancelLongPress"
    >
      <TaskCardContent
        :is-mobile="appStore.isMobile"
        :is-selected="isSelected"
        :selection-mode="selectionMode"
        :task="task"
        @more-actions="emit('more-actions', task)"
        @run="emit('run', task.id)"
        @selection-change="handleSelectionChange"
        @stop="emit('stop', task.id)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { toRef } from "vue";
import { useAppStore } from "../../stores/app";
import {
  useTaskCardSwipeActions,
  type TaskCardSwipeAction,
} from "../../composables/useTaskCardSwipeActions";
import type { Task } from "@/types";
import TaskCardContent from "./TaskCardContent.vue";
import TaskCardSwipeActions from "./TaskCardSwipeActions.vue";

const props = defineProps<{
  task: Task;
  isSelected: boolean;
  selectionMode: boolean;
}>();

const emit = defineEmits<{
  (event: "delete", id: number): void;
  (event: "disable", id: number): void;
  (event: "edit", task: Task): void;
  (event: "edit-cron", task: Task): void;
  (event: "enable", id: number): void;
  (event: "enter-selection", task: Task): void;
  (event: "logs", task: Task): void;
  (event: "more-actions", task: Task): void;
  (event: "pause", id: number): void;
  (event: "resume", id: number): void;
  (event: "run", id: number): void;
  (event: "selection-change", task: Task, value: boolean): void;
  (event: "stop", id: number): void;
}>();

const appStore = useAppStore();

const emitSwipeAction = (action: TaskCardSwipeAction, task: Task) => {
  const id = task.id;

  if (action === "run") emit("run", id);
  else if (action === "stop") emit("stop", id);
  else if (action === "enable") emit("enable", id);
  else if (action === "disable") emit("disable", id);
  else if (action === "more") emit("more-actions", task);
  else if (action === "delete") emit("delete", id);
};

const {
  cancelLongPress,
  cardRef,
  handleAction,
  handleCardClick,
  handleSelectionChange,
  isSwiping,
  maxLeft,
  maxRight,
  offset,
  startLongPress,
} = useTaskCardSwipeActions({
  isSelected: toRef(props, "isSelected"),
  onAction: emitSwipeAction,
  onEnterSelection: (task) => emit("enter-selection", task),
  onOpenLogs: (task) => emit("logs", task),
  onSelectionChange: (task, selected) =>
    emit("selection-change", task, selected),
  selectionMode: toRef(props, "selectionMode"),
  task: toRef(props, "task"),
});
</script>
