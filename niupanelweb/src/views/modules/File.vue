<template>
  <div class="h-full min-h-0">
    <WorkspaceAppFrame
      v-if="!appStore.isMobile"
      content-class="overflow-hidden"
      @dragover.prevent="handleDragOver"
      @dragleave.prevent="handleDragLeave"
      @drop.prevent="handleDrop"
    >
    <FileToolbar
      v-model:search-query="searchQuery"
      v-model:sort-mode="sortMode"
      v-model:view-mode="viewMode"
      :clipboard-files-count="clipboard?.files?.length || 0"
      :collapsed-breadcrumbs="collapsedBreadcrumbs"
      :current-path="currentPath"
      :is-mobile="appStore.isMobile"
      :loading="loading"
      @back="goUp"
      @create-command="handleCreateCommand"
      @navigate="navigate"
      @paste="pasteFromClipboard"
      @refresh="refreshCurrentPath"
      @trigger-upload="triggerFileUpload"
    />

    <FileUploadProgress
      :label="uploadLabel"
      :loaded-bytes="uploadLoadedBytes"
      :percentage="uploadProgress"
      :total-bytes="uploadTotalBytes"
      :visible="uploading"
      @cancel="cancelUpload"
    />

    <FileBulkActions
      :count="selectedFiles.length"
      :is-all-selected="isAllVisibleSelected"
      @cancel="clearSelection"
      @copy="copyToClipboard(selectedFiles)"
      @cut="cutToClipboard(selectedFiles)"
      @delete="batchDelete"
      @download="handleBatchDownload(selectedFiles)"
      @move="showMoveDialog(selectedFiles)"
      @select-all="handleSelectAll"
    />

    <section class="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
      <FileDesktopList
        :items="sortedFileList"
        :loading="loading"
        :search-query="searchQuery"
        :selected-paths="selectedFilePaths"
        :view-mode="viewMode"
        @command="handleDesktopCommand"
        @context-menu="handleDesktopContextMenu"
        @item-drag-end="handleFileDragEnd"
        @item-drag-start="handleFileDragStart"
        @item-click="handleItemClick"
        @toggle-selection="toggleSelection"
      />
    </section>

    <FileDropOverlay :visible="dragOver" />
    </WorkspaceAppFrame>

    <PageShell v-else compact>
      <div
        class="module-panel relative flex min-h-0 flex-1 flex-col overflow-hidden"
        @dragover.prevent="handleDragOver"
        @dragleave.prevent="handleDragLeave"
        @drop.prevent="handleDrop"
      >
        <FileToolbar
          v-model:search-query="searchQuery"
          v-model:sort-mode="sortMode"
          v-model:view-mode="viewMode"
          :clipboard-files-count="clipboard?.files?.length || 0"
          :collapsed-breadcrumbs="collapsedBreadcrumbs"
          :current-path="currentPath"
          :is-mobile="appStore.isMobile"
          :loading="loading"
          @back="goUp"
          @create-command="handleCreateCommand"
          @navigate="navigate"
          @paste="pasteFromClipboard"
          @refresh="refreshCurrentPath"
          @trigger-upload="triggerFileUpload"
        />

        <FileUploadProgress
          :label="uploadLabel"
          :loaded-bytes="uploadLoadedBytes"
          :percentage="uploadProgress"
          :total-bytes="uploadTotalBytes"
          :visible="uploading"
          @cancel="cancelUpload"
        />

        <FileBulkActions
          :count="selectedFiles.length"
          :is-all-selected="selectedFiles.length === (searchQuery ? filteredFileList.length : fileList.length) && fileList.length > 0"
          @cancel="clearSelection"
          @copy="copyToClipboard(selectedFiles)"
          @cut="cutToClipboard(selectedFiles)"
          @delete="batchDelete"
          @download="handleBatchDownload(selectedFiles)"
          @move="showMoveDialog(selectedFiles)"
          @select-all="handleSelectAll"
        />

        <FileMobileList
          :items="filteredFileList"
          :loading="loading"
          :search-query="searchQuery"
          :selected-paths="selectedFilePaths"
          @command="handleMobileCommand"
          @item-click="handleItemClickMobile"
          @touch-end="handleTouchEnd"
          @touch-move="handleTouchMove"
          @touch-start="handleTouchStart"
        />

        <FileDropOverlay :visible="dragOver" />
      </div>
    </PageShell>

    <FileCrudDialogs
      :create-visible="createDialogVisible"
      :create-type="createType"
      :creating="creating"
      :create-form="createForm"
      :move-visible="moveDialogVisible"
      :moving-file="movingFile"
      :move-form="moveForm"
      :rename-visible="renameDialogVisible"
      :renaming="renaming"
      :rename-form="renameForm"
      :download-url-visible="downloadUrlDialogVisible"
      :downloading-url="downloadingUrl"
      :download-url-form="downloadUrlForm"
      :on-create-submit="handleCreateItem"
      :on-move-submit="executeMove"
      :on-rename-submit="handleRenameItem"
      :on-download-url-submit="handleDownloadFromUrl"
      @update:create-visible="createDialogVisible = $event"
      @update:move-visible="moveDialogVisible = $event"
      @update:rename-visible="renameDialogVisible = $event"
      @update:download-url-visible="downloadUrlDialogVisible = $event"
    />

    <FileEditorDialog
      v-model:visible="editFileDialogVisible"
      v-model:content="fileContent"
      :current-file="currentFile"
      :is-dark="appStore.isDark"
      :is-mobile="appStore.isMobile"
      :saving="savingFile"
      @save="saveFileContent"
    />

    <FileImagePreviewDialog v-model="imagePreviewVisible" :src="imageUrl" />

    <ContextMenu
      v-model:visible="contextMenuVisible"
      :position="contextMenuPosition"
      :items="contextMenuItems"
      @select="handleContextMenuSelect"
    />

    <input
      ref="fileInputRef"
      type="file"
      class="hidden"
      multiple
      @change="handleFileUpload"
    />
  </div>
