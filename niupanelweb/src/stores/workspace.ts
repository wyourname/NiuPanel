import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { workspaceAppMap } from "@/workspace/apps";
import type {
  FileEditorWindowPayload,
  PluginWorkspaceWindowPayload,
  TaskEditorWindowMode,
  TaskEditorWindowPayload,
  TaskLogWindowPayload,
  WorkspaceAppId,
  WorkspaceWindow,
  WorkspaceWindowBounds,
  WorkspaceWindowPlacement,
} from "@/types/workspace";
import type { FileItem, PluginAppRecord, Task } from "@/types";
import { primaryPluginRoute, usePluginAppsStore } from "@/stores/pluginApps";
import { normalizePluginRoute, pluginRoutePath } from "@/utils/pluginRoutes";

const taskWizardWindowWidth = 480;

type WorkspaceCloseGuard = () => boolean | Promise<boolean>;

const defaultBounds = (offset: number): WorkspaceWindowBounds => ({
  x: 120 + offset,
  y: 78 + offset,
  width: 900,
  height: 620,
});

const taskEditorMeta: Record<
  TaskEditorWindowMode,
  { title: string; icon: string }
> = {
  create: { title: "新建任务", icon: "i-ep-plus" },
  edit: { title: "编辑任务", icon: "i-ep-edit" },
  script: { title: "编辑脚本", icon: "i-ep-document" },
  variables: { title: "任务变量", icon: "i-ep-key" },
  cron: { title: "定时规则", icon: "i-ep-clock" },
};

const getViewportBounds = (): WorkspaceWindowBounds => {
  if (typeof window === "undefined") {
    return { x: 16, y: 16, width: 900, height: 620 };
  }

  return {
    x: 12,
    y: 12,
    width: Math.max(320, window.innerWidth - 24),
    height: Math.max(240, window.innerHeight - 104),
  };
};

const getWorkspaceArea = (): WorkspaceWindowBounds => {
  if (typeof window === "undefined") {
    return { x: 16, y: 56, width: 1120, height: 680 };
  }

  return {
    x: 16,
    y: 58,
    width: Math.max(640, window.innerWidth - 32),
    height: Math.max(420, window.innerHeight - 148),
  };
};

const getTaskEditorBounds = (
  mode: TaskEditorWindowMode,
  offset: number,
): WorkspaceWindowBounds => {
  const area = getWorkspaceArea();

  if (mode === "script") {
    return {
      x: area.x + Math.max(0, Math.floor((area.width - 1120) / 2)) + offset,
      y: area.y + 12 + offset,
      width: Math.min(1120, area.width - offset),
      height: Math.min(740, area.height - 24 - offset),
    };
  }

  if (mode === "variables") {
    return {
      x: area.x + Math.max(0, Math.floor((area.width - 980) / 2)) + offset,
      y: area.y + 18 + offset,
      width: Math.min(980, area.width - offset),
      height: Math.min(680, area.height - 28 - offset),
    };
  }

  if (mode === "cron") {
    return {
      x: area.x + Math.max(0, Math.floor((area.width - 620) / 2)) + offset,
      y: area.y + 32 + offset,
      width: Math.min(620, area.width - offset),
      height: Math.min(560, area.height - 40 - offset),
    };
  }

  return {
    x:
      area.x +
      Math.max(0, Math.floor((area.width - taskWizardWindowWidth) / 2)) +
      offset,
    y: area.y + 18 + offset,
    width: Math.min(taskWizardWindowWidth, area.width - offset),
    height: Math.min(700, area.height - 30 - offset),
  };
};

const getFileEditorBounds = (offset: number): WorkspaceWindowBounds => {
  const area = getWorkspaceArea();
  const width = Math.max(560, Math.min(1120, area.width - 24 - offset));
  const height = Math.max(420, Math.min(760, area.height - 24 - offset));

  return {
    x: area.x + Math.max(0, Math.floor((area.width - width) / 2)) + offset,
    y: area.y + 12 + offset,
    width,
    height,
  };
};

