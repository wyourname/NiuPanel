import type { FileItem } from "@/types";

export type { FileItem };

export type FileTableRef = {
  clearSelection?: () => void;
  setCurrentRow?: (row: FileItem | null) => void;
  toggleRowSelection?: (row: FileItem, selected?: boolean) => void;
};

export interface Breadcrumb {
  name: string;
  path: string;
  type?: "ellipsis";
  items?: { name: string; path: string }[];
}

export type FileClipboardState = {
  action: "copy" | "cut" | null;
  files: FileItem[];
};
