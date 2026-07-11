<template>
  <div class="plugin-host h-full min-h-0 bg-base">
    <div
      v-if="loading"
      class="h-full min-h-[260px] flex-center text-sm font-medium text-muted"
    >
      加载插件应用...
    </div>

    <div
      v-else-if="error"
      class="h-full min-h-[260px] flex flex-col items-center justify-center gap-3 px-6 text-center"
    >
      <div class="i-ep-warning text-2xl text-amber-500"></div>
      <div class="text-sm font-bold text-default">插件应用加载失败</div>
      <div class="max-w-[520px] text-xs leading-5 text-muted">{{ error }}</div>
      <button
        type="button"
        class="rounded-lg bg-primary px-3 py-2 text-xs font-bold text-white transition-opacity hover:opacity-90"
        @click="mountPlugin"
      >
        重试
      </button>
    </div>

    <div
      v-show="!loading && !error"
      ref="containerRef"
      class="h-full min-h-0"
    ></div>
  </div>
</template>

<script setup lang="ts">
import { ElMessage, ElMessageBox } from "element-plus";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import request from "@/utils/request";
import { resolvePluginAssetUrl } from "@/api/plugins";
import { hasPluginAppPermissions, usePluginAppsStore } from "@/stores/pluginApps";
import { hasPermission } from "@/utils/permission";
import type {
  NiuPanelPluginApp,
  NiuPanelPluginApiRequest,
  NiuPanelPluginContext,
  NiuPanelPluginModule,
  NiuPanelPluginRouteListener,
  NiuPanelPluginRouteSnapshot,
} from "@niupanel/plugin-sdk";

const props = defineProps<{
  pluginId?: string;
  routePath?: string;
  routeQuery?: Record<string, unknown>;
}>();

const emit = defineEmits<{
  navigate: [path: string];
}>();

const route = useRoute();
const router = useRouter();
const pluginApps = usePluginAppsStore();
const containerRef = ref<HTMLElement | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

let mountedInstance: unknown = null;
let mountedModule: NiuPanelPluginModule | null = null;
let mountedContext: NiuPanelPluginContext | null = null;
let mountedPluginId: string | null = null;
let mountedEntrySignature: string | null = null;
let routeListeners = new Set<NiuPanelPluginRouteListener>();

const pluginId = computed(() => {
  const routePluginId = route.params.pluginId;
  return props.pluginId ?? (Array.isArray(routePluginId) ? routePluginId[0] : routePluginId);
});

const pluginRoutePath = computed(() => {
  if (props.routePath !== undefined) return props.routePath;
  const match = route.params.pathMatch;
  if (Array.isArray(match)) return match.join("/");
  return typeof match === "string" ? match : "";
});

const pluginEntrySignature = computed(() => {
  const id = pluginId.value;
  if (!id) return null;
  const app = pluginApps.getApp(id);
  if (!app) return null;
  return `${app.plugin_id}:${app.version}:${app.ui.entry_url}`;
});

const currentPluginRouteSnapshot = (): NiuPanelPluginRouteSnapshot => ({
  path: pluginRoutePath.value,
  query: props.routeQuery ? { ...props.routeQuery } : { ...route.query },
});

const normalizeApiPath = (path: string) =>
  path.startsWith("/api/v1/") ? path.slice("/api/v1".length) : path;

const canonicalApiPath = (path: string) => {
  const normalized = normalizeApiPath(path);
  return normalized.startsWith("/") ? normalized : `/${normalized}`;
};

const methodFor = (method?: string) => (method ?? "GET").toUpperCase();

const manifestDeclaresPermission = (
  declaredPermissions: string[],
  requiredPermission: string,
) => declaredPermissions.includes(requiredPermission);

