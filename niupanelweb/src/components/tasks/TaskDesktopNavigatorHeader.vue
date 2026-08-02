<template>
  <transition name="el-fade-in" mode="out-in">
    <div
      v-if="selectionMode"
      key="select-header"
      class="h-14 px-3 flex items-center justify-between border-b border-base bg-card shrink-0"
    >
      <button
        class="h-8 px-2 rounded-md text-[11px] font-semibold text-rose-500 hover:bg-rose-500/5 transition-colors"
        @click="emit('cancel-selection')"
      >
        退出多选
      </button>
      <span class="text-[12px] font-bold text-default">已选 {{ selectedCount }} 项</span>
      <button
        class="h-8 px-2 rounded-md text-[11px] font-semibold text-primary hover:bg-primary/5 transition-colors"
        @click="emit('select-all')"
      >
        {{ isAllSelected ? "取消全选" : "选择全部" }}
      </button>
    </div>

    <div
      v-else
      key="normal-header"
      class="shrink-0 border-b border-base bg-card px-3.5 py-3"
    >
      <div class="mb-3 flex h-9 items-center justify-between">
        <div class="min-w-0">
          <div class="flex items-baseline gap-2">
            <h1 class="m-0 text-[18px] font-bold text-default">任务</h1>
            <span class="text-[11px] font-semibold text-muted">{{ totalTasks }} 个</span>
          </div>
        </div>
        <div class="flex items-center gap-1">
          <button
            type="button"
            class="h-8 w-8 rounded-md text-secondary flex-center transition-colors hover:bg-soft hover:text-default"
            title="批量操作"
            @click="selectionModeValue = true"
          >
            <span class="i-ep-finished text-[14px]"></span>
          </button>
          <el-dropdown trigger="click">
            <button
              type="button"
              class="h-8 rounded-md bg-primary px-2.5 text-[11px] font-bold text-white flex items-center gap-1.5 transition-opacity hover:opacity-90 outline-none"
              title="新建任务"
            >
              <span class="i-ep-plus text-[15px]"></span>
              新建
            </button>
            <template #dropdown>
              <el-dropdown-menu class="modern-dropdown">
                <el-dropdown-item @click="emit('create')">
                  <span class="i-ep-plus mr-2"></span>新建任务
                </el-dropdown-item>
                <el-dropdown-item @click="emit('quick-create')">
                  <span class="i-ep-link mr-2"></span>从 URL 导入
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </div>
      <label class="group flex h-9 items-center rounded-md border border-light bg-base px-2.5 focus-within:border-primary/40">
        <span class="i-ep-search mr-2 text-[13px] text-muted group-focus-within:text-primary"></span>
        <input
          v-model="searchValue"
          placeholder="搜索任务名称、脚本或环境"
          class="task-search-input h-full min-w-0 flex-1 border-none bg-transparent text-[12px] font-medium text-default outline-none"
        />
      </label>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  isAllSelected: boolean;
  searchQuery: string;
  selectedCount: number;
  selectionMode: boolean;
  totalTasks: number;
}>();

const emit = defineEmits<{
  (event: "cancel-selection"): void;
  (event: "create"): void;
  (event: "quick-create"): void;
  (event: "select-all"): void;
  (event: "update:searchQuery", value: string): void;
  (event: "update:selectionMode", value: boolean): void;
}>();

const searchValue = computed({
  get: () => props.searchQuery,
  set: (value: string) => emit("update:searchQuery", value),
});

const selectionModeValue = computed({
  get: () => props.selectionMode,
  set: (value: boolean) => emit("update:selectionMode", value),
});
</script>

<style scoped>
.task-search-input::placeholder {
  color: var(--text-muted);
  opacity: 0.7;
}
</style>
