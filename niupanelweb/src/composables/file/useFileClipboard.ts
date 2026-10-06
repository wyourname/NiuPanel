import { computed, ref, type Ref } from "vue";
import { ElMessage } from "element-plus";
import * as fileManagerApi from "@/api/file_manager";
import { useFileClipboardStore } from "@/stores/fileClipboard";
import type { FileItem } from "./fileOperationTypes";
import { joinDirectoryPath } from "./fileOperationUtils";

type UseFileClipboardOptions = {
  clearSelection: () => void;
  currentPath: Ref<string>;
  loadContents: (path: string) => Promise<void>;
};

export function useFileClipboard({
  clearSelection,
  currentPath,
  loadContents,
}: UseFileClipboardOptions) {
  const clipboardStore = useFileClipboardStore();
  const clipboard = computed(() => clipboardStore.clipboard);
  const pasting = ref(false);

  const copyToClipboard = (files: FileItem[]) => {
    clipboardStore.setClipboard("copy", files);
    ElMessage.success(`已复制 ${files.length} 项`);
    clearSelection();
  };

  const cutToClipboard = (files: FileItem[]) => {
    clipboardStore.setClipboard("cut", files);
    ElMessage.success(`已剪切 ${files.length} 项`);
    clearSelection();
  };

  const pasteFromClipboard = async () => {
    if (pasting.value || !clipboard.value.action || clipboard.value.files.length === 0) return;

    pasting.value = true;
    const action = clipboard.value.action;
    const originalClipboard = clipboard.value;
    const destination = currentPath.value;
    const files = clipboard.value.files.map((file) => ({ ...file }));
    const failedFiles: FileItem[] = [];
    let successCount = 0;
    let failCount = 0;
    const chunkSize = 5;

    try {
      for (let i = 0; i < files.length; i += chunkSize) {
        const chunk = files.slice(i, i + chunkSize);
        const results = await Promise.all(chunk.map(async (file) => {
          const targetPath = joinDirectoryPath(destination, file.name);
          if (targetPath === file.path) return false;

          try {
            if (action === "copy") {
              await fileManagerApi.copyItem(file.path, targetPath);
            } else if (action === "cut") {
              await fileManagerApi.renameItem(file.path, targetPath);
            }
            return true;
          } catch {
            return false;
          }
        }));

        results.forEach((success, index) => {
          if (success) successCount++;
          else { failCount++; failedFiles.push(chunk[index]!); }
        });
      }

      if (successCount > 0) {
        ElMessage.success(`成功${action === "copy" ? "复制" : "移动"} ${successCount} 项`);
        if (currentPath.value === destination) await loadContents(destination);
      }
      if (clipboard.value === originalClipboard) {
        if (failedFiles.length) clipboardStore.setClipboard(action, failedFiles);
        else clipboardStore.clearClipboard();
      }
      if (failCount > 0) ElMessage.warning(`${failCount} 项操作失败`);
    } finally {
      pasting.value = false;
    }
  };

  return {
    clipboard,
    clearClipboard: clipboardStore.clearClipboard,
    copyToClipboard,
    cutToClipboard,
    pasteFromClipboard,
    pasting,
  };
}
