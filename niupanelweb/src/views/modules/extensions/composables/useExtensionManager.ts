import { computed, onMounted, reactive, ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { useRouter } from "vue-router";
import {
  createUploadFormData,
  type UploadFormEntry,
} from "@/api/upload";
import {
  checkPluginMarketUpdates,
  disablePlugin,
  enablePlugin,
  getPluginMarket,
  installMarketPlugin,
  installPlugin,
  listPluginHealth,
  listPluginMarketSources,
  listPlugins,
  listPluginVersions,
  previewInstallPlugin,
  previewMarketPlugin,
  previewUpdatePlugin,
  previewUploadInstallPlugin,
  previewUploadUpdatePlugin,
  rollbackPlugin,
  uninstallPlugin,
  updatePlugin,
  updatePluginMarketSources,
  uploadInstallPlugin,
  uploadUpdatePlugin,
} from "@/api/plugins";
import { useAppStore } from "@/stores/app";
import { primaryPluginRoute, usePluginAppsStore } from "@/stores/pluginApps";
import { usePluginThemesStore } from "@/stores/pluginThemes";
import { useWorkspaceStore } from "@/stores/workspace";
import type {
  PluginHealthReport,
  PluginImpactPreview,
  PluginMarketEntry,
  PluginMarketIndex,
  PluginMarketSource,
  PluginMarketUpdateRecord,
  PluginRecord,
  PluginStatus,
  PluginThemeRecord,
  PluginVersionRecord,
} from "@/types";

type ManagedPlugin = { record: PluginRecord };

export function useExtensionManager() {
  const views = [
    { id: "installed" as const, label: "已安装" },
    { id: "themes" as const, label: "外观" },
    { id: "market" as const, label: "插件市场" },
  ];
  const installMethodOptions = [
    { label: "上传包", value: "upload" },
    { label: "服务端路径", value: "path" },
  ];

  const router = useRouter();
  const appStore = useAppStore();
  const workspace = useWorkspaceStore();
  const pluginApps = usePluginAppsStore();
  const pluginThemes = usePluginThemesStore();
  const activeView = ref<"installed" | "themes" | "market">("installed");
  const searchQuery = ref("");
  const statusFilter = ref<"all" | PluginStatus>("all");
  const loading = ref(false);
  const busyPlugin = ref("");
  const installedPluginRecords = ref<PluginRecord[]>([]);
  const pluginHealth = ref<PluginHealthReport[]>([]);
  const marketSourcesDialogVisible = ref(false);

  const market = reactive({
    sources: [] as PluginMarketSource[],
    selectedUrl: "",
    draftName: "",
    draftUrl: "",
    sourcesSaving: false,
    loading: false,
    updatesLoading: false,
    index: null as PluginMarketIndex | null,
    updates: [] as PluginMarketUpdateRecord[],
  });

  const installDialog = reactive({
    visible: false,
    submitting: false,
    operation: "install" as "install" | "update",
    method: "upload" as "upload" | "path",
    pluginId: "",
    pluginName: "",
    sourcePath: "",
    file: null as File | null,
    enable: true,
    checksumSha256: "",
  });

  const historyDialog = reactive({
    visible: false,
    loading: false,
    pluginId: "",
    pluginName: "",
    versions: [] as PluginVersionRecord[],
  });

  const impactDialog = reactive({
    visible: false,
    preview: null as PluginImpactPreview | null,
    resolve: null as ((confirmed: boolean) => void) | null,
  });

  const allPlugins = computed<ManagedPlugin[]>(() =>
    installedPluginRecords.value.map((record) => ({ record })),
  );
  const capabilityLabel = (capability: string) => capability.split(".").slice(0, 2).join(".");
  const visibleCapabilities = (item: ManagedPlugin) =>
    Array.from(new Set(item.record.manifest.capabilities)).slice(0, 3);
  const normalizedSearch = computed(() => searchQuery.value.trim().toLowerCase());
  const visiblePlugins = computed(() =>
    allPlugins.value.filter((item) => {
      const searchable = [
        item.record.manifest.name,
        item.record.manifest.id,
        item.record.manifest.description,
        item.record.manifest.runtime,
        ...item.record.manifest.capabilities,
      ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
      return (
        (statusFilter.value === "all" || item.record.status === statusFilter.value) &&
        (!normalizedSearch.value || searchable.includes(normalizedSearch.value))
      );
    }),
  );
  const marketVisiblePlugins = computed(() =>
    (market.index?.plugins ?? []).filter((entry) => {
      const searchable = [entry.name, entry.id, entry.description]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
      return !normalizedSearch.value || searchable.includes(normalizedSearch.value);
    }),
  );
  const marketVisibleUpdates = computed(() =>
    market.updates.filter((item) => {
      const searchable = [item.entry.name, item.plugin_id, item.source_name]
        .join(" ")
        .toLowerCase();
      return !normalizedSearch.value || searchable.includes(normalizedSearch.value);
    }),
  );
  const enabledCount = computed(() => allPlugins.value.filter((item) => item.record.enabled).length);
  const appCount = computed(() => allPlugins.value.filter((item) => item.record.manifest.ui?.enabled).length);
  const themeCount = computed(() => allPlugins.value.filter((item) => item.record.manifest.theme?.enabled).length);
  const themeSwatches = (theme: PluginThemeRecord) => {
    const palette = appStore.isDark ? theme.theme.dark : theme.theme.light;
    return [
      palette.primary ?? "#2563EB",
      palette.bg_base ?? "#F3F5F7",
      palette.bg_card ?? "#FFFFFF",
      palette.text_default ?? "#172033",
    ];
  };

  const healthByPlugin = computed(() => {
    const values = new Map<string, PluginHealthReport>();
    for (const report of pluginHealth.value) values.set(report.plugin_id, report);
    return values;
  });
  const pluginHealthReport = (item: ManagedPlugin) =>
    healthByPlugin.value.get(item.record.manifest.id);
  const pluginIcon = (item: ManagedPlugin) =>
    item.record.manifest.ui?.routes?.[0]?.icon ?? marketIcon();
  const marketIcon = () => "i-carbon-application-web";
  const marketEntryIsSigned = (entry: PluginMarketEntry) =>
    entry.assets.some((asset) => Boolean(asset.signature_ed25519));
  const healthText = (report?: PluginHealthReport) => {
    if (!report) return "未知";
    if (!report.healthy) return "异常";
    return report.checks.some((check) => check.severity === "warning") ? "警告" : "健康";
  };
  const healthTone = (report?: PluginHealthReport) => {
    if (!report) return "text-muted";
    if (!report.healthy) return "text-rose-600 dark:text-rose-300";
    return report.checks.some((check) => check.severity === "warning")
      ? "text-amber-600 dark:text-amber-300"
      : "text-emerald-600 dark:text-emerald-300";
  };
  const formatTime = (value: string) => {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
  };

  const loadPlugins = async () => {
    loading.value = true;
    try {
      const [plugins, health] = await Promise.all([
        listPlugins(),
        listPluginHealth(),
        pluginApps.loadApps({ force: true }),
        pluginThemes.loadThemes(true),
      ]);
      installedPluginRecords.value = plugins.data;
      pluginHealth.value = health.data;
    } finally {
      loading.value = false;
    }
  };

  const loadMarketSources = async () => {
    const response = await listPluginMarketSources();
    market.sources = response.data;
    if (!market.sources.some((source) => source.url === market.selectedUrl)) {
      market.selectedUrl = market.sources.find((source) => source.enabled)?.url ?? market.sources[0]?.url ?? "";
    }
  };

  const loadAll = async () => {
    await Promise.all([loadPlugins(), loadMarketSources()]);
  };

  const addMarketSource = () => {
    const url = market.draftUrl.trim();
    if (!url) {
      ElMessage.warning("请输入发布源地址");
      return;
    }
    if (market.sources.some((source) => source.url === url)) {
      ElMessage.warning("该发布源已存在");
      return;
    }
    market.sources.push({ name: market.draftName.trim(), url, enabled: true });
    market.draftName = "";
    market.draftUrl = "";
  };
  const removeMarketSource = (source: PluginMarketSource) => {
    market.sources = market.sources.filter((item) => item.url !== source.url);
  };
  const saveMarketSources = async () => {
    market.sourcesSaving = true;
    try {
      const response = await updatePluginMarketSources({ sources: market.sources });
      market.sources = response.data;
      marketSourcesDialogVisible.value = false;
      await loadMarketSources();
      ElMessage.success("发布源已保存");
    } finally {
      market.sourcesSaving = false;
    }
  };
  const loadMarket = async () => {
    if (!market.selectedUrl) {
      ElMessage.warning("请先添加并选择发布源");
      return;
    }
    market.loading = true;
    try {
      market.index = (await getPluginMarket(market.selectedUrl)).data;
    } finally {
      market.loading = false;
    }
  };
  const checkMarketUpdates = async () => {
    market.updatesLoading = true;
    try {
      market.updates = (await checkPluginMarketUpdates()).data;
      if (!market.updates.length) ElMessage.success("当前没有可更新扩展");
    } finally {
      market.updatesLoading = false;
    }
  };

  const installedVersion = (entry: PluginMarketEntry) => {
    return allPlugins.value.find(
      (item) => item.record.manifest.id === entry.id,
    )?.record.manifest.version ?? "";
  };

  const resolveImpactPreview = (confirmed: boolean) => {
    const resolve = impactDialog.resolve;
    impactDialog.resolve = null;
    impactDialog.visible = false;
    const allowed = confirmed && Boolean(impactDialog.preview?.install_allowed);
    impactDialog.preview = null;
    resolve?.(allowed);
  };
  const confirmPreview = (preview: PluginImpactPreview) => {
    impactDialog.resolve?.(false);
    impactDialog.preview = preview;
    impactDialog.visible = true;
    return new Promise<boolean>((resolve) => {
      impactDialog.resolve = resolve;
    });
  };

  const openInstall = (method: "upload" | "path") => {
    Object.assign(installDialog, {
      visible: true,
      submitting: false,
      operation: "install",
      method,
      pluginId: "",
      pluginName: "",
      sourcePath: "",
      file: null,
      enable: true,
      checksumSha256: "",
    });
  };
  const openUpdate = (item: ManagedPlugin, method: "upload" | "path") => {
    Object.assign(installDialog, {
      visible: true,
      submitting: false,
      operation: "update",
      method,
      pluginId: item.record.manifest.id,
      pluginName: item.record.manifest.name,
      sourcePath: "",
      file: null,
      enable: item.record.enabled,
      checksumSha256: "",
    });
  };
  const handleInstallFile = (event: Event) => {
    installDialog.file = (event.target as HTMLInputElement).files?.[0] ?? null;
  };
  const uploadForm = () => {
    const entries: UploadFormEntry[] = [
      ["enable", installDialog.enable ? "true" : "false"],
    ];
    if (installDialog.file) entries.unshift(["file", installDialog.file]);
    if (installDialog.checksumSha256.trim()) {
      entries.push(["checksum_sha256", installDialog.checksumSha256.trim()]);
    }
    return createUploadFormData(entries);
  };
  const submitInstallDialog = async () => {
    if (installDialog.method === "path" && !installDialog.sourcePath.trim()) {
      ElMessage.warning("请输入服务端目录");
      return;
    }
    if (installDialog.method === "upload" && !installDialog.file) {
      ElMessage.warning("请选择扩展安装包");
      return;
    }
    installDialog.submitting = true;
    try {
      if (installDialog.operation === "install") {
        if (installDialog.method === "path") {
          const payload = { source_path: installDialog.sourcePath.trim(), enable: installDialog.enable };
          const preview = await previewInstallPlugin(payload);
          if (!(await confirmPreview(preview.data))) return;
          await installPlugin(payload);
        } else {
          const preview = await previewUploadInstallPlugin(uploadForm());
          if (!(await confirmPreview(preview.data))) return;
          await uploadInstallPlugin(uploadForm());
        }
      } else if (installDialog.method === "path") {
        const payload = { source_path: installDialog.sourcePath.trim() };
        const preview = await previewUpdatePlugin(installDialog.pluginId, payload);
        if (!(await confirmPreview(preview.data))) return;
        await updatePlugin(installDialog.pluginId, payload);
      } else {
        const preview = await previewUploadUpdatePlugin(installDialog.pluginId, uploadForm());
        if (!(await confirmPreview(preview.data))) return;
        await uploadUpdatePlugin(installDialog.pluginId, uploadForm());
      }
      installDialog.visible = false;
      ElMessage.success(installDialog.operation === "install" ? "扩展已安装" : "扩展已更新");
      await loadPlugins();
    } finally {
      installDialog.submitting = false;
    }
  };

  const togglePlugin = async (item: ManagedPlugin) => {
    busyPlugin.value = item.record.manifest.id;
    try {
      if (item.record.enabled) await disablePlugin(item.record.manifest.id);
      else await enablePlugin(item.record.manifest.id);
      await loadPlugins();
    } finally {
      busyPlugin.value = "";
    }
  };
  const removePlugin = async (item: ManagedPlugin) => {
    await ElMessageBox.confirm(`确认卸载 ${item.record.manifest.name}？`, "卸载扩展", {
      type: "warning",
      confirmButtonText: "卸载",
      cancelButtonText: "取消",
    });
    await uninstallPlugin(item.record.manifest.id);
    ElMessage.success("扩展已卸载");
    await loadPlugins();
  };
  const handlePluginCommand = (command: string, item: ManagedPlugin) => {
    if (command === "path-update") openUpdate(item, "path");
    if (command === "upload-update") openUpdate(item, "upload");
    if (command === "history") void openHistory(item);
    if (command === "uninstall") void removePlugin(item);
  };
  const openHistory = async (item: ManagedPlugin) => {
    Object.assign(historyDialog, {
      visible: true,
      loading: true,
      pluginId: item.record.manifest.id,
      pluginName: item.record.manifest.name,
      versions: [],
    });
    try {
      historyDialog.versions = (await listPluginVersions(item.record.manifest.id)).data;
    } finally {
      historyDialog.loading = false;
    }
  };
  const rollbackVersion = async (version: PluginVersionRecord) => {
    await ElMessageBox.confirm(`确认回滚到 v${version.version}？`, "回滚扩展", {
      type: "warning",
      confirmButtonText: "回滚",
      cancelButtonText: "取消",
    });
    await rollbackPlugin(historyDialog.pluginId, version.id);
    ElMessage.success("扩展已回滚");
    historyDialog.visible = false;
    await loadPlugins();
  };

  const installFromMarket = async (entry: PluginMarketEntry, sourceUrl = market.selectedUrl) => {
    if (!sourceUrl) {
      ElMessage.warning("请选择发布源");
      return;
    }
    const payload = { index_url: sourceUrl, plugin_id: entry.id, enable: true };
    const preview = await previewMarketPlugin(payload);
    if (!(await confirmPreview(preview.data))) return;
    const updating = Boolean(installedVersion(entry));
    await installMarketPlugin(payload);
    ElMessage.success(updating ? "扩展已更新" : "扩展已安装");
    await loadPlugins();
    if (market.updates.length) await checkMarketUpdates();
  };

  const openPluginApp = async (item: ManagedPlugin) => {
    await pluginApps.loadApps({ force: true });
    const app = pluginApps.getApp(item.record.manifest.id);
    if (!app) {
      ElMessage.warning("该扩展没有可用的应用入口");
      return;
    }
    if (appStore.isMobile) await router.push(primaryPluginRoute(app)?.path ?? `/plugins/${app.plugin_id}`);
    else workspace.openPluginAppWindow(app);
  };


  onMounted(loadAll);

  return {
    views, installMethodOptions, router, appStore, workspace, pluginApps, pluginThemes, activeView,
    searchQuery, statusFilter, loading, busyPlugin, installedPluginRecords, pluginHealth, marketSourcesDialogVisible, market,
    installDialog, historyDialog, impactDialog, allPlugins, capabilityLabel, visibleCapabilities, normalizedSearch, visiblePlugins, marketVisiblePlugins,
    marketVisibleUpdates, enabledCount, appCount, themeCount, themeSwatches, healthByPlugin, pluginHealthReport, pluginIcon,
    marketIcon, marketEntryIsSigned, healthText, healthTone, formatTime, loadPlugins, loadMarketSources, loadAll, addMarketSource,
    removeMarketSource, saveMarketSources, loadMarket, checkMarketUpdates, installedVersion, confirmPreview, resolveImpactPreview, openInstall,
    openUpdate, handleInstallFile, uploadForm, submitInstallDialog, togglePlugin, removePlugin, handlePluginCommand, openHistory,
    rollbackVersion, installFromMarket, openPluginApp,
  };
}
