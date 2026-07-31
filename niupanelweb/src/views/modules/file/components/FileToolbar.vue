<template>
  <div
    v-if="!isMobile"
    class="shrink-0 border-b border-light bg-card px-2.5 py-1.5"
  >
    <div class="grid min-h-8 grid-cols-[minmax(0,1fr)_minmax(190px,280px)_auto] items-center gap-1.5">
      <div class="flex min-w-0 items-center gap-1">
        <button
          type="button"
          class="h-7 w-7 shrink-0 cursor-pointer rounded-md text-muted flex-center transition-colors hover:bg-subtle hover:text-default"
          title="刷新"
          @click="emit('refresh')"
        >
          <div
            class="i-ep-refresh text-[15px]"
            :class="{ 'animate-spin': loading }"
          ></div>
        </button>

        <FileBreadcrumbs
          class="min-w-0 flex-1 !border-none !bg-transparent !px-0"
          :current-path="currentPath"
          :collapsed-breadcrumbs="collapsedBreadcrumbs"
          @back="emit('back')"
          @navigate="(path) => emit('navigate', path)"
        />
      </div>

      <el-input
        ref="searchInputRef"
        :model-value="searchQuery"
        placeholder="搜索文件"
        size="small"
        class="modern-input !w-full"
        clearable
        @update:model-value="handleSearchInput"
      >
        <template #prefix>
          <div class="i-ep-search text-[12px] text-muted"></div>
        </template>
      </el-input>

      <div class="flex shrink-0 items-center justify-end gap-1">
        <transition name="el-zoom-in-center">
          <button
            v-if="clipboardFilesCount > 0"
            type="button"
            class="h-8 cursor-pointer rounded-md border border-amber-500/20 bg-amber-500/10 px-2.5 text-[11px] font-semibold text-amber-700 inline-flex items-center gap-1.5 transition-colors hover:bg-amber-500/16 dark:text-amber-300"
            @click="emit('paste')"
          >
            <div class="i-ep-copy-document text-[14px]"></div>
            粘贴 {{ clipboardFilesCount }}
          </button>
        </transition>

        <el-dropdown trigger="click" @command="handleSortCommand">
          <button
            type="button"
            class="h-8 cursor-pointer rounded-md px-2 text-[11px] font-semibold text-secondary inline-flex items-center gap-1.5 transition-colors hover:bg-subtle hover:text-default"
          >
            <div class="i-ep-sort text-[14px]"></div>
            {{ sortLabel }}
          </button>
          <template #dropdown>
            <el-dropdown-menu class="modern-dropdown">
              <el-dropdown-item command="name">
                <div class="flex items-center gap-2">
                  <div :class="sortMode === 'name' ? 'i-ep-check' : 'i-ep-sort'"></div>
                  名称
                </div>
              </el-dropdown-item>
              <el-dropdown-item command="mtime">
                <div class="flex items-center gap-2">
                  <div :class="sortMode === 'mtime' ? 'i-ep-check' : 'i-ep-clock'"></div>
                  修改时间
                </div>
              </el-dropdown-item>
              <el-dropdown-item command="size">
                <div class="flex items-center gap-2">
                  <div :class="sortMode === 'size' ? 'i-ep-check' : 'i-ep-files'"></div>
                  大小
                </div>
              </el-dropdown-item>
              <el-dropdown-item command="type">
                <div class="flex items-center gap-2">
                  <div :class="sortMode === 'type' ? 'i-ep-check' : 'i-ep-collection'"></div>
                  类型
                </div>
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>

        <div class="flex h-8 shrink-0 items-center gap-1 rounded-md bg-subtle p-0.5">
          <button
            type="button"
            class="h-7 w-7 cursor-pointer rounded-md flex-center transition-colors"
            :class="viewMode === 'detail' ? 'bg-white text-primary shadow-sm dark:bg-white/14' : 'text-muted hover:text-default'"
            title="详情视图"
            @click="emit('update:viewMode', 'detail')"
          >
            <div class="i-ep-list text-[14px]"></div>
          </button>
          <button
            type="button"
            class="h-7 w-7 cursor-pointer rounded-md flex-center transition-colors"
            :class="viewMode === 'grid' ? 'bg-white text-primary shadow-sm dark:bg-white/14' : 'text-muted hover:text-default'"
            title="网格视图"
            @click="emit('update:viewMode', 'grid')"
          >
            <div class="i-ep-grid text-[14px]"></div>
          </button>
        </div>

        <button
          type="button"
          class="h-8 cursor-pointer rounded-md px-2.5 text-[11px] font-semibold text-secondary inline-flex items-center gap-1.5 transition-colors hover:bg-subtle hover:text-default"
          @click="emit('trigger-upload')"
        >
          <div class="i-ep-upload-filled text-[14px]"></div>
          上传
        </button>

        <el-dropdown trigger="click" @command="handleCreateCommand">
          <button
            type="button"
            class="h-8 cursor-pointer rounded-md bg-primary px-2.5 text-[11px] font-semibold text-white inline-flex items-center gap-1.5 transition-colors hover:bg-primary/90"
          >
            <div class="i-ep-plus text-[14px]"></div>
            新建
          </button>
          <template #dropdown>
            <el-dropdown-menu class="modern-dropdown">
              <el-dropdown-item command="file">
                <div class="flex items-center gap-2">
                  <div class="i-ep-document"></div>
                  新建文件
                </div>
              </el-dropdown-item>
              <el-dropdown-item command="directory">
                <div class="flex items-center gap-2">
                  <div class="i-ep-folder"></div>
                  新建目录
                </div>
              </el-dropdown-item>
              <div class="mx-2 my-1 h-px bg-light/50"></div>
              <el-dropdown-item command="download_url">
                <div class="flex items-center gap-2 text-primary">
                  <div class="i-ep-link"></div>
                  远程下载
                </div>
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>
    </div>
  </div>

  <div v-else class="flex items-center justify-between gap-4 border-b border-light bg-soft/10 px-4 py-3">
    <div class="min-w-0 flex-1">
      <FileBreadcrumbs
        class="!h-auto !border-none !bg-transparent !px-0"
        :current-path="currentPath"
        :collapsed-breadcrumbs="collapsedBreadcrumbs"
        @back="emit('back')"
        @navigate="(path) => emit('navigate', path)"
      />
    </div>

    <div class="flex items-center gap-1.5">
      <transition name="el-fade-in-linear">
        <el-input
          v-if="isSearchExpanded"
          ref="searchInputRef"
          :model-value="searchQuery"
          placeholder="搜索文件"
          size="small"
          class="modern-input !w-40"
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
        v-if="!isSearchExpanded"
        type="button"
        class="h-9 w-9 cursor-pointer rounded-md bg-soft/40 text-primary flex-center transition-colors hover:bg-soft"
        title="搜索文件"
        aria-label="搜索文件"
        @click="expandSearch"
      >
        <div class="i-ep-search text-lg"></div>
      </button>

      <button
        v-if="clipboardFilesCount > 0"
        type="button"
        class="h-9 cursor-pointer rounded-md border border-orange-200/60 bg-orange-50 px-3 text-xs font-semibold text-orange-600 inline-flex items-center gap-1.5 dark:border-orange-500/20 dark:bg-orange-500/10 dark:text-orange-300"
        @click="emit('paste')"
      >
        <div class="i-ep-copy-document text-lg"></div>
      </button>

      <button
        type="button"
        class="h-9 w-9 cursor-pointer rounded-md text-secondary flex-center transition-colors hover:bg-soft"
        title="上传文件"
        aria-label="上传文件"
        @click="emit('trigger-upload')"
      >
        <div class="i-ep-upload-filled text-lg"></div>
      </button>

      <el-dropdown trigger="click" @command="handleCreateCommand">
        <button
          type="button"
          class="h-9 w-9 cursor-pointer rounded-md bg-primary text-white flex-center transition-colors hover:bg-primary/90"
          title="新建"
          aria-label="新建文件或目录"
        >
          <div class="i-ep-plus text-lg"></div>
        </button>
        <template #dropdown>
          <el-dropdown-menu class="modern-dropdown">
            <el-dropdown-item command="file">
              <div class="flex items-center gap-2">
                <div class="i-ep-document"></div>
                新建文件
              </div>
            </el-dropdown-item>
            <el-dropdown-item command="directory">
              <div class="flex items-center gap-2">
                <div class="i-ep-folder"></div>
                新建目录
              </div>
            </el-dropdown-item>
            <div class="mx-2 my-1 h-px bg-light/50"></div>
            <el-dropdown-item command="download_url">
              <div class="flex items-center gap-2 text-primary">
                <div class="i-ep-link"></div>
                远程下载
              </div>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import type { Breadcrumb } from "../../../../composables/useFileOperations";
