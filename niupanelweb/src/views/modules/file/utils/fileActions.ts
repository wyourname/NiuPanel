import type { FileItem } from "@/types";

export type FileCommand =
  | "copy"
  | "cut"
  | "delete"
  | "download"
  | "edit"
  | "extract"
  | "move"
  | "open"
  | "preview"
  | "rename";

export const isFileCommand = (command: unknown): command is FileCommand =>
  typeof command === "string" &&
  [
    "copy",
    "cut",
    "delete",
    "download",
    "edit",
    "extract",
    "move",
    "open",
    "preview",
    "rename",
  ].includes(command);

export interface FileActionHandlers {
  showEditFileDialog: (row: FileItem) => void;
  handleDownload: (row: FileItem) => void;
  showRenameDialog: (row: FileItem) => void;
  showMoveDialog: (rows: FileItem[]) => void;
  copyToClipboard: (rows: FileItem[]) => void;
  cutToClipboard: (rows: FileItem[]) => void;
  deleteItem: (row: FileItem) => void;
  extractArchive: (row: FileItem) => void;
  previewImage: (row: FileItem) => void;
  navigate: (path: string) => void;
  batchDelete: () => void;
}

export const handleDesktopFileCommand = (
  cmd: FileCommand,
  row: FileItem,
  handlers: FileActionHandlers,
) => {
  if (cmd === "edit") handlers.showEditFileDialog(row);
  if (cmd === "preview") handlers.previewImage(row);
  if (cmd === "download") handlers.handleDownload(row);
  if (cmd === "extract") handlers.extractArchive(row);
  if (cmd === "rename") handlers.showRenameDialog(row);
  if (cmd === "move") handlers.showMoveDialog([row]);
  if (cmd === "copy") handlers.copyToClipboard([row]);
  if (cmd === "cut") handlers.cutToClipboard([row]);
  if (cmd === "delete") handlers.deleteItem(row);
};

export const handleMobileFileCommand = (
  cmd: FileCommand,
  row: FileItem,
  handlers: FileActionHandlers,
) => {
  if (cmd === "edit") handlers.showEditFileDialog(row);
  if (cmd === "download") handlers.handleDownload(row);
  if (cmd === "extract") handlers.extractArchive(row);
  if (cmd === "rename") handlers.showRenameDialog(row);
  if (cmd === "move") handlers.showMoveDialog([row]);
  if (cmd === "delete") handlers.deleteItem(row);
  if (cmd === "copy") handlers.copyToClipboard([row]);
  if (cmd === "cut") handlers.cutToClipboard([row]);
};

export const handleFileItemOpen = (
  row: FileItem,
  options: {
    navigate: (path: string) => void;
    isImageFile: (name: string) => boolean;
    isEditableFile: (name: string) => boolean;
    previewImage: (row: FileItem) => void;
    showEditFileDialog: (row: FileItem) => void;
    onUnsupported: () => void;
  },
) => {
  if (row.is_dir) {
    options.navigate(row.path);
    return;
  }
  if (options.isImageFile(row.name)) {
    options.previewImage(row);
    return;
  }
  if (options.isEditableFile(row.name)) {
    options.showEditFileDialog(row);
    return;
  }
  options.onUnsupported();
};

export const handleContextMenuAction = (
  action: FileCommand,
  row: FileItem,
  selectedItems: FileItem[],
  handlers: FileActionHandlers,
) => {
  const items = selectedItems.length > 0 ? selectedItems : [row];

  switch (action) {
    case "open":
      handlers.navigate(row.path);
      break;
    case "edit":
      handlers.showEditFileDialog(row);
      break;
    case "preview":
      handlers.previewImage(row);
      break;
    case "download":
      handlers.handleDownload(row);
      break;
    case "extract":
      handlers.extractArchive(row);
      break;
    case "copy":
      handlers.copyToClipboard(items);
      break;
    case "cut":
      handlers.cutToClipboard(items);
      break;
    case "rename":
      handlers.showRenameDialog(row);
      break;
    case "move":
      handlers.showMoveDialog(items);
      break;
    case "delete":
      if (items.length > 1) handlers.batchDelete();
      else handlers.deleteItem(row);
      break;
  }
};
