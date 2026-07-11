import { onScopeDispose, ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import * as settingsApi from "@/api/settings";
import type { BackupOptions, MaintenanceStatus } from "@/types";

const POLL_INTERVAL_MS = 1000;
const RESTORE_RELOAD_DELAY_MS = 1500;

const createMaintenanceStatus = (
  message: string,
  status: MaintenanceStatus["status"] = "pending",
): MaintenanceStatus => ({
  progress: 0,
  message,
  status,
  filename: null,
});

export function useSystemMaintenance() {
  const backingUp = ref(false);
  const restoring = ref(false);
  const cleaningLogs = ref(false);
  const cleanupDays = ref(30);
  const restoreInputRef = ref<HTMLInputElement | null>(null);

  const backupOptions = ref<BackupOptions>({
    tasks: true,
    variables: true,
    settings: true,
    environments: false,
    telegram: true,
  });

  const maintenanceProgress = ref<MaintenanceStatus>(
    createMaintenanceStatus("准备中"),
  );

  let pollTimer: ReturnType<typeof setInterval> | null = null;

  const clearPolling = () => {
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = null;
  };

  const triggerDownload = async (filename: string) => {
    try {
      const blob = await settingsApi.downloadBackup(filename);
      const url = window.URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.setAttribute("download", filename);
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      window.URL.revokeObjectURL(url);
      ElMessage.success("备份已创建并开始下载");
    } catch {
      ElMessage.error("下载备份文件失败");
    }
  };

  const startPolling = (type: "backup" | "restore") => {
    clearPolling();
    pollTimer = setInterval(async () => {
      try {
        const { data } = await settingsApi.getMaintenanceStatus();
        maintenanceProgress.value = data;

        if (data.status === "completed") {
          clearPolling();
          if (type === "backup") {
            if (data.filename) {
              await triggerDownload(data.filename);
            }
            backingUp.value = false;
          } else if (type === "restore") {
            ElMessage.success("系统恢复成功，正在刷新...");
            setTimeout(() => window.location.reload(), RESTORE_RELOAD_DELAY_MS);
          }
        } else if (data.status === "error") {
          clearPolling();
          ElMessage.error(data.message || "任务失败");
          backingUp.value = false;
          restoring.value = false;
        }
      } catch {
        clearPolling();
        backingUp.value = false;
        restoring.value = false;
      }
    }, POLL_INTERVAL_MS);
  };

  const handleBackup = async () => {
    const hasSelection = Object.values(backupOptions.value).some((selected) => selected);
    if (!hasSelection) {
      ElMessage.warning("请至少选择一项备份内容");
      return;
    }

    backingUp.value = true;
    maintenanceProgress.value = createMaintenanceStatus(
      "正在启动备份任务...",
      "processing",
    );

    try {
      await settingsApi.backupSystem(backupOptions.value);
      startPolling("backup");
    } catch {
      backingUp.value = false;
    }
  };

  const triggerRestore = () => restoreInputRef.value?.click();

  const handleRestore = async (event: Event) => {
    const target = event.target;
    if (!(target instanceof HTMLInputElement)) return;
    const file = target.files?.[0];
    if (!file) return;

    try {
      await ElMessageBox.confirm(
        "确定恢复数据？这将覆盖勾选相关的现有数据！",
        "高风险操作",
        {
          confirmButtonText: "确定覆盖",
          cancelButtonText: "取消",
          type: "warning",
        },
      );

      restoring.value = true;
      maintenanceProgress.value = createMaintenanceStatus(
        "正在上传并准备恢复...",
        "processing",
      );
      const formData = new FormData();
      formData.append("file", file);
      await settingsApi.restoreSystem(formData);
      startPolling("restore");
    } catch {
      restoring.value = false;
    } finally {
      target.value = "";
    }
  };

  const handleCleanupLogs = async () => {
    try {
      await ElMessageBox.confirm(
        `确定清理 ${cleanupDays.value} 天前的所有日志？`,
        "提示",
        { confirmButtonText: "确定", cancelButtonText: "取消", type: "warning" },
      );
      cleaningLogs.value = true;
      const res = await settingsApi.cleanupLogs(cleanupDays.value);
      ElMessage.success(`日志清理完成，共清理 ${res.data || 0} 条记录`);
    } catch {
    } finally {
      cleaningLogs.value = false;
    }
  };

  onScopeDispose(clearPolling);

  return {
    backingUp,
    backupOptions,
    cleaningLogs,
    cleanupDays,
    handleBackup,
    handleCleanupLogs,
    handleRestore,
    maintenanceProgress,
    restoring,
    restoreInputRef,
    triggerRestore,
  };
}
