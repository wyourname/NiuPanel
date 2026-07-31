import { onScopeDispose, ref, type Ref } from "vue";
import { ElMessage } from "element-plus";
import * as fileManagerApi from "@/api/file_manager";
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

  const performUpload = async (files: FileList | null) => {
    if (!files || files.length === 0) return;

    loading.value = true;
    try {
      const formData = new FormData();
      for (let i = 0; i < files.length; i++) {
        formData.append("files", files[i]);
      }
      await fileManagerApi.uploadFile(currentPath.value, formData);
      ElMessage.success(`成功上传 ${files.length} 个文件`);
      await loadContents(currentPath.value);
    } finally {
      loading.value = false;
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
    extractArchive,
    handleBatchDownload,
    handleDownload,
    imagePreviewVisible,
    imageUrl,
    performUpload,
    previewImage,
  };
}
