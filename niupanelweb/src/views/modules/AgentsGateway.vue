<template>
  <PluginHostView
    v-if="agentApp"
    :plugin-id="agentApp.plugin_id"
    :route-path="pluginRoutePath"
    :route-query="pluginRouteQuery"
    @navigate="handlePluginNavigate"
  />

  <div
    v-else-if="pluginApps.loading && !pluginApps.loaded"
    class="h-full min-h-[260px] flex-center bg-base text-sm font-medium text-muted"
  >
    加载 Agents 插件应用...
  </div>

  <section
    v-else
    class="h-full min-h-0 overflow-auto bg-base p-5 text-default"
  >
    <div class="mx-auto flex max-w-4xl flex-col gap-4">
      <header class="flex flex-wrap items-center justify-between gap-3 border-b border-light pb-4">
        <div>
          <h1 class="text-lg font-bold">Agents</h1>
          <p class="mt-1 text-xs text-muted">未安装可用的 Agents 插件应用</p>
        </div>
        <button
          type="button"
          class="rounded-lg bg-primary px-3 py-2 text-xs font-bold text-white transition-opacity hover:opacity-90"
          @click="openPluginSettings"
        >
          打开插件管理
        </button>
      </header>

      <div class="rounded-lg border border-light bg-card p-4">
        <div class="text-sm font-bold text-default">需要安装 Agents 插件</div>
        <div class="mt-2 text-xs leading-5 text-muted">
          当前公开构建不包含 legacy Agents 页面。请通过插件市场、上传包或路径安装启用一个声明
          agents.* 能力的原生 Vue 插件应用。
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  primaryPluginAppForCapability,
  primaryPluginRoute,
  usePluginAppsStore,
} from "@/stores/pluginApps";
import { normalizePluginRoute } from "@/utils/pluginRoutes";
import PluginHostView from "@/views/plugins/PluginHostView.vue";

const route = useRoute();
const router = useRouter();
const pluginApps = usePluginAppsStore();
const pluginRoutePath = ref("");
const pluginRouteQuery = ref<Record<string, unknown>>({});
const mountedPluginId = ref<string | null>(null);

const isPluginAgentsRoute = computed(() => route.name === "plugin-agents");

const agentApp = computed(() =>
  primaryPluginAppForCapability(pluginApps.apps, "agents"),
);

const aliasRoutePath = computed(() => {
  const match = route.params.pathMatch;
  if (Array.isArray(match)) return match.join("/");
  return typeof match === "string" ? match : "";
});

const aliasRouteQuery = computed<Record<string, unknown>>(() => ({ ...route.query }));

const resetPluginRoute = () => {
  const app = agentApp.value;
  if (!app) {
    mountedPluginId.value = null;
    pluginRoutePath.value = "";
    pluginRouteQuery.value = {};
    return;
  }

  const primaryRoute = primaryPluginRoute(app);
  const normalized = isPluginAgentsRoute.value
    ? {
        routePath: aliasRoutePath.value,
        routeQuery: aliasRouteQuery.value,
      }
    : normalizePluginRoute(app.plugin_id, primaryRoute?.path);
  mountedPluginId.value = app.plugin_id;
  pluginRoutePath.value = normalized.routePath;
  pluginRouteQuery.value = normalized.routeQuery;
};

const handlePluginNavigate = async (path: string) => {
  const app = agentApp.value;
  if (!app) return;

  const normalized = normalizePluginRoute(app.plugin_id, path);
  pluginRoutePath.value = normalized.routePath;
  pluginRouteQuery.value = normalized.routeQuery;

  if (isPluginAgentsRoute.value) {
    const aliasPath = normalized.routePath
      ? `/plugins/agents/${normalized.routePath}`
      : "/plugins/agents";
    await router.push({
      path: aliasPath,
      query: normalized.routeQuery as Record<string, string>,
    });
  }
};

const openPluginSettings = async () => {
  await router.push({ name: "settings", query: { tab: "plugins" } });
};

onMounted(async () => {
  await pluginApps.loadApps().catch(() => undefined);
  resetPluginRoute();
});

watch(
  () => [
    agentApp.value?.plugin_id ?? null,
    isPluginAgentsRoute.value,
    aliasRoutePath.value,
    JSON.stringify(aliasRouteQuery.value),
  ],
  ([pluginId]) => {
    if (pluginId !== mountedPluginId.value) resetPluginRoute();
    if (isPluginAgentsRoute.value) resetPluginRoute();
  },
);
</script>