</template>

<script setup lang="ts">
import {
  computed,
  onMounted,
  ref,
  watch,
} from "vue";
import { useAppStore } from "../../stores/app";
import { useFileOperations } from "../../composables/useFileOperations";
import ContextMenu from "../../components/common/ContextMenu.vue";
import PageShell from "../../components/common/PageShell.vue";
import WorkspaceAppFrame from "../../components/workspace/WorkspaceAppFrame.vue";
import { useFileCommandHandlers } from "./file/composables/useFileCommandHandlers";
import { useFileContextMenu } from "./file/composables/useFileContextMenu";
import { useFileSaveShortcut } from "./file/composables/useFileSaveShortcut";
import { useFileTouchSelection } from "./file/composables/useFileTouchSelection";
import {
  INTERNAL_FILE_DRAG_MIME,
  useFileUploadDrop,
} from "./file/composables/useFileUploadDrop";

import FileCrudDialogs from "./file/components/FileCrudDialogs.vue";
import FileBulkActions from "./file/components/FileBulkActions.vue";
import FileDesktopList from "./file/components/FileDesktopList.vue";
import FileDropOverlay from "./file/components/FileDropOverlay.vue";
import FileEditorDialog from "./file/components/FileEditorDialog.vue";
import FileImagePreviewDialog from "./file/components/FileImagePreviewDialog.vue";
import FileMobileList from "./file/components/FileMobileList.vue";
import FileToolbar from "./file/components/FileToolbar.vue";
import FileUploadProgress from "./file/components/FileUploadProgress.vue";

import type {
  FileItem,
  FileTableRef,
} from "../../composables/useFileOperations";
import { getFileExtension } from "./file/utils/fileDisplay";

type FileSortMode = "mtime" | "name" | "size" | "type";
type FileViewMode = "detail" | "grid";

const VIEW_MODE_KEY = "niupanel.file.viewMode";
const SORT_MODE_KEY = "niupanel.file.sortMode";

const readStoredValue = <T extends string>(
  key: string,
  fallback: T,
  values: T[],
) => {
  if (typeof window === "undefined") return fallback;
  const stored = window.localStorage.getItem(key);
  return values.includes(stored as T) ? (stored as T) : fallback;
};

const appStore = useAppStore();
const fileTableRef = ref<FileTableRef | null>(null);
const viewMode = ref<FileViewMode>(
  readStoredValue<FileViewMode>(VIEW_MODE_KEY, "detail", ["detail", "grid"]),
);
const sortMode = ref<FileSortMode>(
  readStoredValue<FileSortMode>(SORT_MODE_KEY, "name", [
    "name",
    "mtime",
    "size",
    "type",
  ]),
);

