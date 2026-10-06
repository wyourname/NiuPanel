import {
  computed,
  onMounted,
  onUnmounted,
  reactive,
  ref,
  toValue,
  watch,
  type MaybeRefOrGetter,
} from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { useEnvironmentJobsStore, environmentJobKey, isJobActive } from "@/stores/environmentJobs";
import { environmentLabel, parsePackageInput } from "./packageInput";
import * as envApi from "@/api/environment";
import type {
  Env,
  Package,
  PackageListPayload,
} from "@/types";

type UseEnvPackageManagerOptions = {
  env: MaybeRefOrGetter<Env | null | undefined>;
  isVisible: MaybeRefOrGetter<boolean>;
  onShowLog: (id: number | string, name: string) => void;
};

const isObjectRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

const normalizePackage = (value: unknown): Package | null => {
  if (!isObjectRecord(value) || typeof value.name !== "string") return null;

  const version =
    typeof value.version === "string" || typeof value.version === "number"
      ? String(value.version)
      : "unknown";

  return { name: value.name, version };
};

export const normalizePackagePayload = (
  payload: PackageListPayload,
): Package[] => {
  let rawPayload: unknown = payload;

  if (typeof rawPayload === "string") {
    try {
      rawPayload = JSON.parse(rawPayload) as unknown;
    } catch {
      throw new Error("Invalid package list");
    }
  }

  if (Array.isArray(rawPayload)) {
    const result = rawPayload.map(normalizePackage);
    if (result.some(item => item === null)) throw new Error("Invalid package list");
    return result as Package[];
  }

  if (isObjectRecord(rawPayload) && isObjectRecord(rawPayload.dependencies)) {
    return Object.entries(rawPayload.dependencies).map(([name, info]) => {
      if (typeof info === "string") return { name, version: info };
      if (!isObjectRecord(info)) throw new Error("Invalid package version");
      return { name, version: typeof info.version === "string" ? info.version : "unknown" };
    });
  }

  throw new Error("Invalid package list");
};

export function useEnvPackageManager({ env, isVisible, onShowLog }: UseEnvPackageManagerOptions) {
  const loading = ref(false);
  const loadError = ref("");
  const packages = ref<Package[]>([]);
  const installing = ref(false);
  const installForm = reactive({ packages: "" });
  const uninstallingPackage = ref("");
  const currentEnv = computed(() => toValue(env) ?? null);
  const keyOf = environmentJobKey;
  const jobStore = useEnvironmentJobsStore();
  const currentJob = computed(() => currentEnv.value ? jobStore.jobs[keyOf(currentEnv.value)] : undefined);
  const restoringJobs = computed(() => jobStore.restoring);
  const jobRestoreError = computed(() => jobStore.error);
  const restoreJobs = jobStore.restore;
  const busy = computed(() => installing.value || Boolean(uninstallingPackage.value) || jobStore.restoring || !!jobStore.error || isJobActive(currentJob.value));
  const parsedInput = computed(() => parsePackageInput(installForm.packages, currentEnv.value?.env_type ?? "python"));
  const dialogTitle = computed(() => currentEnv.value ? `${environmentLabel(currentEnv.value)} 依赖管理` : "依赖管理");
  let listRequest = 0;
  const loadPackages = async (target = currentEnv.value) => {
    if (!target || !toValue(isVisible)) return;
    const id = ++listRequest;
    const key = keyOf(target);
    loading.value = true;
    loadError.value = "";
    try {
      const res = await envApi.getPackages(target);
      if (id === listRequest && currentEnv.value && keyOf(currentEnv.value) === key) packages.value = normalizePackagePayload(res.data);
    } catch {
      if (id === listRequest) loadError.value = "依赖列表加载失败，请重试";
    } finally { if (id === listRequest) loading.value = false; }
  };

  const trackJob = (target: Env, id: number, name: string) => jobStore.track(target, id, name);
  const handleInstallPackages = async () => {
    const target = currentEnv.value;
    if (!target || busy.value) return;
    if (parsedInput.value.error) { ElMessage.warning(parsedInput.value.error); return; }
    const packageList = parsedInput.value.packages;
    if (!packageList.length) return;
    installing.value = true;
    try {
      const response = await envApi.installPackages(target, { packages: packageList });
      if (currentEnv.value && keyOf(currentEnv.value) === keyOf(target)) installForm.packages = "";
      ElMessage.success("安装任务已提交，可在日志中查看结果");
      trackJob(target, response.data, `安装依赖 · ${environmentLabel(target)}`);
    } catch {
      // Preserve the input for retry.
    } finally { installing.value = false; }
  };
  const handleUninstallPackage = async (packageName: string) => {
    const target = currentEnv.value;
    if (!target || !packageName || busy.value) return;
    uninstallingPackage.value = packageName;
    try {
      await ElMessageBox.confirm(`确定从 ${environmentLabel(target)} 卸载“${packageName}”吗？依赖该包的任务可能无法运行。`, "卸载依赖", { type: "warning", confirmButtonText: "确认卸载", cancelButtonText: "取消" });
      const response = await envApi.uninstallPackage(target, packageName);
      trackJob(target, response.data, `卸载 ${packageName} · ${environmentLabel(target)}`);
      ElMessage.success("卸载任务已提交，可在日志中查看结果");
    } catch {
      // Cancellation is expected; API failures are reported by the interceptor.
    } finally { uninstallingPackage.value = ""; }
  };
  const showJobLog = () => { if (currentJob.value) onShowLog(currentJob.value.id, currentJob.value.name); };
  const jobStatusLabel = computed(() => {

    const status = currentJob.value?.status;
    if (!status) return "";
    return ({ Pending: "排队中", Running: "执行中", Success: "已完成", Finished: "已完成", Failed: "执行失败，请查看日志", Cancelled: "已取消" } as Record<string, string>)[status] || status;
  });
  const handleJobFinished = () => { if (toValue(isVisible)) void loadPackages(); };
  watch(() => [toValue(isVisible), currentEnv.value ? keyOf(currentEnv.value) : ""] as const, ([visible], previous) => {
    listRequest++;
    loading.value = false;
    if (!visible || !previous || previous[1] !== (currentEnv.value ? keyOf(currentEnv.value) : "")) {
      installForm.packages = "";
      packages.value = [];
      loadError.value = "";
    }
    if (visible) { void jobStore.restore(); void loadPackages(); }
  }, { immediate: true });
  onMounted(() => window.addEventListener("niu:job-finished", handleJobFinished));
  onUnmounted(() => {
    listRequest++;
    window.removeEventListener("niu:job-finished", handleJobFinished);
  });
  return { restoringJobs, jobRestoreError, restoreJobs, dialogTitle, handleInstallPackages, handleUninstallPackage, installForm, installing, loadPackages, loading, loadError, packages, uninstallingPackage, parsedInput, busy, currentJob, jobStatusLabel, showJobLog };
}
