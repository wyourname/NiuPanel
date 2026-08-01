import { onScopeDispose, ref } from "vue";
import type { UploadRequestOptions } from "@/api/upload";

export type UploadProgressSnapshot = {
  loadedBytes: number;
  percentage: number;
  totalBytes: number;
};

export type UploadTransferResult<T> =
  | { cancelled: true }
  | { cancelled: false; value: T };

type RunUploadOptions = {
  initialTotalBytes?: number;
  onProgress?: (progress: UploadProgressSnapshot) => void;
};

export function useUploadTransfer() {
  const uploading = ref(false);
  const progress = ref(0);
  const loadedBytes = ref(0);
  const totalBytes = ref(0);
  let controller: AbortController | null = null;

  const snapshot = (): UploadProgressSnapshot => ({
    loadedBytes: loadedBytes.value,
    percentage: progress.value,
    totalBytes: totalBytes.value,
  });

  const cancel = () => controller?.abort();

  const run = async <T>(
    request: (options: UploadRequestOptions) => Promise<T>,
    options: RunUploadOptions = {},
  ): Promise<UploadTransferResult<T>> => {
    controller?.abort();
    const currentController = new AbortController();
    controller = currentController;
    uploading.value = true;
    progress.value = 0;
    loadedBytes.value = 0;
    totalBytes.value = options.initialTotalBytes ?? 0;
    options.onProgress?.(snapshot());

    try {
      const value = await request({
        signal: currentController.signal,
        onUploadProgress: (event) => {
          if (controller !== currentController) return;
          loadedBytes.value = event.loaded;
          if (event.total) totalBytes.value = event.total;
          if (totalBytes.value > 0) {
            progress.value = Math.min(
              99,
              Math.round((loadedBytes.value / totalBytes.value) * 100),
            );
          }
          options.onProgress?.(snapshot());
        },
      });
      if (currentController.signal.aborted) return { cancelled: true };
      if (controller === currentController) {
        progress.value = 100;
        if (totalBytes.value > 0) loadedBytes.value = totalBytes.value;
        options.onProgress?.(snapshot());
      }
      return { cancelled: false, value };
    } catch (error) {
      if (currentController.signal.aborted) return { cancelled: true };
      throw error;
    } finally {
      if (controller === currentController) {
        controller = null;
        uploading.value = false;
      }
    }
  };

  onScopeDispose(cancel);

  return {
    cancel,
    loadedBytes,
    progress,
    run,
    totalBytes,
    uploading,
  };
}
