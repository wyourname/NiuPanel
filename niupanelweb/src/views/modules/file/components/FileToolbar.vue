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
              <el-dropdown-item
                v-for="option in FILE_SORT_OPTIONS"
                :key="option.value"
                :command="option.value"
              >
                <div class="flex items-center gap-2">
                  <div :class="sortMode === option.value ? 'i-ep-check' : option.icon"></div>
                  {{ option.label }}
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

  <div v-else class="shrink-0 space-y-2 border-b border-light bg-soft/10 px-3 py-2.5">
    <div class="flex min-w-0 items-center gap-2">
      <button
        type="button"
        class="h-11 w-11 shrink-0 cursor-pointer rounded-md text-secondary flex-center transition-colors hover:bg-soft hover:text-default"
        title="刷新"
        aria-label="刷新当前目录"
        @click="emit('refresh')"
      >
        <div class="i-ep-refresh text-[17px]" :class="{ 'animate-spin': loading }"></div>
      </button>
      <FileBreadcrumbs
        class="min-w-0 flex-1 !h-auto !border-none !bg-transparent !px-0"
        :current-path="currentPath"
        :collapsed-breadcrumbs="collapsedBreadcrumbs"
        @back="emit('back')"
        @navigate="(path) => emit('navigate', path)"
      />
    </div>

    <div class="flex min-w-0 items-center gap-1.5">
      <el-input
        :model-value="searchQuery"
        placeholder="搜索文件"
        class="modern-input min-w-[88px] flex-1"
        clearable
        @update:model-value="handleSearchInput"
      >
        <template #prefix>
          <div class="i-ep-search text-sm text-muted"></div>
        </template>
      </el-input>

      <el-dropdown trigger="click" @command="handleSortCommand">
        <button
          type="button"
          class="h-11 w-11 shrink-0 cursor-pointer rounded-md text-secondary flex-center transition-colors hover:bg-soft hover:text-default"
          :title="`排序：${sortLabel}`"
          :aria-label="`文件排序：${sortLabel}`"
        >
          <div class="i-ep-sort text-lg"></div>
        </button>
        <template #dropdown>
          <el-dropdown-menu class="modern-dropdown">
            <el-dropdown-item
              v-for="option in FILE_SORT_OPTIONS"
              :key="option.value"
              :command="option.value"
            >
              <div class="flex items-center gap-2">
                <div :class="sortMode === option.value ? 'i-ep-check' : option.icon"></div>
                {{ option.label }}
              </div>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>

      <button
        v-if="clipboardFilesCount > 0"
        type="button"
        class="h-11 w-11 shrink-0 cursor-pointer rounded-md border border-orange-200/60 bg-orange-50 text-orange-600 flex-center dark:border-orange-500/20 dark:bg-orange-500/10 dark:text-orange-300"
        :title="`粘贴 ${clipboardFilesCount} 个项目`"
        aria-label="粘贴剪贴板文件"
        @click="emit('paste')"
      >
        <div class="i-ep-copy-document text-lg"></div>
      </button>

      <button
        type="button"
        class="h-11 w-11 shrink-0 cursor-pointer rounded-md text-secondary flex-center transition-colors hover:bg-soft"
        title="上传文件"
        aria-label="上传文件"
        @click="emit('trigger-upload')"
      >
        <div class="i-ep-upload-filled text-lg"></div>
      </button>

      <el-dropdown trigger="click" @command="handleCreateCommand">
        <button
          type="button"
          class="h-11 w-11 shrink-0 cursor-pointer rounded-md bg-primary text-white flex-center transition-colors hover:bg-primary/90"
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
import { computed } from "vue";
import type { Breadcrumb } from "../../../../composables/useFileOperations";
import FileBreadcrumbs from "./FileBreadcrumbs.vue";
import {
  FILE_SORT_OPTIONS,
  getFileSortLabel,
  isFileSortMode,
  type FileSortMode,
} from "../utils/fileSort";

type FileCreateCommand = "file" | "directory" | "download_url";
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

const sortLabel = computed(() => getFileSortLabel(props.sortMode));

const isFileCreateCommand = (
  command: unknown,
): command is FileCreateCommand =>
  command === "file" || command === "directory" || command === "download_url";

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
