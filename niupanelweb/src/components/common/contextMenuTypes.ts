export type ContextMenuPosition = {
  x: number;
  y: number;
};

export type ContextMenuItem = {
  type?: "divider";
  label?: string;
  action?: string;
  icon?: string;
  class?: string;
  children?: ContextMenuItem[];
};
