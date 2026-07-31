import type { FileNode } from "@/types";

export type TaskShareFileSelection = {
  main: string | null;
  deps: Set<string>;
};

export type CheckedFileOption = {
  label: string;
  value: string;
};

export type TaskShareFileSelectorExpose = {
  getCheckedNodes: () => FileNode[];
  setCheckedKeys: (keys: string[]) => void;
  setCheckedNodes: (nodes: FileNode[]) => void;
};
