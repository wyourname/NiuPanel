import type { Ref } from "vue";
import { ElMessage } from "element-plus";
import type { FileItem } from "@/composables/useFileOperations";
import {
  handleDesktopFileCommand,
  handleFileItemOpen,
  handleMobileFileCommand,
  type FileActionHandlers,
  type FileCommand,
} from "../utils/fileActions";
import { isEditableFile, isImageFile } from "../utils/fileDisplay";

export type FileCreateCommand = "directory" | "download_url" | "file";

type FileCreateType = Exclude<FileCreateCommand, "download_url">;

type CreateForm = {
  name: string;
};

type DownloadUrlForm = {
  filename: string;
  url: string;
};

type UseFileCommandHandlersOptions = FileActionHandlers & {
  createDialogVisible: Ref<boolean>;
  createForm: Ref<CreateForm>;
  createType: Ref<FileCreateType>;
  downloadUrlDialogVisible: Ref<boolean>;
  downloadUrlForm: Ref<DownloadUrlForm>;
};

export function useFileCommandHandlers(options: UseFileCommandHandlersOptions) {
  const fileActionHandlers: FileActionHandlers = {
    batchDelete: options.batchDelete,
    copyToClipboard: options.copyToClipboard,
    cutToClipboard: options.cutToClipboard,
    deleteItem: options.deleteItem,
    extractArchive: options.extractArchive,
    handleDownload: options.handleDownload,
    navigate: options.navigate,
    previewImage: options.previewImage,
    showEditFileDialog: options.showEditFileDialog,
    showMoveDialog: options.showMoveDialog,
    showRenameDialog: options.showRenameDialog,
  };

  const handleDesktopCommand = (cmd: FileCommand, row: FileItem) =>
    handleDesktopFileCommand(cmd, row, fileActionHandlers);

  const handleMobileCommand = (cmd: FileCommand, row: FileItem) =>
    handleMobileFileCommand(cmd, row, fileActionHandlers);

  const handleItemClick = (row: FileItem) =>
    handleFileItemOpen(row, {
      isEditableFile,
      isImageFile,
      navigate: options.navigate,
      onUnsupported: () => ElMessage.warning("不支持预览"),
      previewImage: options.previewImage,
      showEditFileDialog: options.showEditFileDialog,
    });

  const handleCreateCommand = (cmd: FileCreateCommand) => {
    if (cmd === "download_url") {
      options.downloadUrlForm.value = { url: "", filename: "" };
      options.downloadUrlDialogVisible.value = true;
      return;
    }

    options.createType.value = cmd;
    options.createForm.value.name = "";
    options.createDialogVisible.value = true;
  };

  return {
    fileActionHandlers,
    handleCreateCommand,
    handleDesktopCommand,
    handleItemClick,
    handleMobileCommand,
  };
}
