import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { listPluginThemes } from "@/api/plugins";
import { storageKey as makeStorageKey } from "@/utils/storage";
import type { PluginThemePalette, PluginThemeRecord } from "@/types";

const activeThemeStorageKey = makeStorageKey("plugin_theme");
const themeStyleId = "niupanel-plugin-theme";

const paletteVariables: Record<keyof PluginThemePalette, string> = {
  primary: "--brand-primary",
  bg_base: "--bg-base",
  bg_card: "--bg-card",
  bg_subtle: "--bg-subtle",
  bg_soft: "--bg-soft",
  text_default: "--text-default",
  text_secondary: "--text-secondary",
  text_muted: "--text-muted",
  border_base: "--border-base",
  border_light: "--border-light",
};

const mixHex = (from: string, to: string, ratio: number) => {
  const parse = (color: string) => [1, 3, 5].map((offset) => Number.parseInt(color.slice(offset, offset + 2), 16));
  const source = parse(from.slice(0, 7));
  const target = parse(to);
  const mixed = source.map((channel, index) =>
    Math.round(channel + (target[index] - channel) * ratio)
      .toString(16)
      .padStart(2, "0"),
  );
  return `#${mixed.join("")}`;
};

const rgbChannels = (color: string) =>
  [1, 3, 5]
    .map((offset) => Number.parseInt(color.slice(offset, offset + 2), 16))
    .join(" ");

const primaryVariables = (
  primary?: string | null,
  dark = false,
): Record<string, string> => {
  if (!primary) return {};
  const variables: Record<string, string> = {
    "--brand-primary": primary,
    "--brand-primary-rgb": rgbChannels(primary),
    "--el-color-primary": primary,
    "--el-color-primary-dark-2": mixHex(primary, "#000000", 0.2),
    "--accent-subtle-bg": mixHex(primary, dark ? "#000000" : "#ffffff", dark ? 0.72 : 0.9),
    "--accent-subtle-text": mixHex(primary, dark ? "#ffffff" : "#000000", dark ? 0.36 : 0.28),
    "--accent-subtle-border": mixHex(primary, dark ? "#000000" : "#ffffff", dark ? 0.48 : 0.68),
  };
  for (let step = 1; step <= 8; step += 1) {
    variables[`--el-color-primary-light-${step}`] = mixHex(primary, "#ffffff", step / 10);
  }
  variables["--el-color-primary-light-9"] = mixHex(
    primary,
    dark ? "#000000" : "#ffffff",
    0.9,
  );
  return variables;
};

const cssVariables = (palette: PluginThemePalette, dark = false) => {
  const variables: Record<string, string> = primaryVariables(palette.primary, dark);
  for (const [token, variable] of Object.entries(paletteVariables)) {
    const value = palette[token as keyof PluginThemePalette];
    if (value) variables[variable] = value;
  }
  if (palette.bg_card) {
    variables["--el-bg-color"] = palette.bg_card;
    variables["--el-bg-color-overlay"] = palette.bg_card;
    variables["--bg-glass"] = `color-mix(in srgb, ${palette.bg_card} 94%, transparent)`;
  }
  if (palette.text_default) variables["--el-text-color-primary"] = palette.text_default;
  if (palette.text_secondary) variables["--el-text-color-regular"] = palette.text_secondary;
  if (palette.text_muted) variables["--el-text-color-secondary"] = palette.text_muted;
  if (palette.border_base) variables["--el-border-color"] = palette.border_base;
  if (palette.border_light) variables["--el-border-color-light"] = palette.border_light;
  return Object.entries(variables)
    .map(([name, value]) => `  ${name}: ${value};`)
    .join("\n");
};

const installThemeStyle = (theme: PluginThemeRecord) => {
  document.getElementById(themeStyleId)?.remove();
  const style = document.createElement("style");
  style.id = themeStyleId;
  style.textContent = `:root[data-plugin-theme="${theme.plugin_id}"] {\n${cssVariables(theme.theme.light)}\n}\nhtml.dark[data-plugin-theme="${theme.plugin_id}"] {\n${cssVariables(theme.theme.dark, true)}\n}`;
  document.head.appendChild(style);
  document.documentElement.dataset.pluginTheme = theme.plugin_id;
};

const removeThemeStyle = () => {
  document.getElementById(themeStyleId)?.remove();
  delete document.documentElement.dataset.pluginTheme;
};

export const usePluginThemesStore = defineStore("pluginThemes", () => {
  const themes = ref<PluginThemeRecord[]>([]);
  const activeThemeId = ref(localStorage.getItem(activeThemeStorageKey));
  const loaded = ref(false);
  const loading = ref(false);

  const activeTheme = computed(
    () => themes.value.find((theme) => theme.plugin_id === activeThemeId.value) ?? null,
  );

  const applyActiveTheme = () => {
    if (activeTheme.value) {
      installThemeStyle(activeTheme.value);
    } else {
      removeThemeStyle();
    }
  };

  const setActiveTheme = (pluginId: string | null) => {
    activeThemeId.value = pluginId;
    if (pluginId) localStorage.setItem(activeThemeStorageKey, pluginId);
    else localStorage.removeItem(activeThemeStorageKey);
    applyActiveTheme();
  };

  const loadThemes = async (force = false) => {
    if (loading.value || (loaded.value && !force)) return themes.value;
    loading.value = true;
    try {
      const response = await listPluginThemes();
      themes.value = response.data ?? [];
      loaded.value = true;
      if (activeThemeId.value && !activeTheme.value) {
        setActiveTheme(null);
      } else {
        applyActiveTheme();
      }
      return themes.value;
    } finally {
      loading.value = false;
    }
  };

  return {
    activeTheme,
    activeThemeId,
    applyActiveTheme,
    loaded,
    loading,
    loadThemes,
    setActiveTheme,
    themes,
  };
});
