import type { Component } from "vue";
import type { Task } from "./tasks";

export type BuiltinWorkspaceAppId =
  | "overview"
  | "tasks"
  | "variables"
  | "files"
  | "environments"
  | "share"
  | "extensions"
  | "agents"
  | "git"
  | "telegram"
  | "compiler"
  | "terminal"
  | "settings"
  | "more"
  | "task-log"
  | "task-editor";

export type WorkspaceAppId = BuiltinWorkspaceAppId | `plugin:${string}`;

export type WorkspaceLaunchPolicy = "singleton" | "multi";
export type WorkspaceMobileMode = "fullscreen" | "stack";

export type WorkspaceAppDefinition = {
  id: WorkspaceAppId;
  title: string;
  icon: string;
  routeName?: string;
  launchPolicy: WorkspaceLaunchPolicy;
  mobileMode: WorkspaceMobileMode;
  component?: Component;
};

export type WorkspaceWindowBounds = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type WorkspaceWindowPlacement = "center" | "left" | "right" | "restore";

export type TaskLogWindowPayload = {
  task: Task;
  taskId: number;
  taskName: string;
  runId: number | null;
};

export type TaskEditorWindowMode =
  | "create"
  | "edit"
  | "script"
  | "variables"
  | "cron";

export type TaskEditorWindowPayload = {
  task?: Task;
  taskId?: number;
  taskName?: string;
  mode: TaskEditorWindowMode;
  uploadedFile?: File;
};

export type PluginWorkspaceWindowPayload = {
  pluginId: string;
  routePath: string;
  routeQuery: Record<string, unknown>;
};

export type WorkspaceWindowPayload =
  | TaskLogWindowPayload
  | TaskEditorWindowPayload
  | PluginWorkspaceWindowPayload
  | Record<string, never>;

export type WorkspaceWindow = {
  id: string;
  appId: WorkspaceAppId;
  documentKey: string;
  title: string;
  subtitle?: string;
  icon: string;
  bounds: WorkspaceWindowBounds;
  zIndex: number;
  minimized: boolean;
  maximized: boolean;
  payload: WorkspaceWindowPayload;
};
