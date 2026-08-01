import { onScopeDispose, ref, type Ref } from "vue";
import { ElMessage } from "element-plus";
import * as fileManagerApi from "@/api/file_manager";
import { createUploadFormData } from "@/api/upload";
import { useUploadTransfer } from "@/composables/useUploadTransfer";
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
  const uploadLabel = ref("");
  const {
    cancel: cancelUpload,
    loadedBytes: uploadLoadedBytes,
    progress: uploadProgress,
    run: runUpload,
    totalBytes: uploadTotalBytes,
    uploading,
  } = useUploadTransfer();

  const performUpload = async (files: FileList | null) => {
    if (!files || files.length === 0) return;

    const selectedFiles = Array.from(files);
    const selectedBytes = selectedFiles.reduce((total, file) => total + file.size, 0);
    uploadLabel.value = selectedFiles.length === 1
      ? selectedFiles[0].name
      : `${selectedFiles.length} 个文件`;
    loading.value = true;
    try {
      const formData = createUploadFormData(
        selectedFiles.map((file) => ["files", file] as const),
      );
      const result = await runUpload(
        (options) => fileManagerApi.uploadFile(currentPath.value, formData, options),
        { initialTotalBytes: selectedBytes },
      );
      if (result.cancelled) {
        ElMessage.info("上传已取消");
        return;
      }
      ElMessage.success(`成功上传 ${selectedFiles.length} 个文件`);
      await loadContents(currentPath.value);
    } catch (error) {
      ElMessage.error(error instanceof Error ? error.message : "文件上传失败");
    } finally {
      if (!uploading.value) {
        loading.value = false;
      }
    }
  };

  const handleDownload = async (row: FileItem) => {
    try {
      const blob = await fileManagerApi.downloadFile(row.path);
      const url = window.URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = row.name;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      window.URL.revokeObjectURL(url);
    } catch {
    }
  };

  const handleBatchDownload = async (files: FileItem[]) => {
    if (files.length === 0) return;

    loading.value = true;
    try {
      const paths = files.map((file) => file.path);
      const blob = await fileManagerApi.downloadBatch(paths);
      const url = window.URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = `batch_download_${new Date().getTime()}.tar`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      window.URL.revokeObjectURL(url);
      ElMessage.success("批量下载已启动");
    } catch {
      ElMessage.error("批量打包下载失败");
    } finally {
      loading.value = false;
    }
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
    if (imageUrl.value) URL.revokeObjectURL(imageUrl.value);
  });

  return {
    cancelUpload,
    extractArchive,
    handleBatchDownload,
    handleDownload,
    imagePreviewVisible,
    imageUrl,
    performUpload,
    previewImage,
    uploadLabel,
    uploadLoadedBytes,
    uploadProgress,
    uploadTotalBytes,
    uploading,
  };
}