const {
  cancelUpload,
  loading,
  fileList,
  currentPath,
  selectedFiles,
  searchQuery,
  filteredFileList,
  clipboard,
  createDialogVisible,
  createType,
  creating,
  createForm,
  renameDialogVisible,
  renaming,
  renameForm,
  editFileDialogVisible,
  currentFile,
  fileContent,
  savingFile,
  imagePreviewVisible,
  imageUrl,
  collapsedBreadcrumbs,
  downloadUrlDialogVisible,
  downloadingUrl,
  downloadUrlForm,
  loadContents,
  navigate,
  goUp,
  toggleSelection,
  clearSelection,
  handleSelectAll,
  copyToClipboard,
  cutToClipboard,
  pasteFromClipboard,
  deleteItem,
  batchDelete,
  handleCreateItem,
  handleRenameItem,
  showEditFileDialog,
  saveFileContent,
  performUpload,
  handleDownload,
  handleBatchDownload,
  extractArchive,
  previewImage,
  showRenameDialog,
  moveDialogVisible,
  movingFile,
  moveForm,
  showMoveDialog,
  executeMove,
  copyDroppedFiles,
  handleDownloadFromUrl,
  uploadLabel,
  uploadLoadedBytes,
  uploadProgress,
  uploadTotalBytes,
  uploading,
} = useFileOperations(fileTableRef);

const selectedFilePaths = computed(() =>
  selectedFiles.value.map((file) => file.path),
);

const sortItems = (items: FileItem[]) => {
  return [...items].sort((a, b) => {
    const directorySort = Number(b.is_dir) - Number(a.is_dir);
    if (directorySort !== 0) return directorySort;

    if (sortMode.value === "mtime") {
      const result = (b.mtime ?? 0) - (a.mtime ?? 0);
      return result || a.name.localeCompare(b.name);
    }

    if (sortMode.value === "size") {
      const result = Number(b.size || 0) - Number(a.size || 0);
      return result || a.name.localeCompare(b.name);
    }

    if (sortMode.value === "type") {
      const result = getFileExtension(a.name).localeCompare(
        getFileExtension(b.name),
      );
      return result || a.name.localeCompare(b.name);
    }

    return a.name.localeCompare(b.name);
  });
};

const sortedFileList = computed(() => sortItems(filteredFileList.value));

const isAllVisibleSelected = computed(
  () =>
    sortedFileList.value.length > 0 &&
    sortedFileList.value.every((item) =>
      selectedFilePaths.value.includes(item.path),
    ),
);

const selectSingleFile = (row: FileItem) => {
  selectedFiles.value = [row];
};

const handleDesktopContextMenu = (row: FileItem, event: MouseEvent) => {
  if (!selectedFilePaths.value.includes(row.path)) {
    selectSingleFile(row);
  }
  handleRowContextMenu(row, event);
};

const refreshCurrentPath = () => {
  void loadContents(currentPath.value || "/", Boolean(searchQuery.value));
};

const {
  fileActionHandlers,
  handleCreateCommand,
  handleDesktopCommand,
  handleItemClick,
  handleMobileCommand,
} = useFileCommandHandlers({
  batchDelete,
  copyToClipboard,
  createDialogVisible,
  createForm,
  createType,
  cutToClipboard,
  deleteItem,
  downloadUrlDialogVisible,
  downloadUrlForm,
  extractArchive,
  handleDownload,
  navigate,
  previewImage,
  showEditFileDialog,
  showMoveDialog,
  showRenameDialog,
});

const {
  contextMenuItems,
  contextMenuPosition,
  contextMenuVisible,
  handleContextMenuSelect,
  handleRowContextMenu,
} = useFileContextMenu({
  actionHandlers: fileActionHandlers,
  fileTableRef,
  selectedFiles,
});

const {
  handleItemClickMobile,
  handleTouchEnd,
  handleTouchMove,
  handleTouchStart,
} = useFileTouchSelection({
  handleItemClick,
  isMobile: () => appStore.isMobile,
  selectedFiles,
  toggleSelection,
});

const {
  dragOver,
  fileInputRef,
  handleDragLeave,
  handleDragOver,
  handleDrop,
  handleFileUpload,
  triggerFileUpload,
} = useFileUploadDrop(performUpload, copyDroppedFiles);

const handleFileDragStart = (row: FileItem, event: DragEvent) => {
  if (!event.dataTransfer) return;

  const files = selectedFilePaths.value.includes(row.path)
    ? selectedFiles.value
    : [row];
  const payload = {
    files: files.map((file) => ({ ...file })),
  };

  event.dataTransfer.effectAllowed = "copy";
  event.dataTransfer.setData(INTERNAL_FILE_DRAG_MIME, JSON.stringify(payload));
  event.dataTransfer.setData(
    "text/plain",
    files.map((file) => file.name).join("\n"),
  );
};

const handleFileDragEnd = () => {
  dragOver.value = false;
};

watch(viewMode, (mode) => {
  window.localStorage.setItem(VIEW_MODE_KEY, mode);
});

watch(sortMode, (mode) => {
  window.localStorage.setItem(SORT_MODE_KEY, mode);
});

useFileSaveShortcut({
  isEditing: editFileDialogVisible,
  saveFileContent,
});

onMounted(() => {
  void loadContents("/");
});
</script>
