<template>
  <aside
    class="pointer-events-none fixed inset-x-0 bottom-2 z-50 flex justify-center px-1 pb-safe select-none md:bottom-5 md:px-4"
  >
    <div
      ref="dockScrollRef"
      class="dock-shell w-full max-w-full md:w-auto md:max-w-[calc(100vw-32px)]"
      :class="appStore.isMobile ? 'dock-mobile-scroll overflow-x-auto overscroll-x-contain no-scrollbar' : 'justify-center overflow-visible'"
    >
      <button
        type="button"
        class="group dock-item-base h-11 w-11 overflow-hidden rounded-lg bg-primary text-sm font-extrabold text-white shadow-sm md:h-10 md:w-10"
        :class="isOverviewActive ? 'ring-2 ring-primary/25' : 'hover:opacity-90'"
        :aria-current="isOverviewActive ? 'page' : undefined"
        :title="systemName"
        aria-label="系统概览"
        @click="handleOverviewSelect"
        @contextmenu.prevent.stop="openDockContextMenu($event, overviewDockItem)"
      >
        <span v-if="!logoUrl">N</span>
        <img v-else :src="logoUrl" :alt="systemName" class="h-full w-full object-cover" />
      </button>

      <nav
        class="flex min-h-0 shrink-0 overflow-visible gap-1 px-0.5"
        aria-label="主导航"
      >
        <button
          v-for="item in primaryDockItems"
          :key="item.index"
          type="button"
          class="group dock-item-base h-11 w-11 rounded-lg md:h-10 md:w-10"
          :class="isMenuItemActive(item) ? activeDockItemClass : 'dock-item-idle'"
          :aria-current="isMenuItemActive(item) ? 'page' : undefined"
          :aria-label="item.title"
          :title="item.title"
          @click="handleSelect(item)"
          @contextmenu.prevent.stop="openDockContextMenu($event, item)"
        >
          <div
            :class="[
              item.icon,
              appStore.isMobile ? 'text-[18px]' : 'text-[20px]',
              isMenuItemActive(item) ? activeDockIconClass : 'text-muted group-hover:text-default',
            ]"
          ></div>
          <div
            v-if="isMenuItemActive(item)"
            :class="activeDockDotClass"
          ></div>
          <div v-if="!appStore.isMobile" class="dock-tooltip">
            {{ item.title }}
          </div>
        </button>
      </nav>

      <template v-if="visibleWindowItems.length > 0">
        <div class="dock-divider"></div>
        <div class="flex min-w-0 max-w-[420px] items-center gap-1 px-0.5 no-scrollbar">
          <button
            v-for="item in visibleWindowItems"
            :key="item.id"
            type="button"
            class="group dock-item-base h-11 w-11 rounded-lg md:h-10 md:w-10"
            :class="workspace.activeWindowId === item.id ? activeDockItemClass : 'dock-item-idle'"
            :aria-current="workspace.activeWindowId === item.id ? 'page' : undefined"
            :aria-label="item.title"
            :title="item.title"
            @click="handleWindowItemSelect(item.id)"
          >
            <div
              :class="[
                item.icon,
                appStore.isMobile ? 'text-[18px]' : 'text-[20px]',
                workspace.activeWindowId === item.id ? activeDockIconClass : 'text-muted group-hover:text-default',
              ]"
            ></div>
            <div class="absolute -right-0.5 -top-0.5 h-2 w-2 rounded-full bg-emerald-500 ring-2 ring-white dark:ring-[#101a27]"></div>
            <div v-if="!appStore.isMobile" class="dock-tooltip">
              {{ item.title }}
            </div>
          </button>
        </div>
      </template>

      <template v-if="systemDockItems.length > 0">
        <div class="dock-divider"></div>
        <nav class="flex shrink-0 items-center gap-1 px-0.5" aria-label="系统入口">
          <button
            v-for="item in systemDockItems"
            :key="item.index"
            type="button"
            class="group dock-item-base h-11 w-11 rounded-lg md:h-10 md:w-10"
            :class="isMenuItemActive(item) ? activeDockItemClass : 'dock-item-idle'"
            :aria-current="isMenuItemActive(item) ? 'page' : undefined"
            :aria-label="item.title"
            :title="item.title"
            @click="handleSelect(item)"
            @contextmenu.prevent.stop="openDockContextMenu($event, item)"
          >
            <div
              :class="[
                item.icon,
                'text-[19px]',
                isMenuItemActive(item) ? activeDockIconClass : 'text-muted group-hover:text-default',
              ]"
            ></div>
            <div v-if="!appStore.isMobile" class="dock-tooltip">{{ item.title }}</div>
          </button>
        </nav>
      </template>
    </div>

    <ContextMenu
      v-model:visible="dockContextMenuVisible"
      class="pointer-events-auto"
      :items="dockContextMenuItems"
      :position="dockContextMenuPosition"
      @select="handleDockContextSelect"
    />
  </aside>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import ContextMenu from "../../components/common/ContextMenu.vue";
