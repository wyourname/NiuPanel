import { computed, ref } from "vue";
import { ElMessage } from "element-plus";
import { useClipboard } from "@vueuse/core";
import * as gitApi from "@/api/git";
import type { FileEntry, GitRepo } from "@/api/git";

export function useGitFileBrowser() {
  const { copy } = useClipboard();

  const fileDialogVisible = ref(false);
  const filesLoading = ref(false);
  const currentBrowseRepoId = ref<number | null>(null);
  const currentPath = ref("");
  const currentFiles = ref<FileEntry[]>([]);

  const pathParts = computed(() => {
    return currentPath.value ? currentPath.value.split("/").filter(Boolean) : [];
  });

  const navigateFiles = async (path: string) => {
    if (currentBrowseRepoId.value === null) return;

    currentPath.value = path;
    filesLoading.value = true;
    try {
      const res = await gitApi.getRepoFiles(currentBrowseRepoId.value, path);
      currentFiles.value = res.data;
    } catch {
      ElMessage.error("加载文件列表失败");
    } finally {
      filesLoading.value = false;
    }
  };

  const openFileBrowser = (row: GitRepo) => {
    currentBrowseRepoId.value = row.id;
    currentPath.value = "";
    fileDialogVisible.value = true;
    void navigateFiles("");
  };

  const handleFileClick = async (file: FileEntry) => {
    if (file.is_dir) {
      const newPath = currentPath.value
        ? `${currentPath.value}/${file.name}`
        : file.name;
      await navigateFiles(newPath);
      return;
    }

    await copy(file.full_path || file.path);
    ElMessage.success(`已复制路径: ${file.name}`);
  };

  return {
    currentFiles,
    fileDialogVisible,
    filesLoading,
    handleFileClick,
    navigateFiles,
    openFileBrowser,
    pathParts,
  };
}
