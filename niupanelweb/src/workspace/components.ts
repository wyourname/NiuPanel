import { defineAsyncComponent, type Component } from "vue";
import type { WorkspaceAppId } from "@/types/workspace";

type WorkspaceComponentModule = Promise<{ default: Component }>;

/**
 * Router pages and desktop windows share the same import functions so Rollup
 * emits one lazy chunk per application instead of one eager copy per shell.
 */
export const workspaceAppLoaders = {
  overview: () => import("@/views/modules/overview/index.vue"),
  tasks: () => import("@/views/modules/Tasks.vue"),
  variables: () => import("@/views/modules/Variables.vue"),
  files: () => import("@/views/modules/File.vue"),
  environments: () => import("@/views/modules/Environment.vue"),
  share: () => import("@/views/modules/share/index.vue"),
  extensions: () => import("@/views/modules/extensions/index.vue"),
  git: () => import("@/views/modules/git/index.vue"),
  telegram: () => import("@/views/modules/telegram/index.vue"),
  terminal: () => import("@/views/modules/terminal/index.vue"),
  settings: () => import("@/views/modules/settings/index.vue"),
  more: () => import("@/views/modules/more/index.vue"),
} satisfies Partial<Record<WorkspaceAppId, () => WorkspaceComponentModule>>;

export const workspaceAppComponents = Object.fromEntries(
  Object.entries(workspaceAppLoaders).map(([appId, loader]) => [
    appId,
    defineAsyncComponent({ loader }),
  ]),
) as Partial<Record<WorkspaceAppId, Component>>;