const getTaskEditorWindowMode = (
  target: WorkspaceWindow,
): TaskEditorWindowMode | null => {
  if (target.appId !== "task-editor") return null;
  return (target.payload as TaskEditorWindowPayload).mode ?? null;
};

const pluginRouteMeta = (
  app: PluginAppRecord,
  routePath: string,
) =>
  app.ui.routes.find(
    (route) => pluginRoutePath(app.plugin_id, route.path) === routePath,
  ) ?? primaryPluginRoute(app);

const getRestoredWindowBounds = (
  target: WorkspaceWindow,
  offset: number,
): WorkspaceWindowBounds => {
  const taskEditorMode = getTaskEditorWindowMode(target);
  if (taskEditorMode) return getTaskEditorBounds(taskEditorMode, offset);
  if (target.appId === "file-editor") return getFileEditorBounds(offset);
  return defaultBounds(offset);
};

export const useWorkspaceStore = defineStore("workspace", () => {
  const windows = ref<WorkspaceWindow[]>([]);
  const activeWindowId = ref<string | null>(null);
  const zCounter = ref(80);
  const windowSeq = ref(0);
  const closeGuards = new Map<string, WorkspaceCloseGuard>();

  const activeWindow = computed(() =>
    windows.value.find((item) => item.id === activeWindowId.value) ?? null,
  );

  const visibleWindows = computed(() =>
    windows.value.filter((item) => !item.minimized),
  );

  const windowsByZIndex = computed(() =>
    [...windows.value].sort((a, b) => a.zIndex - b.zIndex),
  );

  const taskLogWindows = computed(() =>
    windows.value.filter((item) => item.appId === "task-log"),
  );

  const nextZIndex = () => {
    zCounter.value += 1;
    return zCounter.value;
  };

  const nextWindowSeq = () => {
    windowSeq.value += 1;
    return windowSeq.value;
  };

  const focusWindow = (id: string) => {
    const target = windows.value.find((item) => item.id === id);
    if (!target) return;

    target.minimized = false;
    target.zIndex = nextZIndex();
    activeWindowId.value = id;
  };

  const closeWindow = (id: string) => {
    const index = windows.value.findIndex((item) => item.id === id);
    if (index === -1) return;

    windows.value.splice(index, 1);
    closeGuards.delete(id);
    if (activeWindowId.value === id) {
      const next = windows.value.at(-1);
      activeWindowId.value = next?.id ?? null;
      if (next) next.zIndex = nextZIndex();
    }
  };

  const closeActiveWindow = () => {
    if (!activeWindowId.value) return;
    closeWindow(activeWindowId.value);
  };

  const registerCloseGuard = (id: string, guard: WorkspaceCloseGuard) => {
    closeGuards.set(id, guard);

    return () => {
      if (closeGuards.get(id) === guard) closeGuards.delete(id);
    };
  };

  const requestCloseWindow = async (id: string) => {
    const guard = closeGuards.get(id);
    if (guard && !(await guard())) return false;

    closeWindow(id);
    return true;
  };

  const requestCloseActiveWindow = async () => {
    if (!activeWindowId.value) return false;
    return requestCloseWindow(activeWindowId.value);
  };

  const requestCloseAll = async () => {
    const targets = [...windows.value].sort((a, b) => b.zIndex - a.zIndex);

    for (const target of targets) {
      const guard = closeGuards.get(target.id);
      if (guard && !(await guard())) return false;
    }

    targets.forEach((target) => closeWindow(target.id));
    return true;
  };

  const closeWindowsByApp = (appId: WorkspaceAppId) => {
    const shouldPickNext = activeWindow.value?.appId === appId;
    const closingIds = windows.value
      .filter((item) => item.appId === appId)
      .map((item) => item.id);
    windows.value = windows.value.filter((item) => item.appId !== appId);
    closingIds.forEach((id) => closeGuards.delete(id));
    if (shouldPickNext) {
      const next = windows.value.at(-1);
      activeWindowId.value = next?.id ?? null;
      if (next) next.zIndex = nextZIndex();
    }
  };

  const minimizeWindow = (id: string) => {
    const target = windows.value.find((item) => item.id === id);
    if (!target) return;

    target.minimized = true;
    if (activeWindowId.value === id) {
      const next = [...windows.value]
        .reverse()
        .find((item) => !item.minimized && item.id !== id);
      activeWindowId.value = next?.id ?? null;
      if (next) next.zIndex = nextZIndex();
    }
  };

  const minimizeActiveWindow = () => {
    if (!activeWindowId.value) return;
    minimizeWindow(activeWindowId.value);
  };

  const minimizeWindowsByApp = (appId: WorkspaceAppId) => {
    windows.value.forEach((item) => {
      if (item.appId === appId) item.minimized = true;
    });

    if (activeWindow.value?.appId === appId) {
      const next = [...windows.value]
        .reverse()
        .find((item) => !item.minimized && item.appId !== appId);
      activeWindowId.value = next?.id ?? null;
      if (next) next.zIndex = nextZIndex();
    }
  };

  const toggleMaximizeWindow = (id: string) => {
    const target = windows.value.find((item) => item.id === id);
    if (!target) return;

    target.maximized = !target.maximized;
    target.bounds = target.maximized
      ? getViewportBounds()
      : getRestoredWindowBounds(target, 20);
    focusWindow(id);
  };

  const updateWindowBounds = (
    id: string,
    bounds: Partial<WorkspaceWindowBounds>,
  ) => {
    const target = windows.value.find((item) => item.id === id);
    if (!target || target.maximized) return;

    target.bounds = {
      ...target.bounds,
      ...bounds,
    };
  };

  const placeWindow = (
    id: string,
    placement: WorkspaceWindowPlacement,
  ) => {
    const target = windows.value.find((item) => item.id === id);
    if (!target) return;

    const area = getWorkspaceArea();
    const gap = 12;
    const halfWidth = Math.floor((area.width - gap) / 2);

    target.maximized = false;

    if (placement === "left") {
      target.bounds = {
        x: area.x,
        y: area.y,
        width: halfWidth,
        height: area.height,
      };
    } else if (placement === "right") {
      target.bounds = {
        x: area.x + halfWidth + gap,
        y: area.y,
        width: halfWidth,
        height: area.height,
      };
    } else if (placement === "center") {
      const taskEditorMode = getTaskEditorWindowMode(target);
      if (target.appId === "file-editor") {
        target.bounds = getFileEditorBounds(0);
      } else if (taskEditorMode) {
        target.bounds = getTaskEditorBounds(taskEditorMode, 0);
      } else {
        target.bounds = {
          x: area.x + Math.max(0, Math.floor((area.width - 980) / 2)),
          y: area.y + 18,
          width: Math.min(980, area.width),
          height: Math.min(680, area.height - 20),
        };
      }
    } else {
      target.bounds = getRestoredWindowBounds(
        target,
        (windows.value.length % 5) * 28,
      );
    }

    focusWindow(id);
  };

  const focusNextWindow = () => {
    if (windows.value.length === 0) return;

    const ordered = windowsByZIndex.value;
    const currentIndex = activeWindowId.value
      ? ordered.findIndex((item) => item.id === activeWindowId.value)
      : -1;
    const next = ordered[(currentIndex + 1) % ordered.length] ?? ordered[0];
    if (next) focusWindow(next.id);
  };

  const cascadeWindows = () => {
    const area = getWorkspaceArea();
    const targets = windows.value.filter((item) => !item.minimized);
    targets.forEach((item, index) => {
      const offset = (index % 7) * 26;
      item.maximized = false;
      item.bounds = {
        x: area.x + offset,
        y: area.y + offset,
        width: Math.min(980, area.width - offset),
        height: Math.min(660, area.height - offset),
      };
      item.zIndex = nextZIndex();
    });
    activeWindowId.value = targets.at(-1)?.id ?? activeWindowId.value;
  };

  const tileVisibleWindows = () => {
    const targets = windows.value.filter((item) => !item.minimized);
    if (targets.length === 0) return;

    const area = getWorkspaceArea();
    const gap = 12;
    const columns = Math.ceil(Math.sqrt(targets.length));
    const rows = Math.ceil(targets.length / columns);
    const width = Math.floor((area.width - gap * (columns - 1)) / columns);
    const height = Math.floor((area.height - gap * (rows - 1)) / rows);

    targets.forEach((item, index) => {
      const col = index % columns;
      const row = Math.floor(index / columns);
      item.maximized = false;
      item.bounds = {
        x: area.x + col * (width + gap),
        y: area.y + row * (height + gap),
        width,
        height,
      };
      item.zIndex = nextZIndex();
    });
    activeWindowId.value = targets[0]?.id ?? activeWindowId.value;
  };

  const openWindow = (
    appId: WorkspaceAppId,
    documentKey: string,
    payload: WorkspaceWindow["payload"],
    options: {
      title?: string;
      subtitle?: string;
      icon?: string;
      bounds?: WorkspaceWindowBounds;
    } = {},
  ) => {
    const app = workspaceAppMap.get(appId);
    const existing = windows.value.find(
      (item) => item.appId === appId && item.documentKey === documentKey,
    );

    if (existing) {
      focusWindow(existing.id);
      return existing;
    }

    const id = `${appId}:${documentKey}:${Date.now()}:${nextWindowSeq()}`;
    const offset = (windows.value.length % 5) * 28;
    const created: WorkspaceWindow = {
      id,
      appId,
      documentKey,
      title: options.title ?? app?.title ?? "Workspace",
      subtitle: options.subtitle,
      icon: options.icon ?? app?.icon ?? "i-ep-copy-document",
      bounds: options.bounds ?? defaultBounds(offset),
      zIndex: nextZIndex(),
      minimized: false,
      maximized: false,
      payload,
    };

    windows.value.push(created);
    activeWindowId.value = id;
    return created;
  };

  const openTaskLogWindow = (task: Task, runId: number | null = null) => {
    const documentKey = `task-log:${task.id}:${runId ?? "live"}`;
    const payload: TaskLogWindowPayload = {
      task,
      taskId: task.id,
      taskName: task.name,
      runId,
    };

    return openWindow("task-log", documentKey, payload, {
      title: task.name,
      subtitle: runId ? `运行 #${runId}` : "实时日志",
      icon: "i-ep-monitor",
    });
  };

  const openTaskEditorWindow = (
    task: Task,
    mode: TaskEditorWindowMode,
  ) => {
    if (mode === "create") return openTaskCreateWindow();

    const documentKey = `task-editor:${mode}:${task.id}`;
    const payload: TaskEditorWindowPayload = {
      task,
      taskId: task.id,
      taskName: task.name,
      mode,
    };
    const meta = taskEditorMeta[mode];
    const offset = (windows.value.length % 5) * 22;

    return openWindow("task-editor", documentKey, payload, {
      title: `${meta.title} - ${task.name}`,
      icon: meta.icon,
      bounds: getTaskEditorBounds(mode, offset),
    });
  };

  const openTaskCreateWindow = (options: { uploadedFile?: File } = {}) => {
    const documentKey = options.uploadedFile
      ? `task-editor:create-upload:${options.uploadedFile.name}:${options.uploadedFile.lastModified}:${nextWindowSeq()}`
      : "task-editor:create";
    const payload: TaskEditorWindowPayload = {
      mode: "create",
      uploadedFile: options.uploadedFile,
    };
    const offset = (windows.value.length % 5) * 22;

    return openWindow("task-editor", documentKey, payload, {
      title: "新建任务",
      icon: "i-ep-plus",
      bounds: getTaskEditorBounds("create", offset),
    });
  };

  const openFileEditorWindow = (file: Pick<FileItem, "name" | "path">) => {
    const payload: FileEditorWindowPayload = {
      fileName: file.name,
      filePath: file.path,
      session: {
        content: "",
        initialized: false,
        loadError: "",
        loading: false,
        savedContent: "",
        saving: false,
      },
    };
    const offset = (windows.value.length % 5) * 22;

    return openWindow("file-editor", `file-editor:${file.path}`, payload, {
      title: file.name,
      subtitle: file.path,
      icon: "i-ep-document",
      bounds: getFileEditorBounds(offset),
    });
  };

  const openAppWindow = (
    appId: WorkspaceAppId,
    options: {
      forceNew?: boolean;
    } = {},
  ) => {
    if (
      appId === "task-log" ||
      appId === "task-editor" ||
      appId === "file-editor"
    ) {
      return null;
    }

    const app = workspaceAppMap.get(appId);
    if (!app) return null;

    const canCreateInstance = app.launchPolicy === "multi" && options.forceNew;
    const existing = canCreateInstance
      ? null
      : [...windows.value].reverse().find((item) => item.appId === appId);

    if (existing) {
      focusWindow(existing.id);
      return existing;
    }

    const instanceNumber =
      windows.value.filter((item) => item.appId === appId).length + 1;
    const documentKey = canCreateInstance
      ? `app:${appId}:${nextWindowSeq()}`
      : `app:${appId}`;

    return openWindow(appId, documentKey, {}, {
      title: app.title,
      subtitle: instanceNumber > 1 ? `窗口 ${instanceNumber}` : undefined,
      icon: app.icon,
      bounds: {
        x: 88 + (windows.value.length % 4) * 30,
        y: 58 + (windows.value.length % 4) * 26,
        width: Math.min(1180, Math.max(860, window.innerWidth - 180)),
        height: Math.min(760, Math.max(560, window.innerHeight - 170)),
      },
    });
  };

  const openPluginAppWindow = (app: PluginAppRecord, path?: string) => {
    const route = primaryPluginRoute(app);
    const appId = `plugin:${app.plugin_id}` as WorkspaceAppId;
    const normalizedRoute = normalizePluginRoute(
      app.plugin_id,
      path ?? route?.path,
    );
    const payload: PluginWorkspaceWindowPayload = {
      pluginId: app.plugin_id,
      routePath: normalizedRoute.routePath,
      routeQuery: normalizedRoute.routeQuery,
    };

    return openWindow(appId, `plugin:${app.plugin_id}`, payload, {
      title: route?.title ?? app.name,
      icon: route?.icon ?? "i-ep-box",
      bounds: {
        x: 88 + (windows.value.length % 4) * 30,
        y: 58 + (windows.value.length % 4) * 26,
        width: Math.min(1180, Math.max(860, window.innerWidth - 180)),
        height: Math.min(760, Math.max(560, window.innerHeight - 170)),
      },
    });
  };

  const navigatePluginWindow = (
    windowId: string,
    path: string,
  ) => {
    const target = windows.value.find((item) => item.id === windowId);
    if (!target || !target.appId.startsWith("plugin:")) return;

    const payload = target.payload as PluginWorkspaceWindowPayload;
    const normalizedRoute = normalizePluginRoute(payload.pluginId, path);
    target.payload = {
      ...payload,
      routePath: normalizedRoute.routePath,
      routeQuery: normalizedRoute.routeQuery,
    };

    const app = usePluginAppsStore().getApp(payload.pluginId);
    const route = app ? pluginRouteMeta(app, normalizedRoute.routePath) : null;
    target.title = route?.title ?? target.title;
    target.icon = route?.icon ?? target.icon;
  };

  const closeAll = () => {
    windows.value = [];
    activeWindowId.value = null;
    closeGuards.clear();
  };

  const minimizeAll = () => {
    windows.value.forEach((item) => {
      item.minimized = true;
    });
    activeWindowId.value = null;
  };

  return {
    activeWindow,
    activeWindowId,
    cascadeWindows,
    closeAll,
    closeActiveWindow,
    closeWindow,
    closeWindowsByApp,
    focusNextWindow,
    focusWindow,
    minimizeActiveWindow,
    minimizeWindow,
    minimizeAll,
    minimizeWindowsByApp,
    navigatePluginWindow,
    openAppWindow,
    openFileEditorWindow,
    openPluginAppWindow,
    openTaskCreateWindow,
    openTaskEditorWindow,
    openTaskLogWindow,
    openWindow,
    placeWindow,
    registerCloseGuard,
    requestCloseActiveWindow,
    requestCloseAll,
    requestCloseWindow,
    taskLogWindows,
    toggleMaximizeWindow,
    tileVisibleWindows,
    updateWindowBounds,
    visibleWindows,
    windows,
    windowsByZIndex,
  };
});
