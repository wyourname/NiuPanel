import { ref, type Ref } from "vue";
import type { ContextMenuItem } from "@/components/common/contextMenuTypes";
import type { FileItem, FileTableRef } from "@/composables/useFileOperations";
import {
  handleContextMenuAction,
  isFileCommand,
  type FileActionHandlers,
} from "../utils/fileActions";
import { isArchiveFile, isEditableFile, isImageFile } from "../utils/fileDisplay";

type UseFileContextMenuOptions = {
  actionHandlers: FileActionHandlers;
  fileTableRef: Ref<FileTableRef | null>;
  selectedFiles: Ref<FileItem[]>;
};

const createContextMenuItems = (row: FileItem): ContextMenuItem[] => {
  const items: ContextMenuItem[] = [];

  if (row.is_dir) {
    items.push({ label: "打开", action: "open", icon: "i-ep-folder-opened" });
  } else if (isEditableFile(row.name)) {
    items.push({ label: "编辑", action: "edit", icon: "i-ep-edit" });
  } else if (isImageFile(row.name)) {
    items.push({ label: "预览", action: "preview", icon: "i-ep-picture" });
  }

  if (!row.is_dir) {
    items.push({ label: "下载", action: "download", icon: "i-ep-download" });
    if (isArchiveFile(row.name)) {
      items.push({ label: "解压", action: "extract", icon: "i-ep-box" });
    }
  }

  items.push({ type: "divider" });
  items.push({ label: "复制", action: "copy", icon: "i-ep-copy-document" });
  items.push({ label: "剪切", action: "cut", icon: "i-ep-scissor" });
  items.push({ label: "重命名", action: "rename", icon: "i-ep-edit-pen" });
  items.push({ label: "移动", action: "move", icon: "i-ep-position" });
  items.push({ type: "divider" });
  items.push({
    label: "删除",
    action: "delete",
    icon: "i-ep-delete",
    class: "text-red-500",
  });

  return items;
};

export function useFileContextMenu(options: UseFileContextMenuOptions) {
  const contextMenuVisible = ref(false);
  const contextMenuPosition = ref({ x: 0, y: 0 });
  const contextMenuItems = ref<ContextMenuItem[]>([]);
  const contextRow = ref<FileItem | null>(null);

  const handleRowContextMenu = (row: FileItem, event: MouseEvent) => {
    event.preventDefault();
    contextRow.value = row;
    options.fileTableRef.value?.setCurrentRow?.(row);
    contextMenuItems.value = createContextMenuItems(row);
    contextMenuPosition.value = { x: event.clientX, y: event.clientY };
    contextMenuVisible.value = true;
  };

  const handleContextMenuSelect = (action: string) => {
    if (!isFileCommand(action)) return;
    if (!contextRow.value) return;

    const row = contextRow.value;
    const isRowSelected = options.selectedFiles.value.some(
      (file) => file.path === row.path,
    );
    const items = isRowSelected ? options.selectedFiles.value : [];
    handleContextMenuAction(action, row, items, options.actionHandlers);
  };

  return {
    contextMenuItems,
    contextMenuPosition,
    contextMenuVisible,
    handleContextMenuSelect,
    handleRowContextMenu,
  };
}
