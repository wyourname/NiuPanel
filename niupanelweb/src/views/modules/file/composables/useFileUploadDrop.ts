import { ref } from "vue";
import type { FileItem } from "@/composables/file/fileOperationTypes";

export const INTERNAL_FILE_DRAG_MIME = "application/x-niupanel-files";

type InternalFileDragPayload = {
  files?: FileItem[];
};

const getInternalDraggedFiles = (event: DragEvent): FileItem[] => {
  const raw = event.dataTransfer?.getData(INTERNAL_FILE_DRAG_MIME);
  if (!raw) return [];

  try {
    const payload = JSON.parse(raw) as InternalFileDragPayload;
    return Array.isArray(payload.files) ? payload.files : [];
  } catch {
    return [];
  }
};

export function useFileUploadDrop(
  performUpload: (files: FileList) => void,
  copyInternalFiles?: (files: FileItem[]) => Promise<void>,
) {
  const fileInputRef = ref<HTMLInputElement | null>(null);
  const dragOver = ref(false);

  const triggerFileUpload = () => fileInputRef.value?.click();

  const handleFileUpload = (event: Event) => {
    const target = event.target;
    if (!(target instanceof HTMLInputElement)) return;
    if (target.files) performUpload(target.files);
  };

  const handleDragOver = (event: DragEvent) => {
    dragOver.value = true;
    if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
  };

  const handleDragLeave = () => {
    dragOver.value = false;
  };

  const handleDrop = async (event: DragEvent) => {
    dragOver.value = false;
    const internalFiles = getInternalDraggedFiles(event);
    if (internalFiles.length > 0 && copyInternalFiles) {
      await copyInternalFiles(internalFiles);
      return;
    }

    if (event.dataTransfer?.files?.length) performUpload(event.dataTransfer.files);
  };

  return {
    dragOver,
    fileInputRef,
    handleDragLeave,
    handleDragOver,
    handleDrop,
    handleFileUpload,
    triggerFileUpload,
  };
}
