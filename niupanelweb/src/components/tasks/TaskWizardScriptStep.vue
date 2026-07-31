<template>
  <div class="flex flex-col">
    <div class="mb-4 flex gap-1 rounded-md border border-light bg-soft p-1">
      <button
        v-for="mode in scriptSourceModes"
        :key="mode"
        type="button"
        class="flex flex-1 cursor-pointer items-center justify-center gap-2 rounded py-2 text-xs font-bold transition-colors duration-200"
        :class="
          scriptSourceMode === mode
            ? 'bg-card text-primary shadow-sm'
            : 'text-muted hover:text-default hover:bg-hover'
        "
        @click="emit('update:scriptSourceMode', mode)"
      >
        <div
          class="text-base"
          :class="[
            { 'i-ep-upload-filled': mode === 'upload' },
            { 'i-ep-folder': mode === 'file' },
            { 'i-ep-edit': mode === 'command' },
          ]"
        />
        <span>{{ scriptSourceModeLabels[mode] }}</span>
      </button>
    </div>

    <div class="space-y-4">
      <div v-if="scriptSourceMode === 'command'">
        <span class="label-xs mb-2 block">执行命令</span>
        <el-input
          :model-value="command"
          type="textarea"
          :rows="10"
          placeholder="请输入执行命令，例如：python main.py"
          class="modern-input font-mono !text-xs"
          @update:model-value="emit('update:command', String($event))"
        />
      </div>

      <div
        v-else-if="scriptSourceMode === 'file'"
        class="flex flex-col overflow-hidden rounded-md border border-light bg-card"
      >
        <div class="px-3 py-1 bg-soft border-b border-light flex items-center gap-3">
          <div class="i-ep-search text-muted shrink-0 text-sm"></div>

          <div
            v-if="path && !searchQuery"
            class="flex-1 flex items-center min-w-0 h-9"
            @click="emit('update:searchQuery', '')"
          >
            <span
              class="text-xs font-mono text-primary truncate mr-2 cursor-pointer hover:opacity-80 transition-opacity"
            >
              {{ path }}
            </span>
            <div
              class="i-ep-close text-muted hover:text-rose-500 cursor-pointer p-0.5 shrink-0"
              @click.stop="emit('update:path', '')"
            ></div>
          </div>

          <el-input
            v-else
            :model-value="searchQuery"
            placeholder="搜索您的脚本文件..."
            clearable
            class="flex-1 text-sm no-prefix-icon modern-input [&_.el-input__wrapper]:!bg-transparent [&_.el-input__wrapper]:!shadow-none"
            @update:model-value="emit('update:searchQuery', String($event))"
          />
        </div>

        <div
          class="px-4 py-2 border-b border-light flex items-center gap-1 overflow-x-auto text-[11px] bg-soft/50"
        >
          <div
            class="i-ep-house text-muted cursor-pointer hover:text-primary shrink-0 transition-colors"
            @click="emit('navigate', '')"
          ></div>
          <template v-for="(part, index) in pathParts" :key="index">
            <span class="text-muted/30 mx-1">/</span>
            <span
              v-if="index < pathParts.length - 1"
              class="cursor-pointer text-primary hover:underline whitespace-nowrap transition-all"
              @click="emit('navigate', pathParts.slice(0, index + 1).join('/'))"
            >
              {{ part }}
            </span>
            <span v-else class="font-bold text-default whitespace-nowrap truncate">
              {{ part }}
            </span>
          </template>
        </div>

        <div class="overflow-y-auto p-2 custom-scrollbar" v-loading="browserLoading">
          <div
            v-if="currentPath"
            class="mb-1 flex cursor-pointer items-center gap-3 rounded-md px-3 py-2.5 text-muted transition-colors hover:bg-hover hover:text-default"
            @click="emit('navigate-up')"
          >
            <div class="i-ep-folder text-xl text-yellow-500/60"></div>
            <span class="text-sm font-semibold">返回上级</span>
          </div>

          <div
            v-if="browserItems.length === 0 && !browserLoading"
            class="h-48 flex flex-col items-center justify-center text-muted"
          >
            <div class="i-ep-folder-opened text-5xl mb-3 opacity-20"></div>
            <span class="text-xs font-medium opacity-50">
              {{ searchQuery ? "未找到搜索结果" : "空目录" }}
            </span>
          </div>

          <div
            v-for="item in browserItems"
            :key="item.path"
            class="mb-1 flex cursor-pointer items-center gap-3 rounded-md border border-transparent px-3 py-2.5 transition-colors hover:bg-hover"
            :class="{
              'border-primary/20 shadow-sm':
                path === item.path ||
                (item.is_dir && path?.startsWith(item.path + '/')),
            }"
            @click="emit('browser-item-click', item)"
          >
            <div
              class="shrink-0 text-xl"
              :class="
                item.is_dir
                  ? 'i-ep-folder text-yellow-500'
                  : 'i-ep-document text-blue-500/70 group-hover:text-blue-500'
              "
            ></div>
            <div class="flex-1 min-w-0">
              <div
                class="truncate text-sm text-default font-medium"
                :class="{
                  'text-primary font-bold': path === item.path,
                }"
              >
                <span
                  v-if="searchQuery && item.path.includes('/')"
                  class="opacity-40 font-normal"
                >
                  {{ item.path.substring(0, item.path.lastIndexOf('/')) }}/
                </span>
                <span>{{ item.name }}</span>
              </div>
            </div>
            <div
              v-if="path === item.path"
              class="i-ep-circle-check-filled text-primary text-xl animate-in fade-in zoom-in duration-300"
            ></div>
          </div>
        </div>
      </div>

      <div v-else class="space-y-4">
        <span class="label-xs mb-2 block">本地文件上传</span>
        <el-upload
          drag
          action=""
          :auto-upload="false"
          :on-change="handleUploadChange"
          :show-file-list="false"
          class="w-full [&_.el-upload-dragger]:!rounded-md [&_.el-upload-dragger]:!border [&_.el-upload-dragger]:!border-dashed [&_.el-upload-dragger]:!border-light [&_.el-upload-dragger]:!bg-card [&_.el-upload-dragger]:transition-colors"
        >
          <div class="flex flex-col items-center py-8 md:py-10">
            <div class="accent-subtle mb-3 h-12 w-12 rounded-md flex-center">
              <div class="i-ep-upload-filled text-2xl"></div>
            </div>
            <div class="text-sm font-bold text-default mb-1">点击或拖拽文件上传</div>
            <div class="text-[11px] text-muted">支持 .js、.py、.sh 等脚本文件</div>
          </div>
        </el-upload>

        <div
          v-if="uploadedFile"
          class="mt-3 flex items-center justify-between rounded-md border border-primary/20 bg-primary/5 p-3"
        >
          <div class="flex items-center gap-3 overflow-hidden">
            <div
              class="accent-subtle h-9 w-9 rounded-md flex-center"
            >
              <div class="text-xl i-ep-document"></div>
            </div>
            <div class="flex flex-col overflow-hidden">
              <span class="truncate text-sm font-bold text-default">
                {{ uploadedFile.name }}
              </span>
              <span class="text-[10px] font-medium text-muted">已准备就绪</span>
            </div>
          </div>
          <button
            type="button"
            class="rounded-md p-2 text-muted transition-colors hover:bg-rose-500/10 hover:text-rose-500"
            title="移除文件"
            aria-label="移除已上传文件"
            @click.stop="emit('clear-uploaded-file')"
          >
            <div class="i-ep-close"></div>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { UploadFile } from "element-plus";
