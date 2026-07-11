import type { Component } from "vue";
import type { LocationQueryRaw } from "vue-router";

export type PaletteItemType = "nav" | "task" | "command" | "variable" | "env";

export type PaletteCommandAction = "refresh" | "toggle_theme" | "logout";

export type PaletteItem = {
  title: string;
  desc?: string;
  path?: string;
  query?: LocationQueryRaw;
  action?: PaletteCommandAction;
  icon?: Component;
  type: PaletteItemType;
  actionText?: string;
  id?: number | string;
};
