<template>
  <BulkActionBar
    v-if="appStore.isMobile"
    :count="count"
    :show-select-all="true"
    :is-all-selected="isAllSelected"
    @select-all="emit('select-all')"
    @cancel="emit('cancel')"
    @delete="emit('delete')"
  >
    <template #actions>
      <div class="flex items-center gap-2">
        <el-button type="primary" plain size="small" @click="emit('copy')">
          复制
        </el-button>
        <el-button type="primary" plain size="small" @click="emit('download')">
          打包下载
        </el-button>
        <el-button type="danger" plain size="small" @click="emit('cut')">
          剪切
        </el-button>
        <el-button type="success" plain size="small" @click="emit('move')">
          <div class="i-ep-position mr-1"></div>
          移动
        </el-button>
      </div>
    </template>
  </BulkActionBar>

  <transition name="el-zoom-in-top">
    <div
      v-if="count > 0 && !appStore.isMobile"
      class="shrink-0 border-b border-light bg-card px-3 py-2"
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
import { useAppStore } from "../../../../stores/app";
import BulkActionBar from "../../../../components/common/BulkActionBar.vue";

defineProps<{
  count: number;
  isAllSelected: boolean;
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

const appStore = useAppStore();
</script>