import type {
  TaskWizardBrowserItem,
} from "../../composables/useTaskWizardFileBrowser";
import type { TaskWizardScriptSourceMode } from "../../composables/taskWizardHelpers";

const scriptSourceModes: TaskWizardScriptSourceMode[] = [
  "upload",
  "file",
  "command",
];

const scriptSourceModeLabels: Record<TaskWizardScriptSourceMode, string> = {
  upload: "上传脚本",
  file: "文件浏览器",
  command: "手写命令",
};

defineProps<{
  browserItems: TaskWizardBrowserItem[];
  browserLoading: boolean;
  command: string;
  currentPath: string;
  path: string;
  pathParts: string[];
  scriptSourceMode: TaskWizardScriptSourceMode;
  searchQuery: string;
  uploadedFile: File | null;
}>();

const emit = defineEmits<{
  (event: "browser-item-click", item: TaskWizardBrowserItem): void;
  (event: "clear-uploaded-file"): void;
  (event: "navigate", path: string): void;
  (event: "navigate-up"): void;
  (event: "script-upload", file: UploadFile): void;
  (event: "update:command", value: string): void;
  (event: "update:path", value: string): void;
  (event: "update:scriptSourceMode", value: TaskWizardScriptSourceMode): void;
  (event: "update:searchQuery", value: string): void;
}>();

const handleUploadChange = (file: UploadFile) => {
  emit("script-upload", file);
};
</script>

<style scoped>
/* Scoped styles removed in favor of UnoCSS utility classes */
</style>
