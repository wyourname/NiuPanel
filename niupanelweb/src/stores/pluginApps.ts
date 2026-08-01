import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { listPluginApps } from "@/api/plugins";
import type { PluginAppRecord } from "@/types";
import { hasPermission } from "@/utils/permission";

export const hasPluginAppPermissions = (app: PluginAppRecord) =>
  app.ui.permissions.every((permission) => hasPermission(permission));

export const visiblePluginRoutes = (app: PluginAppRecord) =>
  [...app.ui.routes]
    .filter((route) => !route.hidden)
    .sort((a, b) => (a.order ?? 0) - (b.order ?? 0));

export const primaryPluginRoute = (app: PluginAppRecord) =>
  visiblePluginRoutes(app)[0] ?? app.ui.routes[0] ?? null;

export const pluginAppsForCapability = (
  apps: PluginAppRecord[],
  capabilityNamespace: string,
) =>
  apps
    .filter(
      (item) =>
        item.ui.mode === "vue_app" &&
        item.capabilities.some((capability) =>
          capability.startsWith(`${capabilityNamespace}.`),
        ) &&
        hasPluginAppPermissions(item),
    )
    .sort((a, b) => {
      const categoryRank = (app: PluginAppRecord) =>
        app.ui.display.category === capabilityNamespace ? 0 : 1;
      const categoryOrder = categoryRank(a) - categoryRank(b);
      if (categoryOrder !== 0) return categoryOrder;

      const displayOrder = a.ui.display.order - b.ui.display.order;
      if (displayOrder !== 0) return displayOrder;

      const routeOrder =
        (primaryPluginRoute(a)?.order ?? 0) -
        (primaryPluginRoute(b)?.order ?? 0);
      if (routeOrder !== 0) return routeOrder;

      return a.name.localeCompare(b.name);
    });

export const primaryPluginAppForCapability = (
  apps: PluginAppRecord[],
  capabilityNamespace: string,
) => pluginAppsForCapability(apps, capabilityNamespace)[0] ?? null;

const sortPluginApps = (apps: PluginAppRecord[]) =>
  [...apps].sort((a, b) => {
    const displayOrder = a.ui.display.order - b.ui.display.order;
    if (displayOrder !== 0) return displayOrder;
    const routeOrder =
      (primaryPluginRoute(a)?.order ?? 0) -
      (primaryPluginRoute(b)?.order ?? 0);
    if (routeOrder !== 0) return routeOrder;
    return a.name.localeCompare(b.name);
  });

export const usePluginAppsStore = defineStore("pluginApps", () => {
  const apps = ref<PluginAppRecord[]>([]);
  const loaded = ref(false);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const loadApps = async (options: { force?: boolean } = {}) => {
    if (loading.value) return apps.value;
    if (loaded.value && !options.force) return apps.value;

    loading.value = true;
    error.value = null;
    try {
      const response = await listPluginApps();
      apps.value = response.data ?? [];
      loaded.value = true;
      return apps.value;
    } catch (err) {
      error.value = err instanceof Error ? err.message : "插件应用加载失败";
      throw err;
    } finally {
      loading.value = false;
    }
  };

  const getApp = (pluginId: string) =>
    apps.value.find((item) => item.plugin_id === pluginId) ?? null;

  const menuApps = computed(() =>
    sortPluginApps(
      apps.value.filter(
        (item) =>
          item.ui.mode === "vue_app" &&
          item.ui.display.sidebar &&
          visiblePluginRoutes(item).length > 0 &&
          hasPluginAppPermissions(item),
      ),
    ),
  );

  const workspaceApps = computed(() =>
    sortPluginApps(
      apps.value.filter(
        (item) =>
          item.ui.mode === "vue_app" &&
          item.ui.display.workspace &&
          visiblePluginRoutes(item).length > 0 &&
          hasPluginAppPermissions(item),
      ),
    ),
  );

  return {
    apps,
    error,
    getApp,
    loadApps,
    loaded,
    loading,
    menuApps,
    workspaceApps,
  };
});
