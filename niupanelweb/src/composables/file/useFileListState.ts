import { computed, onScopeDispose, ref, watch, type Ref } from "vue";
import { debounce } from "lodash-es";
import * as fileManagerApi from "@/api/file_manager";
import type { Breadcrumb, FileItem, FileTableRef } from "./fileOperationTypes";
import { getParentPath, sortFileItems } from "./fileOperationUtils";

export function useFileListState(fileTableRef: Ref<FileTableRef | null>) {
  const loading = ref(false);
  const fileList = ref<FileItem[]>([]);
  const currentPath = ref("");
  const selectedFiles = ref<FileItem[]>([]);
  const searchQuery = ref("");
  const isSearching = ref(false);

  const filteredFileList = computed(() => fileList.value);

  const breadcrumbs = computed(() => currentPath.value.split("/").filter(Boolean));

  const collapsedBreadcrumbs = computed(() => {
    const parts = breadcrumbs.value.map((name, index) => ({
      name,
      path: breadcrumbs.value.slice(0, index + 1).join("/"),
    }));

    if (parts.length <= 4) return parts as Breadcrumb[];

    return [
      parts[0],
      { type: "ellipsis", items: parts.slice(1, -2) } as Breadcrumb,
      ...parts.slice(-2),
    ];
  });

  const clearSelection = () => {
    fileTableRef.value?.clearSelection?.();
    selectedFiles.value = [];
  };

  const loadContents = async (path: string, fromSearch = false) => {
    loading.value = true;
    if (!fromSearch) {
      searchQuery.value = "";
    }

    try {
      const actualPath = isSearching.value ? currentPath.value : path;
      const queryParam = isSearching.value && searchQuery.value
        ? `?q=${encodeURIComponent(searchQuery.value)}`
        : "";

      const res = await fileManagerApi.listDirectoryContents(actualPath + queryParam);
      fileList.value = sortFileItems(res.data || []);
      if (!isSearching.value) {
        currentPath.value = path;
      }
      clearSelection();
    } finally {
      loading.value = false;
    }
  };

  const loadNode = async (
    row: FileItem,
    _treeNode: unknown,
    resolve: (data: FileItem[]) => void,
  ) => {
    try {
      if (!row.is_dir) {
        resolve([]);
        return;
      }
      const res = await fileManagerApi.listDirectoryContents(row.path);
      resolve(sortFileItems(res.data || []));
    } catch (error) {
      console.error("Failed to load tree node:", error);
      resolve([]);
    }
  };

  const executeSearch = debounce((query: string) => {
    if (query) {
      isSearching.value = true;
      void loadContents(currentPath.value, true);
    } else {
      isSearching.value = false;
      void loadContents(currentPath.value, true);
    }
  }, 500);

  watch(searchQuery, (newVal) => {
    executeSearch(newVal);
  });

  onScopeDispose(() => {
    executeSearch.cancel();
  });

  const navigate = (path: string) => loadContents(path);

  const goUp = () => {
    if (!currentPath.value || currentPath.value === "/") return;
    navigate(getParentPath(currentPath.value));
  };

  const handleSelectionChange = (val: FileItem[]) => {
    selectedFiles.value = val;
  };

  const toggleSelection = (row: FileItem) => {
    const index = selectedFiles.value.findIndex((file) => file.path === row.path);
    if (index > -1) {
      selectedFiles.value.splice(index, 1);
      fileTableRef.value?.toggleRowSelection?.(row, false);
    } else {
      selectedFiles.value.push(row);
      fileTableRef.value?.toggleRowSelection?.(row, true);
    }
  };

  const isSelected = (row: FileItem) => {
    return selectedFiles.value.findIndex((file) => file.path === row.path) > -1;
  };

  const handleSelectAll = () => {
    const targetList = fileList.value;
    if (selectedFiles.value.length === targetList.length && targetList.length > 0) {
      clearSelection();
    } else {
      selectedFiles.value = [...targetList];
      targetList.forEach((row) => {
        fileTableRef.value?.toggleRowSelection?.(row, true);
      });
    }
  };

  return {
    collapsedBreadcrumbs,
    currentPath,
    fileList,
    filteredFileList,
    goUp,
    handleSelectAll,
    handleSelectionChange,
    isSelected,
    loadContents,
    loadNode,
    loading,
    navigate,
    searchQuery,
    selectedFiles,
    toggleSelection,
    clearSelection,
  };
}