import type {
  ContextMenuItem,
  ContextMenuPosition,
} from "../../components/common/contextMenuTypes";
import { useAppStore } from "../../stores/app";
import {
  primaryPluginRoute,
  usePluginAppsStore,
} from "../../stores/pluginApps";
import { useWorkspaceStore } from "../../stores/workspace";
import { useHaptics } from "../../composables/useHaptics";
import { useSystemSettings } from "../../composables/useSystemSettings";
import { workspaceAppMap } from "../../workspace/apps";
import type { WorkspaceAppId } from "../../types/workspace";

type DockMenuItem = {
  appId: WorkspaceAppId;
  index: string;
  title: string;
  icon: string;
  pluginId?: string;
  routePath?: string;
};

const route = useRoute();
const router = useRouter();
const appStore = useAppStore();
const pluginApps = usePluginAppsStore();
const workspace = useWorkspaceStore();
const haptics = useHaptics();
const { systemName, logoUrl } = useSystemSettings();
const dockContextMenuVisible = ref(false);
const dockContextMenuPosition = ref<ContextMenuPosition>({ x: 0, y: 0 });
const dockContextItem = ref<DockMenuItem | null>(null);
const dockScrollRef = ref<HTMLElement | null>(null);
const activeDockItemClass = "dock-item-active !bg-slate-100 !text-slate-900 !ring-slate-900/10 dark:!bg-white/12 dark:!text-white dark:!ring-white/12";
const activeDockIconClass = "!text-slate-900 opacity-100 dark:!text-white";
const activeDockDotClass = "absolute -bottom-1 left-1/2 h-1 w-1 -translate-x-1/2 rounded-full bg-slate-900/70 dark:bg-white/80";

const overviewDockItem: DockMenuItem = {
  appId: "overview",
  index: "overview",
  title: "系统概览",
  icon: "i-ep-data-analysis",
};

const desktopAppIds = [
  "tasks",
  "variables",
  "files",
  "environments",
  "share",
  "extensions",
  "git",
  "telegram",
  "terminal",
  "settings",
] as const;

const mobileAppIds = ["tasks", "variables", "files", "share", "more"] as const;

const toDockItems = (ids: readonly string[]): DockMenuItem[] =>
  ids.flatMap((id) => {
    const app = workspaceAppMap.get(id as WorkspaceAppId);
    return app?.routeName
      ? [{ appId: app.id, index: app.routeName, title: app.title, icon: app.icon }]
      : [];
  });

void pluginApps.loadApps().catch(() => {});

const pluginDockItems = computed<DockMenuItem[]>(() =>
  (appStore.isMobile ? pluginApps.mobileApps : pluginApps.menuApps)
    .map((app) => {
      const route = primaryPluginRoute(app);
      return {
        appId: `plugin:${app.plugin_id}` as WorkspaceAppId,
        index: `plugin:${app.plugin_id}`,
        title: route?.title ?? app.name,
        icon: route?.icon ?? "i-ep-box",
        pluginId: app.plugin_id,
        routePath: route?.path ?? `/plugins/${app.plugin_id}`,
      };
    }),
);

const desktopMenuItems = computed(() => {
  const base = toDockItems(desktopAppIds);
  const settingsIndex = base.findIndex((item) => item.appId === "settings");
  if (settingsIndex === -1) return [...base, ...pluginDockItems.value];
  return [
    ...base.slice(0, settingsIndex),
    ...pluginDockItems.value,
    ...base.slice(settingsIndex),
  ];
});

const mobileMenuItems = computed(() => {
  const base = toDockItems(mobileAppIds);
  const moreIndex = base.findIndex((item) => item.appId === "more");
  if (moreIndex === -1) return [...base, ...pluginDockItems.value];
  return [
    ...base.slice(0, moreIndex),
    ...pluginDockItems.value,
    ...base.slice(moreIndex),
  ];
});

const mobilePrimaryRoutes = computed(
  () => new Set(["overview", "plugin-app", ...mobileMenuItems.value.map((item) => item.index)]),
);

const visibleMenuItems = computed(() =>
  appStore.isMobile ? mobileMenuItems.value : desktopMenuItems.value,
);

const primaryDockItems = computed(() =>
  appStore.isMobile
    ? visibleMenuItems.value
    : visibleMenuItems.value.filter((item) => item.appId !== "settings"),
);

const systemDockItems = computed(() =>
  appStore.isMobile
    ? []
    : visibleMenuItems.value.filter((item) => item.appId === "settings"),
);

