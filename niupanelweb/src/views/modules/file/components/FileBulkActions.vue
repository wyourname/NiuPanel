<template>
  <MobileSelectionBar v-if="appStore.isMobile" class="file-selection-bar" label="已选文件操作" :count="count" :busy="busy" :is-all-selected="isAllSelected" :actions="mobileActions" @select-all="emit('select-all')" @cancel="emit('cancel')" @command="mobileHandlers[$event]?.()" />

  <transition name="el-zoom-in-top">
    <div
      v-if="count > 0 && !appStore.isMobile"
      class="shrink-0 border-b border-light bg-card px-3 py-2"
      :inert="busy"
    >
      <div class="grid min-h-8 grid-cols-[auto_repeat(7,minmax(0,1fr))] items-center gap-1">
        <div class="flex shrink-0 items-center justify-center gap-1.5 px-1">
          <span class="text-[13px] font-black text-primary">{{ count }}</span>
          <span class="whitespace-nowrap text-[11px] font-bold text-muted">已选</span>
        </div>

        <button
          type="button"
          class="h-7 w-full min-w-0 cursor-pointer justify-center rounded-md px-2 text-[11px] font-semibold text-secondary inline-flex items-center gap-1.5 transition-colors hover:bg-subtle hover:text-default"
          @click="emit('select-all')"
        >
          <div :class="isAllSelected ? 'i-ep-close' : 'i-ep-check'" class="text-[13px]"></div>
          {{ isAllSelected ? "取消全选" : "全选" }}
        </button>
        <button
          type="button"
          class="h-7 w-full min-w-0 cursor-pointer justify-center rounded-md px-2 text-[11px] font-semibold text-secondary inline-flex items-center gap-1.5 transition-colors hover:bg-subtle hover:text-default"
          @click="emit('copy')"
        >
          <div class="i-ep-copy-document text-[13px]"></div>
          复制
        </button>
        <button
          type="button"
          class="h-7 w-full min-w-0 cursor-pointer justify-center rounded-md px-2 text-[11px] font-semibold text-secondary inline-flex items-center gap-1.5 transition-colors hover:bg-subtle hover:text-default"
          @click="emit('cut')"
        >
          <div class="i-ep-scissor text-[13px]"></div>
          剪切
        </button>
        <button
          type="button"
          class="h-7 w-full min-w-0 cursor-pointer justify-center rounded-md px-2 text-[11px] font-semibold text-secondary inline-flex items-center gap-1.5 transition-colors hover:bg-subtle hover:text-default"
          @click="emit('move')"
        >
          <div class="i-ep-position text-[13px]"></div>
          移动
        </button>
        <button
          type="button"
          class="h-7 w-full min-w-0 cursor-pointer justify-center rounded-md px-2 text-[11px] font-semibold text-secondary inline-flex items-center gap-1.5 transition-colors hover:bg-subtle hover:text-default"
          @click="emit('download')"
        >
          <div class="i-ep-download text-[13px]"></div>
          下载
        </button>

        <button
          type="button"
          class="h-7 w-full min-w-0 cursor-pointer justify-center rounded-md px-2 text-[11px] font-semibold text-muted inline-flex items-center gap-1.5 transition-colors hover:bg-subtle hover:text-default"
          @click="emit('cancel')"
        >
          <div class="i-ep-close text-[13px]"></div>
          取消
        </button>
        <button
          type="button"
          class="h-7 w-full min-w-0 cursor-pointer justify-center rounded-md px-2 text-[11px] font-semibold text-rose-600 inline-flex items-center gap-1.5 transition-colors hover:bg-rose-500/10 dark:text-rose-300"
          @click="emit('delete')"
        >
          <div class="i-ep-delete text-[13px]"></div>
          删除
        </button>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import MobileSelectionBar from "@/components/common/MobileSelectionBar.vue";
import { useAppStore } from "../../../../stores/app";

defineProps<{
  count: number;
  isAllSelected: boolean;
  busy?: boolean;
}>();

const emit = defineEmits<{
  (event: "cancel"): void;
  (event: "copy"): void;
  (event: "cut"): void;
  (event: "delete"): void;
  (event: "download"): void;
  (event: "move"): void;
  (event: "select-all"): void;
}>();

const mobileActions = [
  { command: 'copy', label: '复制', icon: 'i-ep-copy-document' },
  { command: 'cut', label: '剪切', icon: 'i-ep-scissor' },
  { command: 'move', label: '移动', icon: 'i-ep-position' },
  { command: 'download', label: '打包下载', icon: 'i-ep-download' },
  { command: 'delete', label: '删除', icon: 'i-ep-delete', danger: true },
];
const mobileHandlers: Record<string, () => void> = { copy: () => emit('copy'), cut: () => emit('cut'), move: () => emit('move'), download: () => emit('download'), delete: () => emit('delete') };
const appStore = useAppStore();
</script>
