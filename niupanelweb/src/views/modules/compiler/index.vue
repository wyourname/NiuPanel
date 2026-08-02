<template>
  <WorkspaceAppFrame content-class="relative overflow-hidden">
    <template #toolbar>
      <div class="flex min-h-8 items-center justify-between gap-3">
        <div class="flex min-w-0 items-center gap-2">
          <div class="i-carbon-terminal shrink-0 text-primary"></div>
          <span class="truncate text-xs font-bold text-default">
            {{ currentFileName || "未命名源代码" }}
          </span>
        </div>
        <div class="flex shrink-0 items-center gap-1">
          <button
            type="button"
            class="h-8 cursor-pointer rounded-md px-2 text-xs font-semibold text-secondary flex-center gap-1.5 transition-colors hover:bg-soft hover:text-default sm:px-3"
            title="选择服务器文件"
            @click="openFilePicker"
          >
            <div class="i-ep-folder-opened text-sm"></div>
            <span class="hidden sm:inline">服务器文件</span>
          </button>
          <button
            type="button"
            class="h-8 cursor-pointer rounded-md px-2 text-xs font-semibold text-secondary flex-center gap-1.5 transition-colors hover:bg-soft hover:text-default sm:px-3"
            title="上传本地文件"
            @click="triggerFileUpload"
          >
            <div class="i-ep-upload text-sm"></div>
            <span class="hidden sm:inline">本地文件</span>
          </button>
        </div>
      </div>
    </template>

    <div class="flex h-full min-h-0 overflow-hidden">
      <section class="min-w-0 flex-[3] overflow-hidden border-r border-light/80">
        <VueMonacoEditor
          v-model:value="form.code"
          theme="vs-dark"
          language="python"
          :options="editorOptions"
          class="h-full"
        />
      </section>

      <aside
        v-if="!appStore.isMobile"
        class="w-[330px] shrink-0 overflow-y-auto bg-subtle p-3 custom-scrollbar"
      >
        <ConfigPanel
          v-model:function-name="form.function_name"
          v-model:obfuscate="form.obfuscate"
          :submitting="submitting"
          :loading-versions="loadingVersions"
          :available-versions="availableVersions"
          :target-versions="form.target_versions"
          :result-file="resultFile"
          @compile="handleCompile"
          @toggle-version="toggleVersion"
          @download="handleDownload"
        />
      </aside>
    </div>

    <FloatingActionButton
      v-if="appStore.isMobile"
      icon="i-ep-setting"
      custom-class="!absolute !bottom-4 !right-4 !mb-0"
      @click="showMobileOps = true"
    />
  </WorkspaceAppFrame>

  <input
    ref="fileInputRef"
    type="file"
    class="hidden"
    accept=".py"
    @change="handleFileSelected"
  />

  <ResponsiveDialog
    v-model:visible="showMobileOps"
    title="编译设置"
    desktop-size="sm"
    content-preset="form"
    size="78%"
    append-to-body
  >
    <div class="min-h-0 flex-1 custom-scrollbar">
      <ConfigPanel
        v-model:function-name="form.function_name"
        v-model:obfuscate="form.obfuscate"
        :submitting="submitting"
        :loading-versions="loadingVersions"
        :available-versions="availableVersions"
        :target-versions="form.target_versions"
        :result-file="resultFile"
        @compile="handleCompile"
        @toggle-version="toggleVersion"
        @download="handleDownload"
      />
    </div>
  </ResponsiveDialog>

  <ResponsiveDialog
    v-model:visible="showFilePicker"
    title="服务器代码浏览"
    desktop-size="lg"
    content-preset="list"
    mobile-mode="fullscreen"
    append-to-body
  >
    <div class="flex h-full min-h-0 flex-col md:h-[70vh] md:max-h-[520px]">
      <div class="flex shrink-0 items-center gap-2 overflow-x-auto whitespace-nowrap border-b border-light bg-subtle px-3 py-2 text-xs text-muted">
        <button
          type="button"
          class="cursor-pointer rounded-md px-1.5 py-1 flex-center gap-1 transition-colors hover:bg-card hover:text-primary"
          @click="navigate('')"
        >
          <div class="i-ep-house"></div>
          root
        </button>
        <span
          v-for="(part, index) in pathParts"
          :key="`${part}-${index}`"
          class="flex items-center gap-1"
        >
          <div class="i-ep-arrow-right text-[10px]"></div>
          <button
            type="button"
            class="cursor-pointer rounded-md px-1.5 py-1 transition-colors hover:bg-card hover:text-primary"
            @click="navigate(pathParts.slice(0, index + 1).join('/'))"
          >
            {{ part }}
          </button>
        </span>
      </div>
      <div v-loading="loadingFiles" class="min-h-0 flex-1 overflow-y-auto p-2">
        <button
          v-if="currentPath"
          type="button"
          class="mb-1 flex w-full cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-left text-xs transition-colors hover:bg-subtle"
          @click="navigateUp"
        >
          <div class="i-ep-back"></div>
          返回上级
        </button>
        <button
          v-for="item in fileList"
          :key="item.path"
          type="button"
          class="group flex w-full cursor-pointer items-center gap-3 rounded-md px-2.5 py-2 text-left transition-colors hover:bg-subtle"
          @click="handleFileItemClick(item)"
        >
          <div
            :class="[getFileBrowserIcon(item), item.is_dir ? 'text-amber-500' : 'text-blue-500']"
            class="shrink-0 text-lg"
          ></div>
          <span class="min-w-0 flex-1 truncate font-mono text-sm text-default">
            {{ item.name }}
          </span>
          <div class="i-ep-arrow-right text-xs text-muted opacity-0 transition-opacity group-hover:opacity-100"></div>
        </button>
        <div
          v-if="fileList.length === 0 && !loadingFiles"
          class="h-full flex-center flex-col text-muted"
        >
          <div class="i-ep-folder-opened mb-2 text-3xl opacity-30"></div>
          <span class="text-xs">暂无文件</span>
        </div>
      </div>
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import WorkspaceAppFrame from "../../../components/workspace/WorkspaceAppFrame.vue";
import FloatingActionButton from "../../../components/common/FloatingActionButton.vue";
import { useAppStore } from "../../../stores/app";
import ResponsiveDialog from "../../../components/common/ResponsiveDialog.vue";
import { useCompilerBuild } from "./composables/useCompilerBuild";
import { useCompilerSourceFiles } from "./composables/useCompilerSourceFiles";

import ConfigPanel from "./ConfigPanel.vue";
import { createAsyncMonacoEditor } from "@/utils/monaco";

const VueMonacoEditor = createAsyncMonacoEditor();

const appStore = useAppStore();

const {
  availableVersions,
  currentFileName,
  editorOptions,
  form,
  handleCompile,
  handleDownload,
  loadingVersions,
  resultFile,
  setSourceFile,
  showMobileOps,
  submitting,
  toggleVersion,
} = useCompilerBuild();

const {
  currentPath,
  fileInputRef,
  fileList,
  getFileBrowserIcon,
  handleFileItemClick,
  handleFileSelected,
  loadingFiles,
  navigate,
  navigateUp,
  openFilePicker,
  pathParts,
  showFilePicker,
  triggerFileUpload,
} = useCompilerSourceFiles({ setSourceFile });
</script>
