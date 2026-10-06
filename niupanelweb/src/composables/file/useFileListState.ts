import { computed, onScopeDispose, ref, watch, type Ref } from "vue";
import { debounce } from "lodash-es";
import * as fileManagerApi from "@/api/file_manager";
import type { Breadcrumb, FileItem, FileTableRef } from "./fileOperationTypes";
import { getParentPath, sortFileItems } from "./fileOperationUtils";

export function useFileListState(fileTableRef: Ref<FileTableRef | null>) {
  const loading = ref(false);
  const fileList = ref<FileItem[]>([]);
  const currentPath = ref("/");
  const selectedFiles = ref<FileItem[]>([]);
  const searchQuery = ref("");
  const listError = ref("");
  let requestVersion = 0;
  let requestedPath = "/";
  let changingDirectory = false;

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
    const target = path.replace(/^\/+|\/+$/g, "") || "/";
    const navigating = target !== currentPath.value;
    if (navigating && !fromSearch) {
      executeSearch.cancel();
      changingDirectory = true;
      searchQuery.value = "";
      changingDirectory = false;
    }
    const version = ++requestVersion;
    requestedPath = target;
    loading.value = true;
    listError.value = "";
    const query = searchQuery.value.trim();

    try {
      const res = await fileManagerApi.listDirectoryContents(target, query ? { q: query } : undefined);
      if (version !== requestVersion) return;
      fileList.value = sortFileItems(res.data || []);
      currentPath.value = target;
      if (navigating) clearSelection();
      else selectedFiles.value = selectedFiles.value.filter(file => fileList.value.some(row => row.path === file.path));
    } catch {
      if (version === requestVersion) listError.value = `无法读取 ${target === "/" ? "根目录" : target}，请重试。`;
    } finally {
      if (version === requestVersion) loading.value = false;
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

  const executeSearch = debounce(() => { void loadContents(requestedPath, true); }, 250);

  watch(searchQuery, () => {
    if (changingDirectory) return;
    ++requestVersion;
    loading.value = true;
    executeSearch();
  }, { flush: "sync" });

  onScopeDispose(() => {
    executeSearch.cancel();
    ++requestVersion;
  });

  const navigate = (path: string) => {
    executeSearch.cancel();
    changingDirectory = true;
    searchQuery.value = "";
    changingDirectory = false;
    return loadContents(path);
  };
  const retryLoad = () => loadContents(requestedPath, true);

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
    listError,
    retryLoad,
    navigate,
    searchQuery,
    selectedFiles,
    toggleSelection,
    clearSelection,
  };
}
