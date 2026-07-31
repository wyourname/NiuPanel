<template>
  <div class="px-3 py-2 border-b border-light flex items-center justify-between gap-3 flex-wrap bg-base/40">
    <div class="flex-1 min-w-0 flex items-center gap-3">
      <SegmentedTabs
        :model-value="activeTab"
        :items="tabItems"
        @update:model-value="handleTabChange"
      />
      <div v-if="initialLoading" class="i-ep-loading animate-spin text-primary text-xs"></div>
    </div>

    <div class="flex items-center gap-2 flex-wrap sm:justify-end">
      <div class="flex items-center gap-1.5 flex-1 sm:flex-none justify-end min-w-0">
        <transition name="el-fade-in-linear">
          <el-input
            v-if="!isMobile || isSearchExpanded"
            ref="searchInputRef"
            :model-value="searchQuery"
            placeholder="搜索变量名或备注..."
            size="small"
            class="modern-input flex-1 !w-full sm:!w-44 lg:!w-60"
            clearable
            @update:model-value="handleSearchInput"
            @blur="isSearchExpanded = !!searchQuery"
          >
            <template #prefix>
              <div class="i-ep-search text-xs opacity-50"></div>
            </template>
          </el-input>
        </transition>

        <button
          v-if="isMobile && !isSearchExpanded"
          type="button"
          class="h-8 w-8 rounded-md bg-soft/40 text-primary flex-center transition-colors hover:bg-soft"
          title="搜索变量"
          aria-label="搜索变量"
          @click="expandSearch"
        >
          <div class="i-ep-search text-lg"></div>
        </button>
      </div>

      <el-dropdown trigger="click" @command="handleActionCommand">
        <ToolbarButton v-if="!isMobile" variant="primary" size="small">
          <template #icon>
            <div class="i-ep-plus"></div>
          </template>
          <span>新建变量</span>
        </ToolbarButton>
        <button
          v-else
          type="button"
          class="h-8 w-8 rounded-md border border-light bg-card text-secondary flex-center transition-colors hover:bg-soft hover:text-default"
          title="导入或导出"
          aria-label="导入或导出变量"
        >
          <span class="i-ep-more-filled"></span>
        </button>
        <template #dropdown>
          <el-dropdown-menu class="modern-dropdown">
            <el-dropdown-item v-if="!isMobile" command="create">
              <div class="flex items-center gap-2">
                <div class="i-ep-plus"></div>
                手动录入
              </div>
            </el-dropdown-item>
            <el-dropdown-item command="import" divided>
              <div class="flex items-center gap-2">
                <div class="i-ep-upload"></div>
                导入 JSON
              </div>
            </el-dropdown-item>
            <el-dropdown-item command="export">
              <div class="flex items-center gap-2">
                <div class="i-ep-download"></div>
                导出数据
              </div>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </div>
</template>

<script setup lang="ts">
import { nextTick, ref } from "vue";
import SegmentedTabs from "../common/SegmentedTabs.vue";
import ToolbarButton from "../common/ToolbarButton.vue";

const props = defineProps<{
  activeTab: string;
  initialLoading: boolean;
  isMobile: boolean;
  searchQuery: string;
}>();

const emit = defineEmits<{
  (event: "create"): void;
  (event: "export-json"): void;
  (event: "import-json"): void;
  (event: "search"): void;
  (event: "tab-change"): void;
  (event: "update:activeTab", tab: string): void;
  (event: "update:searchQuery", query: string): void;
}>();

const tabItems = [
  { label: "脚本变量", value: "Script" },
  { label: "全局变量", value: "Global" },
];

const isSearchExpanded = ref(false);
const searchInputRef = ref<{ focus: () => void } | null>(null);

const expandSearch = () => {
  isSearchExpanded.value = true;
  nextTick(() => {
    searchInputRef.value?.focus();
  });
};

const handleSearchInput = (value: string) => {
  emit("update:searchQuery", value);
  emit("search");
};

const handleTabChange = (tab: string) => {
  emit("update:activeTab", tab);
  emit("tab-change");
};

const handleActionCommand = (command: string | number | object) => {
  if (command === "create") emit("create");
  else if (command === "import") emit("import-json");
  else if (command === "export") emit("export-json");
};
</script>
