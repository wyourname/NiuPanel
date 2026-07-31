import { ref } from "vue";
import { defineStore } from "pinia";
import type {
  FileClipboardState,
  FileItem,
} from "@/composables/file/fileOperationTypes";

export const useFileClipboardStore = defineStore("fileClipboard", () => {
  const clipboard = ref<FileClipboardState>({ action: null, files: [] });

  const setClipboard = (
    action: Exclude<FileClipboardState["action"], null>,
    files: FileItem[],
  ) => {
    clipboard.value = {
      action,
      files: files.map((file) => ({ ...file })),
    };
  };

  const clearClipboard = () => {
    clipboard.value = { action: null, files: [] };
  };

  return {
    clearClipboard,
    clipboard,
    setClipboard,
  };
});