const visibleWindowItems = computed(() => {
  if (appStore.isMobile) return [];
  return workspace.windows.filter(
    (item) => !item.documentKey.startsWith("task-editor:create"),
  );
});

const dockContextMenuItems = computed<ContextMenuItem[]>(() => {
  const item = dockContextItem.value;
  if (!item) return [];

  const app = workspaceAppMap.get(item.appId);
  const appWindowCount = workspace.windows.filter(
    (windowItem) => windowItem.appId === item.appId,
  ).length;
  const items: ContextMenuItem[] = [
    {
      label: "打开",
      action: "open",
      icon: item.icon,
    },
  ];

  if (app?.launchPolicy === "multi") {
    items.push({
      label: "打开新窗口",
      action: "open_new",
      icon: "i-ep-copy-document",
    });
  }

  if (appWindowCount > 0) {
    items.push(
      { type: "divider" },
      {
        label: `最小化 ${appWindowCount} 个窗口`,
        action: "minimize_app",
        icon: "i-ep-minus",
      },
      {
        label: `关闭 ${appWindowCount} 个窗口`,
        action: "close_app",
        icon: "i-ep-close",
        class: "text-rose-600 dark:text-rose-400",
      },
    );
  }

  return items;
});

const isOverviewActive = computed(() =>
  appStore.isMobile
    ? route.name === "overview"
    : workspace.activeWindow?.appId === "overview",
);

const isMenuItemActive = (item: DockMenuItem) => {
  if (!appStore.isMobile) {
    return workspace.activeWindow?.appId === item.appId;
  }

  if (item.pluginId) {
    return route.name === "plugin-app" && route.params.pluginId === item.pluginId;
  }

  const routeName = route.name as string | undefined;

  if (!routeName) return false;
  if (item.index === routeName) return true;

  return item.index === "more" && !mobilePrimaryRoutes.value.has(routeName);
};

const handleOverviewSelect = async () => {
  await haptics.selectionChanged();
  if (!appStore.isMobile) {
    workspace.openAppWindow("overview");
    return;
  }

  router.push({ name: "overview" });
  appStore.showDrawerSidebar = false;
};

const handleSelect = async (item: DockMenuItem) => {
  await haptics.selectionChanged();

  if (!appStore.isMobile) {
    if (item.pluginId) {
      const app = pluginApps.getApp(item.pluginId);
      if (app) workspace.openPluginAppWindow(app);
      return;
    }

    workspace.openAppWindow(item.appId);
    return;
  }

  if (item.pluginId) {
    router.push(item.routePath ?? `/plugins/${item.pluginId}`);
  } else {
    router.push({ name: item.index });
  }
  appStore.showDrawerSidebar = false;
};

const handleWindowItemSelect = async (id: string) => {
  await haptics.selectionChanged();
  workspace.focusWindow(id);
};

const clampDockMenuPosition = (event: MouseEvent): ContextMenuPosition => ({
  x: Math.max(8, Math.min(event.clientX, window.innerWidth - 224)),
  y: Math.max(8, Math.min(event.clientY, window.innerHeight - 132)),
});

const openDockContextMenu = (event: MouseEvent, item: DockMenuItem) => {
  if (appStore.isMobile) return;

  dockContextItem.value = item;
  dockContextMenuPosition.value = clampDockMenuPosition(event);
  dockContextMenuVisible.value = true;
};

const handleDockContextSelect = async (action: string) => {
  const item = dockContextItem.value;
  if (!item) return;

  if (action === "open_new") {
    await haptics.selectionChanged();
    workspace.openAppWindow(item.appId, { forceNew: true });
    return;
  }

  if (action === "minimize_app") {
    await haptics.selectionChanged();
    workspace.minimizeWindowsByApp(item.appId);
    return;
  }

  if (action === "close_app") {
    await haptics.selectionChanged();
    workspace.closeWindowsByApp(item.appId);
    return;
  }

  if (item.appId === "overview") {
    await handleOverviewSelect();
    return;
  }

  await handleSelect(item);
};

const scrollActiveDockItemIntoView = async () => {
  if (!appStore.isMobile) return;
  await nextTick();
  const activeItem = dockScrollRef.value?.querySelector<HTMLElement>(
    '[aria-current="page"]',
  );
  activeItem?.scrollIntoView({ block: "nearest", inline: "center" });
};

watch(
  () => [
    route.fullPath,
    appStore.isMobile,
    primaryDockItems.value.map((item) => item.index).join(","),
  ],
  scrollActiveDockItemIntoView,
  { immediate: true },
);
</script>

<style scoped>
.dock-mobile-scroll {
  justify-content: safe center;
}
</style>
