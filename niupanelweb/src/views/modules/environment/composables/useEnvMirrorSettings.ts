import { computed, reactive, ref, type Ref } from "vue";
import { ElMessage } from "element-plus";
import * as envApi from "@/api/environment";
import * as settingsApi from "@/api/settings";
import {
  NODE_MIRRORS,
  PYTHON_MIRRORS,
} from "@/constants/mirrors";
import type { EnvType, GeneralSettings, SettingItem } from "@/types";

type UseEnvMirrorSettingsOptions = {
  filterType: Ref<EnvType>;
  onClose: () => void;
};

const DEFAULT_NODE_DIST_MIRROR = "https://mirrors.ustc.edu.cn/node/";
const DEFAULT_PNPM_REGISTRY = "https://registry.npmmirror.com/";

const toSettingsMap = (settings: SettingItem[]) => {
  return settings.reduce<Record<string, string>>((map, item) => {
    map[item.key] = item.value;
    return map;
  }, {});
};

const toNumberSetting = (value: string | undefined, fallback: number) => {
  const parsed = parseInt(value || `${fallback}`, 10);
  return Number.isNaN(parsed) ? fallback : parsed;
};

const buildGeneralSettings = (
  settingsMap: Record<string, string>,
  uvPythonMirror: string,
  uvPypiMirror: string,
  nodeDistMirror: string,
  npmRegistryMirror: string,
): GeneralSettings => ({
  name: settingsMap["system.name"] || "",
  logo: settingsMap["system.logo"] || "",
  timezone: settingsMap["system.timezone"] || "",
  max_concurrency: toNumberSetting(settingsMap["system.max_concurrency"], 15),
  log_retention_days: toNumberSetting(settingsMap["system.log_retention_days"], 15),
  github_proxy_url: settingsMap["system.github_proxy_url"] || "",
  uv_python_mirror: uvPythonMirror,
  uv_pypi_mirror: uvPypiMirror,
  default_python_version: settingsMap["system.default_python_version"] || "",
  default_node_version: settingsMap["system.default_node_version"] || "",
  pnpm_node_dist_mirror: nodeDistMirror,
  npm_registry_mirror: npmRegistryMirror,
});

export function useEnvMirrorSettings({
  filterType,
  onClose,
}: UseEnvMirrorSettingsOptions) {
  const loading = ref(false);
  const submitting = ref(false);
  const activeTab = ref("uv");
  const form = reactive({ url: "" });
  const uvForm = reactive({
    pythonMirror: "",
    pypiMirror: "",
  });
  const nodeForm = reactive({
    distMirror: "",
    registryMirror: "",
  });

  const fullSettings = ref<GeneralSettings | null>(null);

  const currentMirrorPresets = computed(() => {
    if (filterType.value === "python") return PYTHON_MIRRORS;
    if (filterType.value === "node") return NODE_MIRRORS;
    return [];
  });

  const getMirrorTitle = () => {
    if (filterType.value === "python") return "Python";
    if (filterType.value === "node") return "Node.js";
    if (filterType.value === "sh") return "Shell";
    return "镜像源";
  };

  const loadCurrentSettings = async () => {
    loading.value = true;
    try {
      const res = await settingsApi.getSettings();
      const settingsMap = toSettingsMap(res.data);

      uvForm.pythonMirror = settingsMap["system.uv_python_mirror"] || "";
      uvForm.pypiMirror = settingsMap["system.uv_pypi_mirror"] || "";
      nodeForm.distMirror =
        settingsMap["system.pnpm_node_dist_mirror"] ||
        settingsMap["system.fnm_node_dist_mirror"] ||
        DEFAULT_NODE_DIST_MIRROR;
      nodeForm.registryMirror =
        settingsMap["system.npm_registry_mirror"] || DEFAULT_PNPM_REGISTRY;

      fullSettings.value = buildGeneralSettings(
        settingsMap,
        uvForm.pythonMirror,
        uvForm.pypiMirror,
        nodeForm.distMirror,
        nodeForm.registryMirror,
      );

      activeTab.value = filterType.value === "node" ? "node_dist" : "uv";
    } catch {
      ElMessage.error("加载设置失败");
    } finally {
      loading.value = false;
    }
  };

  const ensureFullSettings = async () => {
    if (!fullSettings.value) {
      await loadCurrentSettings();
    }
    return fullSettings.value;
  };

  const submitUvSettings = async () => {
    const settings = await ensureFullSettings();
    if (!settings) return;

    await settingsApi.updateGeneralSettings({
      ...settings,
      uv_python_mirror: uvForm.pythonMirror.trim(),
      uv_pypi_mirror: uvForm.pypiMirror.trim(),
    });
    ElMessage.success("UV 镜像设置成功");
  };

  const submitNodeDistSettings = async () => {
    const settings = await ensureFullSettings();
    if (!settings) return;

    await settingsApi.updateGeneralSettings({
      ...settings,
      pnpm_node_dist_mirror: nodeForm.distMirror.trim(),
    });
    ElMessage.success("Node.js 下载镜像设置成功");
  };

  const submitNodeRegistrySettings = async () => {
    const settings = await ensureFullSettings();
    if (!settings) return;

    await settingsApi.updateGeneralSettings({
      ...settings,
      npm_registry_mirror: nodeForm.registryMirror.trim(),
    });
    ElMessage.success("pnpm 包镜像设置成功");
  };

  const submitLegacyMirror = async () => {
    if (!form.url.trim()) {
      ElMessage.warning("请输入镜像地址");
      return false;
    }

    await envApi.setMirrorSource(filterType.value, form.url.trim());
    ElMessage.success("镜像源设置成功");
    return true;
  };

  const handleSubmit = async () => {
    submitting.value = true;
    try {
      if (filterType.value === "python" && activeTab.value === "uv") {
        await submitUvSettings();
      } else if (filterType.value === "node" && activeTab.value === "node_dist") {
        await submitNodeDistSettings();
      } else if (filterType.value === "node" && activeTab.value === "pnpm") {
        await submitNodeRegistrySettings();
      } else {
        const submitted = await submitLegacyMirror();
        if (!submitted) return;
      }
      onClose();
    } catch {
    } finally {
      submitting.value = false;
    }
  };

  const resetForm = () => {
    form.url = "";
  };

  return {
    activeTab,
    currentMirrorPresets,
    form,
    getMirrorTitle,
    handleSubmit,
    loadCurrentSettings,
    loading,
    nodeForm,
    resetForm,
    submitting,
    uvForm,
  };
}
