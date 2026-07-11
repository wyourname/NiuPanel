import type {
  NiuPanelPluginApp,
  NiuPanelPluginApiManifest,
  NiuPanelPluginDisplay,
  NiuPanelPluginRoute,
  NiuPanelPluginThemeManifest,
  NiuPanelPluginThemePalette,
  NiuPanelPluginUiMode,
} from "@niupanel/plugin-sdk";

export type PluginUiMode = NiuPanelPluginUiMode;

export type PluginUiRoute = NiuPanelPluginRoute;

export type PluginUiDisplay = NiuPanelPluginDisplay;

export type PluginUiApiManifest = NiuPanelPluginApiManifest;

export type PluginThemePalette = NiuPanelPluginThemePalette;

export type PluginThemeManifest = NiuPanelPluginThemeManifest;

export type PluginAppRecord = NiuPanelPluginApp;

export type PluginAppUi = NiuPanelPluginApp["ui"];

export type PluginRuntime = "builtin" | "declarative" | "process" | "wasi" | "native";
export type PluginProcessProtocol = "single_shot" | "json_lines";
export type PluginRuntimePermission = "network_outbound";
export type PluginSource = "builtin" | "local";
export type PluginStatus = "enabled" | "disabled" | "error";

export interface PluginVersionRecord {
  id: string;
  version: string;
  archived_at: string;
  path: string;
  package_sha256?: string | null;
}

export interface PluginMarketIndex {
  schema_version: number;
  name: string;
  description?: string | null;
  plugins: PluginMarketEntry[];
}

export interface PluginMarketEntry {
  id: string;
  name: string;
  version: string;
  description: string;
  download_url: string;
  checksum_sha256?: string | null;
  signature_ed25519?: string | null;
  public_key_ed25519?: string | null;
  permissions: string[];
  homepage?: string | null;
  repository?: string | null;
}

export interface PluginMarketInstallRequest {
  index_url: string;
  plugin_id: string;
  enable?: boolean;
}

export interface PluginMarketSource {
  name: string;
  url: string;
  enabled: boolean;
}

export interface PluginMarketSourcesUpdateRequest {
  sources: PluginMarketSource[];
}

export interface PluginMarketUpdateRecord {
  source_name: string;
  source_url: string;
  plugin_id: string;
  installed_version: string;
  available_version: string;
  entry: PluginMarketEntry;
}

export interface PluginImpactPreview {
  operation: string;
  plugin_id: string;
  name: string;
  current_version?: string | null;
  target_version: string;
  ui_enabled: boolean;
  theme_enabled: boolean;
  routes: PluginImpactRoute[];
  route_conflicts: PluginRouteConflict[];
  permissions_added: string[];
  permissions_removed: string[];
  api_allow_added: string[];
  api_allow_removed: string[];
  warnings: string[];
  blockers: string[];
  install_allowed: boolean;
}

export interface PluginImpactRoute {
  path: string;
  title: string;
  hidden: boolean;
}

export interface PluginRouteConflict {
  path: string;
  plugin_id: string;
  name: string;
}

export interface PluginHealthReport {
  plugin_id: string;
  name: string;
  version: string;
  enabled: boolean;
  healthy: boolean;
  summary: string;
  checks: PluginHealthCheck[];
}

export interface PluginHealthCheck {
  code: string;
  severity: "ok" | "warning" | "error";
  message: string;
}

export interface PluginWorkerConfig {
  min: number;
  max: number;
  idle_timeout_sec: number;
}

export interface PluginManifest {
  schema_version: number;
  id: string;
  name: string;
  version: string;
  description: string;
  runtime: PluginRuntime;
  protocol: PluginProcessProtocol;
  entry?: string | null;
  args: string[];
  env: Record<string, string>;
  runtime_permissions: PluginRuntimePermission[];
  timeout_sec?: number | null;
  worker: PluginWorkerConfig;
  capabilities: string[];
  tools: unknown[];
  compatibility: PluginCompatibilityManifest;
  ui?: PluginAppUiManifest | null;
  theme?: PluginThemeManifest | null;
}

export interface PluginThemeRecord {
  plugin_id: string;
  name: string;
  version: string;
  description: string;
  theme: PluginThemeManifest;
}

export interface PluginCompatibilityManifest {
  panel: PluginPanelCompatibility;
  dependencies: PluginDependency[];
}

export interface PluginPanelCompatibility {
  min_version?: string | null;
  max_version?: string | null;
}

export interface PluginDependency {
  id: string;
  min_version?: string | null;
  optional: boolean;
}

export interface PluginAppUiManifest {
  enabled: boolean;
  mode: PluginUiMode;
  entry: string;
  sdk_version?: string | null;
  display?: PluginUiDisplay | null;
  routes: PluginUiRoute[];
  permissions: string[];
  api?: PluginUiApiManifest | null;
}

export interface PluginRecord {
  manifest: PluginManifest;
  enabled: boolean;
  active_version?: string | null;
  source: PluginSource;
  status: PluginStatus;
  path?: string | null;
}

export interface PluginInstallRequest {
  source_path: string;
  enable?: boolean;
}

export interface PluginUpdateRequest {
  source_path: string;
}