const permissionForApiRequest = (method: string, path: string) => {
  const [pathname] = path.split("?");
  const segments = pathname.split("/").filter(Boolean);
  const first = segments[0];

  if (pathname === "/plugins/apps") return null;
  if (first === "plugins") {
    return undefined;
  }
  if (first === "compiler") {
    if (method === "GET") return "compiler:read";
    return "compiler:run";
  }
  if (first === "tasks") {
    if (method === "GET") return segments.length <= 1 ? "task:list" : "task:read";
    if (method === "POST") {
      if (["run"].includes(segments[1])) return "task:run";
      if (["stop", "pause", "resume"].includes(segments[1])) return "task:stop";
      if (["enable", "disable", "pin", "unpin", "settings"].includes(segments[1])) return "task:update";
      return "task:create";
    }
    if (method === "DELETE") return "task:delete";
    return "task:update";
  }
  if (first === "variables") {
    if (method === "GET") return "var:list";
    if (method === "POST") {
      if (["toggle", "reorder"].includes(segments[1])) return "var:update";
      return "var:create";
    }
    if (method === "DELETE") return "var:delete";
    return "var:update";
  }
  if (first === "environments") {
    if (method === "GET") return "env:read";
    if (method === "POST") {
      if (["python", "node"].includes(segments[1]) && segments.length === 2) return "env:create";
      return "env:update";
    }
    if (method === "DELETE") return "env:delete";
    return "env:update";
  }
  if (first === "share") {
    if (method === "GET") return "share:list";
    if (method === "POST") return "share:create";
    if (method === "DELETE") return "share:delete";
    return "share:update";
  }
  if (first === "jobs") {
    if (method === "GET") return segments.length <= 1 ? "job:list" : "job:read";
    return "job:*";
  }
  if (first === "overview" && method === "GET") return "overview:read";

  return undefined;
};

const apiMethodMatches = (methods: string[] | undefined, method: string) =>
  !!methods?.length && methods.some((candidate) => {
    const normalized = candidate.trim().toUpperCase();
    return normalized === method;
  });

const apiPathMatches = (rule: string, path: string) => {
  const trimmed = rule.trim();
  if (trimmed.endsWith("/**")) {
    const prefix = trimmed.slice(0, -3);
    return path === prefix || path.startsWith(`${prefix}/`);
  }
  if (trimmed.endsWith("*")) {
    return path.startsWith(trimmed.slice(0, -1));
  }
  return path === trimmed;
};

const pluginApiAllows = (
  app: NiuPanelPluginApp,
  method: string,
  path: string,
) => {
  const allow = app.ui.api?.allow ?? [];
  if (allow.length === 0) return false;
  return allow.some((rule) => apiMethodMatches(rule.methods, method) && apiPathMatches(rule.path, path));
};

const ensurePluginApiPermission = (
  app: NiuPanelPluginApp,
  options: NiuPanelPluginApiRequest,
) => {
  const path = canonicalApiPath(options.path);
  const method = methodFor(options.method);
  if (!pluginApiAllows(app, method, path)) {
    throw new Error(`插件 manifest api.allow 未允许访问：${method} ${path}`);
  }
  const permission = permissionForApiRequest(method, path);

  if (permission === undefined) {
    throw new Error(`插件 API 不允许访问未知路径：${path}`);
  }
  if (!permission) return;
  if (!manifestDeclaresPermission(app.ui.permissions, permission)) {
    throw new Error(`插件 manifest 未声明权限：${permission}`);
  }
  if (!hasPermission(permission)) {
    throw new Error(`当前用户缺少权限：${permission}`);
  }
};

