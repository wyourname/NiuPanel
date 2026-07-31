<template>
  <WorkspaceAppFrame content-class="overflow-hidden">
    <template #toolbar>
      <div class="flex min-h-8 items-center justify-between gap-3">
        <div class="flex min-w-0 items-center gap-2 text-xs font-semibold text-secondary">
          <span class="h-2 w-2 rounded-full bg-emerald-500"></span>
          <span class="truncate">{{ repos.length }} 个仓库</span>
          <span v-if="syncingId" class="text-primary">同步中</span>
        </div>
        <button
          type="button"
          class="h-8 cursor-pointer rounded-md bg-primary px-2.5 text-xs font-semibold text-white flex-center gap-1.5 transition-colors hover:bg-primary/90 sm:px-3"
          @click="openCreate"
        >
          <div class="i-ep-plus text-sm"></div>
          <span class="hidden sm:inline">添加仓库</span>
          <span class="sm:hidden">新增</span>
        </button>
      </div>
    </template>

    <GitRepoTable
      :get-status-color="getStatusColor"
      :is-mobile="appStore.isMobile"
      :loading="loading"
      :repos="repos"
      :syncing-id="syncingId"
      @browse="openFileBrowser"
      @delete="handleDelete"
      @edit="handleEdit"
      @scan="openScan"
      @sync="handleSync"
    />
  </WorkspaceAppFrame>

  <GitRepoDialog
    v-model:form="form"
    v-model:visible="dialogVisible"
    :is-edit="isEdit"
    :rules="rules"
    :submitting="submitting"
    @submit="handleSubmit"
  />

  <GitFileBrowserDialog
    v-model:visible="fileDialogVisible"
    :files="currentFiles"
    :loading="filesLoading"
    :path-parts="pathParts"
    @file-click="handleFileClick"
    @navigate="navigateFiles"
  />

  <GitTaskImportDialog
    v-model:select-all="selectAll"
    v-model:visible="scanDialogVisible"
    :importing="importing"
    :scanning="scanning"
    :selected-count="selectedDiscoveredTasks.length"
    :tasks="discoveredTasks"
    @import="handleImport"
    @select-all-change="handleSelectAllChange"
    @selection-change="handleTaskSelectionChange"
  />
</template>

<script setup lang="ts">
import { onMounted } from "vue";
import WorkspaceAppFrame from "../../../components/workspace/WorkspaceAppFrame.vue";
import { useAppStore } from "../../../stores/app";
import GitFileBrowserDialog from "./components/GitFileBrowserDialog.vue";
import GitRepoDialog from "./components/GitRepoDialog.vue";
import GitRepoTable from "./components/GitRepoTable.vue";
import GitTaskImportDialog from "./components/GitTaskImportDialog.vue";
import { useGitFileBrowser } from "./composables/useGitFileBrowser";
import { useGitRepoForm } from "./composables/useGitRepoForm";
import { useGitRepos } from "./composables/useGitRepos";
import { useGitTaskImport } from "./composables/useGitTaskImport";

const appStore = useAppStore();

const {
  getStatusColor,
  handleDelete,
  handleSync,
  loadData,
  loading,
  repos,
  syncingId,
} = useGitRepos();

const {
  dialogVisible,
  form,
  handleEdit,
  handleSubmit,
  isEdit,
  openCreate,
  rules,
  submitting,
} = useGitRepoForm({ onSaved: loadData });

const {
  currentFiles,
  fileDialogVisible,
  filesLoading,
  handleFileClick,
  navigateFiles,
  openFileBrowser,
  pathParts,
} = useGitFileBrowser();

const {
  discoveredTasks,
  handleImport,
  handleSelectAllChange,
  handleTaskSelectionChange,
  importing,
  openScan,
  scanDialogVisible,
  scanning,
  selectAll,
  selectedDiscoveredTasks,
} = useGitTaskImport();

onMounted(() => {
  void loadData();
});
</script>
