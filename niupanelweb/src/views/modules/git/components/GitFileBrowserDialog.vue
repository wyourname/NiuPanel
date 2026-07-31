<template>
  <ResponsiveDialog
    v-model:visible="visible"
    title="仓库文件浏览"
    width="600px"
    append-to-body
  >
    <div class="flex h-[70vh] max-h-[520px] flex-col p-3 sm:p-4">
      <div
        class="mb-3 flex items-center gap-2 overflow-x-auto whitespace-nowrap rounded-md bg-subtle p-2 text-sm"
      >
        <div
          class="i-ep-house cursor-pointer hover:text-primary shrink-0"
          @click="$emit('navigate', '')"
        ></div>
        <div
          v-for="(part, index) in pathParts"
          :key="index"
          class="flex items-center shrink-0"
        >
          <div class="i-ep-arrow-right text-muted mx-1 text-xs"></div>
          <span
            class="cursor-pointer hover:text-primary font-mono"
            @click="$emit('navigate', getPathUntil(index))"
          >
            {{ part }}
          </span>
        </div>
      </div>

      <div
        class="min-h-0 flex-1 overflow-auto rounded-md border border-light bg-card"
        v-loading="loading"
      >
        <div
          v-if="files.length === 0 && !loading"
          class="flex flex-col items-center justify-center h-full text-muted gap-2"
        >
          <div class="i-ep-folder-opened text-4xl opacity-20"></div>
          <span class="text-xs">空目录或未同步</span>
        </div>
        <div
          v-for="file in files"
          :key="file.name"
          class="flex items-center gap-3 p-2 hover:bg-base cursor-pointer border-b border-base last:border-0 transition-colors group"
          @click="$emit('fileClick', file)"
        >
          <div
            :class="[
              getGitBrowserIcon(file),
              file.is_dir ? 'text-yellow-500' : 'text-blue-500',
            ]"
            class="text-xl shrink-0"
          ></div>
          <div class="flex-1 truncate text-sm font-mono text-default">
            {{ file.name }}
          </div>
          <div
            v-if="!file.is_dir"
            class="text-xs text-muted font-mono shrink-0 w-16 text-right"
          >
            {{ formatSize(file.size) }}
          </div>
          <div
            v-if="!file.is_dir"
            class="text-xs text-primary font-bold opacity-0 group-hover:opacity-100 shrink-0"
          >
            复制路径
          </div>
        </div>
      </div>

      <div class="mt-2 text-xs text-muted text-center">
        点击文件可复制完整路径，用于创建任务
      </div>
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import type { FileEntry } from "@/api/git";
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";

const props = defineProps<{
  files: FileEntry[];
  loading: boolean;
  pathParts: string[];
}>();

defineEmits<{
  (e: "fileClick", file: FileEntry): void;
  (e: "navigate", path: string): void;
}>();

const visible = defineModel<boolean>("visible", { required: true });

const getPathUntil = (index: number) => {
  return props.pathParts.slice(0, index + 1).join("/");
};

const getGitBrowserIcon = (file: FileEntry) =>
  file.is_dir ? "i-ep-folder" : "i-ep-document";

const formatSize = (bytes: number) => {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const index = Math.min(
    Math.floor(Math.log(bytes) / Math.log(k)),
    sizes.length - 1,
  );
  return `${parseFloat((bytes / Math.pow(k, index)).toFixed(1))} ${sizes[index]}`;
};
</script>
