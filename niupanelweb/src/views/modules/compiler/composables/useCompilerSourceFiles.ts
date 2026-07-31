import { computed, ref } from "vue";
import { ElLoading, ElMessage } from "element-plus";
import * as fileApi from "@/api/file_manager";
import type { FileItem } from "@/types";

type UseCompilerSourceFilesOptions = {
  setSourceFile: (code: string, fileName: string) => void;
};

const getFileBrowserIcon = (item: Pick<FileItem, "is_dir">) =>
  item.is_dir ? "i-ep-folder" : "i-ep-document";

export function useCompilerSourceFiles(options: UseCompilerSourceFilesOptions) {
  const showFilePicker = ref(false);
  const fileList = ref<FileItem[]>([]);
  const currentPath = ref("");
  const loadingFiles = ref(false);
  const fileInputRef = ref<HTMLInputElement | null>(null);

  const pathParts = computed(() =>
    currentPath.value ? currentPath.value.split("/") : [],
  );

  const navigate = async (path: string) => {
    currentPath.value = path;
    loadingFiles.value = true;
    try {
      const res = await fileApi.listDirectoryContents(path);
      fileList.value = (res.data || []).sort(
        (a, b) => Number(b.is_dir) - Number(a.is_dir),
      );
    } catch {
      ElMessage.error("获取文件列表失败");
    } finally {
      loadingFiles.value = false;
    }
  };

  const openFilePicker = () => {
    showFilePicker.value = true;
    void navigate("");
  };

  const navigateUp = () => {
    const parts = currentPath.value.split("/");
    parts.pop();
    void navigate(parts.join("/"));
  };

  const handleFileItemClick = async (item: FileItem) => {
    if (item.is_dir) {
      await navigate(
        currentPath.value ? `${currentPath.value}/${item.name}` : item.name,
      );
      return;
    }

    if (!item.name.endsWith(".py")) {
      ElMessage.warning("仅支持读取 .py 源代码文件");
      return;
    }

    const loading = ElLoading.service({
      text: "载入中...",
      background: "rgba(0, 0, 0, 0.7)",
    });
    try {
      const res = await fileApi.readFileContent(item.path);
      options.setSourceFile(res.data, item.name);
      showFilePicker.value = false;
      ElMessage.success("已载入代码");
    } catch {
      ElMessage.error("文件读取失败");
    } finally {
      loading.close();
    }
  };

  const triggerFileUpload = () => {
    fileInputRef.value?.click();
  };

  const handleFileSelected = (event: Event) => {
    if (!(event.target instanceof HTMLInputElement)) return;

    const input = event.target;
    const file = input.files?.[0];
    if (!file) return;
    if (!file.name.endsWith(".py")) {
      ElMessage.warning("请选择 Python 脚本 (.py)");
      return;
    }

    const reader = new FileReader();
    reader.onload = (readerEvent) => {
      const result = readerEvent.target?.result;
      if (typeof result !== "string") return;

      options.setSourceFile(result, file.name);
      ElMessage.success(`已载入: ${file.name}`);
      input.value = "";
    };
    reader.readAsText(file);
  };

  return {
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
  };
}
