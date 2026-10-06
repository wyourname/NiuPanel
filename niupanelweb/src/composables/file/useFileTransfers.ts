import { onScopeDispose, ref, type Ref } from "vue";
import { ElMessage } from "element-plus";
import * as fileManagerApi from "@/api/file_manager";
import { useFileTransfersStore } from "@/stores/fileTransfers";
import type { FileItem } from "./fileOperationTypes";

type UseFileTransfersOptions = {
  currentPath: Ref<string>;
  loading: Ref<boolean>;
  loadContents: (path: string) => Promise<void>;
};

export function useFileTransfers({
  currentPath,
  loading,
  loadContents,
}: UseFileTransfersOptions) {
  const imagePreviewVisible = ref(false);
  const imageUrl = ref("");
  const transfers = useFileTransfersStore();
  const refreshUploadedDirectory = (event: Event) => {
    const destination = (event as CustomEvent<string>).detail;
    if (currentPath.value === destination) void loadContents(destination);
  };
  window.addEventListener('niu:files-changed', refreshUploadedDirectory);
  const performUpload = async (files: FileList | null) => {
    if (!files?.length) return;
    const destination = currentPath.value;
    await transfers.upload(Array.from(files), destination);
  };
  const handleDownload = (row: FileItem) => transfers.download(row.name, [row.path], false);
  const handleBatchDownload = (files: FileItem[]) => {
    if (!files.length) return;
    return transfers.download(`files_${Date.now()}.tar`, files.map(file => file.path), true);
  };

  const extractArchive = async (row: FileItem) => {
    loading.value = true;
    try {
      const result = await fileManagerApi.extractArchive(row.path);
      ElMessage.success(`解压完成，已释放 ${result.data ?? 0} 项`);
      await loadContents(currentPath.value);
    } finally {
      loading.value = false;
    }
  };

  const previewImage = async (row: FileItem) => {
    try {
      const blob = await fileManagerApi.downloadFile(row.path);
      if (imageUrl.value) URL.revokeObjectURL(imageUrl.value);
      imageUrl.value = URL.createObjectURL(blob);
      imagePreviewVisible.value = true;
    } catch {
    }
  };

  onScopeDispose(() => {
    window.removeEventListener('niu:files-changed', refreshUploadedDirectory);
    if (imageUrl.value) URL.revokeObjectURL(imageUrl.value);
  });

  return {
    extractArchive,
    handleBatchDownload,
    handleDownload,
    imagePreviewVisible,
    imageUrl,
    performUpload,
    previewImage,
  };
}