const createContext = (app: NiuPanelPluginApp): NiuPanelPluginContext => ({
  pluginId: app.plugin_id,
  app,
  route: {
    ...currentPluginRouteSnapshot(),
    onChange(listener) {
      routeListeners.add(listener);
      return () => {
        routeListeners.delete(listener);
      };
    },
  },
  api: {
    async request<T = unknown>(options: NiuPanelPluginApiRequest): Promise<T> {
      ensurePluginApiPermission(app, options);
      const response = await request({
        method: "POST",
        url: `/plugins/${encodeURIComponent(app.plugin_id)}/api`,
        data: {
          method: options.method ?? "GET",
          path: canonicalApiPath(options.path),
          data: options.data,
          params: options.params,
        },
      });
      return (response as { data?: T })?.data ?? (response as T);
    },
    async invoke<T = unknown>(action: string, input?: unknown): Promise<T> {
      if (!app.capabilities.includes("agents.invoke")) {
        throw new Error("当前插件没有声明 agents.invoke 能力，无法使用 invoke");
      }
      const response = await request.post(`/plugins/${app.plugin_id}/invoke`, {
        action,
        input,
      });
      return (response as { data?: T })?.data ?? (response as T);
    },
  },
  ui: {
    toast(message, type = "info") {
      ElMessage({ message, type });
    },
    async confirm(message, title = "确认操作") {
      try {
        await ElMessageBox.confirm(message, title, {
          confirmButtonText: "确定",
          cancelButtonText: "取消",
          type: "warning",
        });
        return true;
      } catch {
        return false;
      }
    },
    async navigate(path: string) {
      if (props.routePath !== undefined) {
        emit("navigate", path);
        return;
      }
      await router.push(path);
    },
  },
});

const unmountPlugin = async () => {
  if (mountedModule?.unmount && mountedContext) {
    await mountedModule.unmount(mountedInstance, mountedContext);
  }
  mountedInstance = null;
  mountedModule = null;
  mountedContext = null;
  mountedPluginId = null;
  mountedEntrySignature = null;
  routeListeners = new Set<NiuPanelPluginRouteListener>();
  if (containerRef.value) containerRef.value.innerHTML = "";
};

const notifyRouteChange = () => {
  if (!mountedContext) return;
  const snapshot = currentPluginRouteSnapshot();
  mountedContext.route.path = snapshot.path;
  mountedContext.route.query = snapshot.query;
  routeListeners.forEach((listener) => {
    listener(snapshot);
  });
};

const mountPlugin = async () => {
  const id = pluginId.value;
  if (!id || !containerRef.value) return;

  loading.value = true;
  error.value = null;
  await unmountPlugin();

  try {
    await pluginApps.loadApps();
    const app = pluginApps.getApp(id);
    if (!app) throw new Error(`未找到插件应用：${id}`);
    if (!hasPluginAppPermissions(app)) {
      throw new Error("当前用户没有访问该插件应用的权限");
    }

    const entryUrl = resolvePluginAssetUrl(app.ui.entry_url);
    const versionedUrl = `${entryUrl}${entryUrl.includes("?") ? "&" : "?"}v=${encodeURIComponent(app.version)}`;
    const module = await import(/* @vite-ignore */ versionedUrl);
    const pluginModule = (module.default ?? module) as NiuPanelPluginModule;
    if (typeof pluginModule.mount !== "function") {
      throw new Error("插件 UI 入口必须导出 mount(el, context)");
    }

    mountedModule = pluginModule;
    mountedContext = createContext(app);
    mountedPluginId = app.plugin_id;
    mountedEntrySignature = `${app.plugin_id}:${app.version}:${app.ui.entry_url}`;
    mountedInstance = await pluginModule.mount(containerRef.value, mountedContext);
  } catch (err) {
    error.value = err instanceof Error ? err.message : "未知错误";
  } finally {
    loading.value = false;
  }
};

onMounted(async () => {
  await nextTick();
  await mountPlugin();
});

watch(
  () => pluginId.value,
  async () => {
    await nextTick();
    await mountPlugin();
  },
);

watch(
  () => pluginEntrySignature.value,
  async (signature) => {
    if (loading.value) return;
    if (mountedPluginId !== pluginId.value) return;
    if (signature === mountedEntrySignature) return;
    await nextTick();
    await mountPlugin();
  },
);

watch(
  () => [
    pluginRoutePath.value,
    JSON.stringify(props.routeQuery ?? route.query),
  ],
  () => {
    if (mountedPluginId === pluginId.value) {
      notifyRouteChange();
    }
  },
);

onBeforeUnmount(() => {
  void unmountPlugin();
});
</script>
