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
import * as envApi from "@/api/environment";
import type {
  Env,
  Package,
  PackageDependencyMap,
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

const normalizeDependencyMap = (
  dependencies: PackageDependencyMap,
): Package[] =>
  Object.entries(dependencies).map(([name, info]) => {
    if (typeof info === "string") return { name, version: info };
    return { name, version: info.version || "unknown" };
  });

export const normalizePackagePayload = (
  payload: PackageListPayload,
): Package[] => {
  let rawPayload: unknown = payload;

  if (typeof rawPayload === "string") {
    try {
      rawPayload = JSON.parse(rawPayload) as unknown;
    } catch {
      return [];
    }
  }

  if (Array.isArray(rawPayload)) {
    return rawPayload
      .map((item) => normalizePackage(item))
      .filter((item): item is Package => item !== null);
  }

  if (isObjectRecord(rawPayload) && isObjectRecord(rawPayload.dependencies)) {
    return normalizeDependencyMap(
      rawPayload.dependencies as PackageDependencyMap,
    );
  }

  return [];
};

export function useEnvPackageManager({
  env,
  isVisible,
  onShowLog,
}: UseEnvPackageManagerOptions) {
  const loading = ref(false);
  const packages = ref<Package[]>([]);
  const searchQuery = ref("");

  const installDialogVisible = ref(false);
  const installing = ref(false);
  const installForm = reactive({ packages: "" });

  const uninstallDialogVisible = ref(false);
  const uninstallForm = reactive({ package: "" });

  const currentEnv = computed(() => toValue(env) ?? null);

  const dialogTitle = computed(() => {
    if (!currentEnv.value) return "依赖管理";
    return currentEnv.value.env_type === "node"
      ? "Node.js 全局依赖管理"
      : `${currentEnv.value.name} 依赖管理`;
  });

  const filteredPackages = computed(() => {
    if (!searchQuery.value) return packages.value;

    const query = searchQuery.value.toLowerCase();
    return packages.value.filter(
      (pkg) =>
        pkg.name.toLowerCase().includes(query) ||
        pkg.version.toLowerCase().includes(query),
    );
  });

  const loadPackages = async (targetEnv = currentEnv.value) => {
    if (!targetEnv) return;

    loading.value = true;
    try {
      const res = await envApi.getPackages(targetEnv);
      packages.value = normalizePackagePayload(res.data);
    } catch {
      ElMessage.error("加载包列表失败");
    } finally {
      loading.value = false;
    }
  };

  const showInstallDialog = () => {
    installForm.packages = "";
    installDialogVisible.value = true;
  };

  const handleInstallPackages = async () => {
    if (!installForm.packages.trim() || !currentEnv.value || installing.value) {
      return;
    }

    installing.value = true;
    const packageList = installForm.packages
      .split("\n")
      .map((pkg) => pkg.trim())
      .filter(Boolean);

    try {
      const res = await envApi.installPackages(currentEnv.value, {
        packages: packageList,
      });
      ElMessage.success("安装任务已提交");
      onShowLog(res.data, `安装依赖 - ${packageList.join(", ")}`);
      installDialogVisible.value = false;
    } finally {
      installing.value = false;
    }
  };

  const showUninstallDialog = () => {
    uninstallForm.package = "";
    uninstallDialogVisible.value = true;
  };

  const handleUninstallPackage = async (packageName: string) => {
    if (!packageName || !currentEnv.value) return;

    try {
      await ElMessageBox.confirm(`确定要卸载 ${packageName} 吗？`, "提示", {
        type: "warning",
      });

      const res = await envApi.uninstallPackage(currentEnv.value, packageName);

      if (
        currentEnv.value.env_type === "python" ||
        currentEnv.value.env_type === "node"
      ) {
        ElMessage.success("卸载任务已提交");
        if (typeof res.data === "string" || typeof res.data === "number") {
          onShowLog(res.data, `卸载依赖 - ${packageName}`);
        }
      } else {
        ElMessage.success("卸载成功");
      }

      void loadPackages();
    } catch {
      // Confirmation cancellation and API failures keep the dialog state intact.
    }
  };

  const handleManualUninstall = async () => {
    if (!uninstallForm.package.trim()) return;

    uninstallDialogVisible.value = false;
    await handleUninstallPackage(uninstallForm.package.trim());
  };

  const handleJobFinished = () => {
    if (toValue(isVisible) && currentEnv.value) {
      void loadPackages();
    }
  };

  watch(
    () => toValue(isVisible),
    (visible) => {
      if (visible && currentEnv.value) {
        void loadPackages();
      }
    },
  );

  onMounted(() => {
    window.addEventListener("niu:job-finished", handleJobFinished);
  });

  onUnmounted(() => {
    window.removeEventListener("niu:job-finished", handleJobFinished);
  });

  return {
    dialogTitle,
    filteredPackages,
    handleInstallPackages,
    handleManualUninstall,
    handleUninstallPackage,
    installDialogVisible,
    installForm,
    installing,
    loadPackages,
    loading,
    packages,
    searchQuery,
    showInstallDialog,
    showUninstallDialog,
    uninstallDialogVisible,
    uninstallForm,
  };
}