import FileBreadcrumbs from "./FileBreadcrumbs.vue";

type FileCreateCommand = "file" | "directory" | "download_url";
type FileSortMode = "mtime" | "name" | "size" | "type";
type FileViewMode = "detail" | "grid";

const props = defineProps<{
  clipboardFilesCount: number;
  collapsedBreadcrumbs: Breadcrumb[];
  currentPath: string;
  isMobile: boolean;
  loading: boolean;
  searchQuery: string;
  sortMode: FileSortMode;
  viewMode: FileViewMode;
}>();

const emit = defineEmits<{
  (event: "back"): void;
  (event: "create-command", command: FileCreateCommand): void;
  (event: "navigate", path: string): void;
  (event: "paste"): void;
  (event: "refresh"): void;
  (event: "trigger-upload"): void;
  (event: "update:searchQuery", query: string): void;
  (event: "update:sortMode", mode: FileSortMode): void;
  (event: "update:viewMode", mode: FileViewMode): void;
}>();

const isSearchExpanded = ref(false);
const searchInputRef = ref<{ focus: () => void } | null>(null);

const sortLabel = computed(() => {
  if (props.sortMode === "mtime") return "时间";
  if (props.sortMode === "size") return "大小";
  if (props.sortMode === "type") return "类型";
  return "名称";
});

const isFileCreateCommand = (
  command: unknown,
): command is FileCreateCommand =>
  command === "file" || command === "directory" || command === "download_url";

const isFileSortMode = (command: unknown): command is FileSortMode =>
  command === "name" ||
  command === "mtime" ||
  command === "size" ||
  command === "type";

const expandSearch = () => {
  isSearchExpanded.value = true;
  nextTick(() => {
    searchInputRef.value?.focus();
  });
};

const handleSearchInput = (value: string | number) => {
  emit("update:searchQuery", String(value));
};

const handleCreateCommand = (command: unknown) => {
  if (isFileCreateCommand(command)) emit("create-command", command);
};

const handleSortCommand = (command: unknown) => {
  if (isFileSortMode(command)) emit("update:sortMode", command);
};
</script>
