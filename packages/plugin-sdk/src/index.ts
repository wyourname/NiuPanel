export type NiuPanelPluginUiMode = "vue_app";

export type NiuPanelPluginRoute = {
  id?: string | null;
  path: string;
  title: string;
  description?: string | null;
  icon?: string | null;
  menu?: string | null;
  order?: number;
  hidden?: boolean;
  exact?: boolean;
};

export type NiuPanelPluginLayout = "panel" | "full_bleed";

export type NiuPanelPluginDisplay = {
  sidebar: boolean;
  workspace: boolean;
  mobile: boolean;
  category?: string | null;
  order: number;
  layout: NiuPanelPluginLayout;
};

export type NiuPanelPluginApiRule = {
  methods: Array<"GET" | "POST" | "PUT" | "PATCH" | "DELETE">;
  path: string;
};

export type NiuPanelPluginApiManifest = {
  allow: NiuPanelPluginApiRule[];
};

export type NiuPanelPluginThemePalette = {
  primary?: string | null;
  bg_base?: string | null;
  bg_card?: string | null;
  bg_subtle?: string | null;
  bg_soft?: string | null;
  text_default?: string | null;
  text_secondary?: string | null;
  text_muted?: string | null;
  border_base?: string | null;
  border_light?: string | null;
};

export type NiuPanelPluginThemeManifest = {
  enabled: boolean;
  light: NiuPanelPluginThemePalette;
  dark: NiuPanelPluginThemePalette;
};

export type NiuPanelPluginApp = {
  plugin_id: string;
  name: string;
  version: string;
  description: string;
  capabilities: string[];
  ui: {
    mode: NiuPanelPluginUiMode;
    entry_url: string;
    sdk_version?: string | null;
    display: NiuPanelPluginDisplay;
    routes: NiuPanelPluginRoute[];
    permissions: string[];
    api: NiuPanelPluginApiManifest;
  };
};

export type NiuPanelPluginApiRequest = {
  method?: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  path: string;
  data?: unknown;
  params?: Record<string, unknown>;
};

export type NiuPanelPluginRouteSnapshot = {
  path: string;
  query: Record<string, unknown>;
};

export type NiuPanelPluginRouteListener = (
  route: NiuPanelPluginRouteSnapshot,
) => void;

export type NiuPanelPluginContext = {
  pluginId: string;
  app: NiuPanelPluginApp;
  route: NiuPanelPluginRouteSnapshot & {
    onChange(listener: NiuPanelPluginRouteListener): () => void;
  };
  api: {
    request<T = unknown>(options: NiuPanelPluginApiRequest): Promise<T>;
    invoke<T = unknown>(action: string, input?: unknown): Promise<T>;
  };
  ui: {
    toast(
      message: string,
      type?: "success" | "warning" | "error" | "info",
    ): void;
    confirm(message: string, title?: string): Promise<boolean>;
    navigate(path: string): Promise<void>;
  };
};

export type NiuPanelPluginModule<TInstance = unknown> = {
  mount(
    el: HTMLElement,
    context: NiuPanelPluginContext,
  ): TInstance | Promise<TInstance>;
  unmount?(
    instance: TInstance,
    context: NiuPanelPluginContext,
  ): void | Promise<void>;
};

export function definePlugin<TInstance = unknown>(
  plugin: NiuPanelPluginModule<TInstance>,
): NiuPanelPluginModule<TInstance> {
  return plugin;
}
