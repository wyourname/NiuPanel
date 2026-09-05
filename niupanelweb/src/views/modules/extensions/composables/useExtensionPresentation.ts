import { computed, type Ref } from "vue";
import type {
  PluginHealthReport,
  PluginMarketEntry,
  PluginMarketIndex,
  PluginMarketUpdateRecord,
  PluginRecord,
  PluginStatus,
  PluginThemeRecord,
} from "@/types";

export type ManagedPlugin = { record: PluginRecord };

type ExtensionPresentationOptions = {
  installedPluginRecords: Ref<PluginRecord[]>;
  pluginHealth: Ref<PluginHealthReport[]>;
  market: {
    index: PluginMarketIndex | null;
    updates: PluginMarketUpdateRecord[];
  };
  searchQuery: Ref<string>;
  statusFilter: Ref<"all" | PluginStatus>;
  isDark: () => boolean;
};

export function useExtensionPresentation(options: ExtensionPresentationOptions) {
  const allPlugins = computed<ManagedPlugin[]>(() =>
    options.installedPluginRecords.value.map((record) => ({ record })),
  );
  const capabilityLabel = (capability: string) =>
    capability.split(".").slice(0, 2).join(".");
  const visibleCapabilities = (item: ManagedPlugin) =>
    Array.from(new Set(item.record.manifest.capabilities)).slice(0, 3);
  const normalizedSearch = computed(() =>
    options.searchQuery.value.trim().toLowerCase(),
  );
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
        (options.statusFilter.value === "all" ||
          item.record.status === options.statusFilter.value) &&
        (!normalizedSearch.value || searchable.includes(normalizedSearch.value))
      );
    }),
  );
  const marketVisiblePlugins = computed(() =>
    (options.market.index?.plugins ?? []).filter((entry) => {
      const searchable = [entry.name, entry.id, entry.description]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
      return (
        !normalizedSearch.value || searchable.includes(normalizedSearch.value)
      );
    }),
  );
  const marketVisibleUpdates = computed(() =>
    options.market.updates.filter((item) => {
      const searchable = [item.entry.name, item.plugin_id, item.source_name]
        .join(" ")
        .toLowerCase();
      return (
        !normalizedSearch.value || searchable.includes(normalizedSearch.value)
      );
    }),
  );
  const enabledCount = computed(
    () => allPlugins.value.filter((item) => item.record.enabled).length,
  );
  const appCount = computed(
    () =>
      allPlugins.value.filter((item) => item.record.manifest.ui?.enabled).length,
  );
  const themeCount = computed(
    () =>
      allPlugins.value.filter((item) => item.record.manifest.theme?.enabled)
        .length,
  );
  const themeSwatches = (theme: PluginThemeRecord) => {
    const palette = options.isDark() ? theme.theme.dark : theme.theme.light;
    return [
      palette.primary ?? "#2563EB",
      palette.bg_base ?? "#F3F5F7",
      palette.bg_card ?? "#FFFFFF",
      palette.text_default ?? "#172033",
    ];
  };

  const healthByPlugin = computed(() => {
    const values = new Map<string, PluginHealthReport>();
    for (const report of options.pluginHealth.value) {
      values.set(report.plugin_id, report);
    }
    return values;
  });
  const pluginHealthReport = (item: ManagedPlugin) =>
    healthByPlugin.value.get(item.record.manifest.id);
  const marketIcon = () => "i-carbon-application-web";
  const pluginIcon = (item: ManagedPlugin) =>
    item.record.manifest.ui?.routes?.[0]?.icon ?? marketIcon();
  const marketEntryIsSigned = (entry: PluginMarketEntry) =>
    entry.assets.some((asset) => Boolean(asset.signature_ed25519));
  const healthText = (report?: PluginHealthReport) => {
    if (!report) return "未知";
    if (!report.healthy) return "异常";
    return report.checks.some((check) => check.severity === "warning")
      ? "警告"
      : "健康";
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

  return {
    allPlugins,
    capabilityLabel,
    visibleCapabilities,
    normalizedSearch,
    visiblePlugins,
    marketVisiblePlugins,
    marketVisibleUpdates,
    enabledCount,
    appCount,
    themeCount,
    themeSwatches,
    healthByPlugin,
    pluginHealthReport,
    pluginIcon,
    marketIcon,
    marketEntryIsSigned,
    healthText,
    healthTone,
    formatTime,
  };
}
