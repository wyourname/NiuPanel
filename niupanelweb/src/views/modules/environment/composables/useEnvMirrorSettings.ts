import { computed, reactive, ref, type Ref } from "vue";
import { ElMessage } from "element-plus";
import * as envApi from "@/api/environment";
import type { EnvType, EnvironmentMirrorSettings } from "@/types";

export function useEnvMirrorSettings({ filterType, onClose }: {
  filterType: Ref<EnvType>;
  onClose: () => void;
}) {
  const loading = ref(false);
  const submitting = ref(false);
  const error = ref("");
  const settings = ref<EnvironmentMirrorSettings | null>(null);
  const form = reactive({ package_url: "", runtime_url: "" });
  let requestId = 0;
  const title = computed(() => filterType.value === "node" ? "Node.js 依赖源" : filterType.value === "python" ? "Python 依赖源" : "Linux 软件源");
  const validUrl = (value: string) => {
    try {
      const url = new URL(value.trim());
      return ["https:", "http:"].includes(url.protocol) && !/\s/.test(value.trim()) && !url.hash;
    } catch { return false; }
  };
  const validationError = computed(() => {
    if (!validUrl(form.package_url)) return "请输入完整的 HTTP 或 HTTPS 包下载源地址";
    if (!validUrl(form.runtime_url)) return "请输入完整的 HTTP 或 HTTPS 运行时下载源地址";
    return "";
  });
  const loadCurrentSettings = async () => {
    const id = ++requestId;
    settings.value = null;
    error.value = "";
    form.package_url = "";
    form.runtime_url = "";
    if (filterType.value === "sh") { loading.value = false; return; }
    const type = filterType.value;
    loading.value = true;
    try {
      const response = await envApi.getMirrorSettings(type);
      if (id !== requestId) return;
      settings.value = response.data;
      Object.assign(form, { package_url: response.data.package_url, runtime_url: response.data.runtime_url });
    } catch {
      if (id === requestId) error.value = "加载依赖源失败，请重试后再保存";
    } finally {
      if (id === requestId) loading.value = false;
    }
  };
  const handleSubmit = async () => {
    if (submitting.value || loading.value || !settings.value || filterType.value === "sh") return;
    if (validationError.value) { ElMessage.warning(validationError.value); return; }
    const type = filterType.value;
    submitting.value = true;
    try {
      await envApi.updateMirrorSettings(type, { package_url: form.package_url.trim(), runtime_url: form.runtime_url.trim() });
      ElMessage.success("依赖源已保存，下次安装时生效");
      onClose();
    } catch {
      // Keep edits available for retry; the request interceptor reports the failure.
    } finally { submitting.value = false; }
  };
  const cancelLoad = () => { requestId++; loading.value = false; };
  return { loading, submitting, error, settings, form, title, validationError, loadCurrentSettings, handleSubmit, cancelLoad };
}
