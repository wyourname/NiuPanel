<template>
  <div
    class="absolute items-center z-0 px-0 overflow-hidden"
    :class="[
      isMobile
        ? 'inset-0 flex justify-between'
        : 'inset-y-0.5 inset-x-2 flex justify-between rounded-sm',
      offset === 0 ? 'opacity-0 pointer-events-none' : 'opacity-100',
    ]"
  >
    <div
      :class="offset >= 0 ? 'opacity-100' : 'opacity-0 pointer-events-none'"
      class="h-full flex items-center justify-start transition-opacity duration-300"
      :style="{ width: `${maxLeft}px` }"
    >
      <button
        class="flex h-full w-full flex-col items-center justify-center gap-1 border-none font-bold text-white outline-none transition-colors active:brightness-90"
        :class="isTaskActive ? 'bg-rose-500' : 'bg-emerald-500'"
        @click.stop="emit('action', isTaskActive ? 'stop' : 'run')"
      >
        <div
          :class="isTaskActive ? 'i-ep-video-pause' : 'i-ep-video-play'"
          class="text-[20px]"
        ></div>
        <span class="text-[10px]">{{
          isTaskActive ? "停止" : "运行"
        }}</span>
      </button>
    </div>

    <div
      :class="offset <= 0 ? 'opacity-100' : 'opacity-0 pointer-events-none'"
      class="h-full flex items-center justify-end font-bold transition-opacity duration-300"
      :style="{ width: `${maxRight}px` }"
    >
      <button
        class="flex h-full flex-1 flex-col items-center justify-center gap-1 border-none font-bold text-white outline-none transition-colors active:brightness-90"
        :class="task.enabled ? 'bg-gray-500 dark:bg-gray-600' : 'bg-blue-500'"
        @click.stop="emit('action', task.enabled ? 'disable' : 'enable')"
      >
        <div
          :class="task.enabled ? 'i-ep-turn-off' : 'i-ep-open'"
          class="text-[18px]"
        ></div>
        <span class="text-[10px]">{{
          task.enabled ? "禁用" : "启用"
        }}</span>
      </button>
      <button
        class="h-full flex-1 bg-gray-400 dark:bg-gray-600 text-white flex flex-col gap-1 items-center justify-center border-none outline-none active:brightness-90 transition-all"
        @click.stop="emit('action', 'more')"
      >
        <div class="i-ep-more-filled text-[20px]"></div>
        <span class="text-[10px]">更多</span>
      </button>
      <button
        class="h-full flex-1 bg-rose-500 text-white flex flex-col gap-1 items-center justify-center border-none outline-none active:brightness-90 transition-all"
        @click.stop="emit('action', 'delete')"
      >
        <div class="i-ep-delete-filled text-[20px]"></div>
        <span class="text-[10px]">删除</span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Task } from "@/types";
import type { TaskCardSwipeAction } from "../../composables/useTaskCardSwipeActions";

const props = defineProps<{
  isMobile: boolean;
  maxLeft: number;
  maxRight: number;
  offset: number;
  task: Task;
}>();

const emit = defineEmits<{
  (event: "action", action: TaskCardSwipeAction): void;
}>();

const isTaskActive = computed(() =>
  ["Running", "Paused"].includes(props.task.status),
);
</script>
