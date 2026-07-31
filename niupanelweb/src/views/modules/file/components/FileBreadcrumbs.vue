<template>
  <div
    class="z-10 flex h-8 shrink-0 items-center gap-1 overflow-x-auto no-scrollbar text-xs"
  >
    <button
      type="button"
      class="h-6 w-6 shrink-0 rounded-md text-muted flex-center transition-colors hover:bg-soft hover:text-primary disabled:cursor-not-allowed disabled:opacity-10"
      title="返回上级目录"
      aria-label="返回上级目录"
      :disabled="currentPath === '/' || currentPath === ''"
      @click="emit('back')"
    >
      <div class="i-ep-back text-[13px]"></div>
    </button>

    <div class="h-3 w-px shrink-0 bg-light/60"></div>

    <div
      class="flex items-center gap-0.5 overflow-x-auto text-[10px] font-bold text-muted no-scrollbar"
    >
      <div
        class="flex shrink-0 cursor-pointer items-center gap-1 rounded-md px-1.5 py-0.5 transition-all"
        :class="currentPath === '/' || currentPath === '' ? 'text-primary' : 'hover:bg-soft hover:text-default'"
        @click="emit('navigate', '')"
      >
        <div class="i-ep-house text-[12px]"></div>
        <span>根目录</span>
      </div>

      <template v-for="(item, index) in collapsedBreadcrumbs" :key="index">
        <div class="i-ep-arrow-right mx-0.5 shrink-0 text-[8px] opacity-30"></div>

        <el-dropdown
          v-if="item.type === 'ellipsis'"
          trigger="click"
          @command="handleCommand"
        >
          <span
            class="shrink-0 cursor-pointer rounded-md px-1.5 py-0.5 font-bold transition-colors hover:bg-soft hover:text-primary"
            >...</span
          >
          <template #dropdown>
            <el-dropdown-menu class="modern-dropdown">
              <el-dropdown-item
                v-for="hidden in item.items"
                :key="hidden.path"
                :command="hidden.path"
              >
                {{ hidden.name }}
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>

        <span
          v-else
          class="max-w-[132px] shrink-0 cursor-pointer truncate rounded-md px-1.5 py-0.5 transition-all"
          :class="index === collapsedBreadcrumbs.length - 1
            ? 'text-default font-bold'
            : 'hover:bg-soft hover:text-default'"
          @click="emit('navigate', item.path)"
        >
          {{ item.name }}
        </span>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Breadcrumb } from "../../../../composables/useFileOperations";

defineProps<{
  currentPath: string;
  collapsedBreadcrumbs: Breadcrumb[];
}>();

const emit = defineEmits<{
  (event: "back"): void;
  (event: "navigate", path: string): void;
}>();

const handleCommand = (path: unknown) => {
  if (typeof path === "string") emit("navigate", path);
};
</script>
