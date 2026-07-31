<template>
  <div class="flex-1 overflow-y-auto px-5 py-4 custom-scrollbar">
    <div v-if="loading" class="grid grid-cols-1 gap-3">
      <el-skeleton v-for="item in 6" :key="item" animated :loading="true">
        <template #template>
          <div class="surface-list !rounded-lg p-4">
            <el-skeleton-item variant="text" style="width: 32%" />
            <el-skeleton-item
              variant="text"
              style="width: 18%; margin-top: 14px"
            />
          </div>
        </template>
      </el-skeleton>
    </div>

    <div
      v-else-if="packages.length === 0"
      class="h-full min-h-[280px] flex-col-center text-center text-[var(--editor-text)]/55"
    >
      <div class="i-ep-box text-5xl mb-4"></div>
      <div class="text-sm font-semibold">
        {{ searchQuery ? "没有匹配的依赖" : "暂未安装依赖" }}
      </div>
      <div class="text-xs mt-2">可以直接批量粘贴包名，支持一行一个。</div>
    </div>

    <div v-else class="grid grid-cols-1 gap-3">
      <div
        v-for="row in packages"
        :key="row.name"
        class="surface-list !rounded-lg"
      >
        <div class="surface-list__row">
          <div
            class="h-10 w-10 shrink-0 rounded-md bg-soft text-primary flex-center"
          >
            <div class="i-ep-box"></div>
          </div>
          <div class="flex-1 min-w-0">
            <div
              class="text-sm font-semibold text-[var(--editor-text)] truncate"
            >
              {{ row.name }}
            </div>
            <div class="text-xs font-mono text-[var(--editor-text)]/50 mt-1">
              {{ row.version }}
            </div>
          </div>
          <button
            type="button"
            class="toolbar-button toolbar-button--danger"
            @click="emit('uninstall', row.name)"
          >
            <div class="i-ep-delete"></div>
            卸载
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Package } from "@/types";

defineProps<{
  loading: boolean;
  packages: Package[];
  searchQuery: string;
}>();

const emit = defineEmits<{
  (event: "uninstall", packageName: string): void;
}>();
</script>
