import { onMounted, onScopeDispose, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import * as settingsApi from "@/api/settings";
import { createUploadFormData } from "@/api/upload";
import { useUploadTransfer } from "@/composables/useUploadTransfer";
import type {
  BackupOptions,
  LogCleanupReport,
  MaintenanceStatus,
} from "@/types";

const POLL_INTERVAL_MS = 1000;
const RESTORE_RELOAD_DELAY_MS = 1500;
const DEFAULT_LOG_RETENTION_DAYS = 15;
const MAX_LOG_RETENTION_DAYS = 365;

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
  const previewingLogs = ref(false);
  const cleanupDays = ref(DEFAULT_LOG_RETENTION_DAYS);
  const logCleanupReport = ref<LogCleanupReport | null>(null);
  const restoreInputRef = ref<HTMLInputElement | null>(null);
  const {
    cancel: cancelRestoreUpload,
    run: runRestoreUpload,
    uploading: uploadingRestore,
  } = useUploadTransfer();

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

    if (
      backupOptions.value.variables ||
      backupOptions.value.settings ||
      backupOptions.value.telegram
    ) {
      try {
        await ElMessageBox.confirm(
          "备份包将包含变量、系统设置或 Telegram 配置的敏感明文。请仅保存到受信任位置，并避免上传到公开网盘。",
          "备份包含敏感数据",
          {
            confirmButtonText: "继续备份",
            cancelButtonText: "取消",
            type: "warning",
          },
        );
      } catch {
        return;
      }
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
    } catch {
      target.value = "";
      return;
    }

    restoring.value = true;
    maintenanceProgress.value = createMaintenanceStatus(
      "正在上传备份包...",
      "processing",
    );
    const formData = createUploadFormData([["file", file]]);
    try {
      const result = await runRestoreUpload(
        (options) => settingsApi.restoreSystem(formData, options),
        {
          initialTotalBytes: file.size,
          onProgress: (progress) => {
            maintenanceProgress.value = createMaintenanceStatus(
              "正在上传备份包...",
              "processing",
            );
            maintenanceProgress.value.progress = progress.percentage;
          },
        },
      );
      if (result.cancelled) {
        restoring.value = false;
        maintenanceProgress.value = createMaintenanceStatus("备份包上传已取消");
        ElMessage.info("备份包上传已取消");
        return;
      }
      maintenanceProgress.value = createMaintenanceStatus(
        "上传完成，正在准备恢复...",
        "processing",
      );
      maintenanceProgress.value.progress = 100;
      startPolling("restore");
    } catch (error) {
      restoring.value = false;
      ElMessage.error(error instanceof Error ? error.message : "备份包上传失败");
    } finally {
      target.value = "";
    }
  };

  const previewCleanupLogs = async () => {
    previewingLogs.value = true;
    try {
      const { data } = await settingsApi.cleanupLogs(cleanupDays.value, true);
      logCleanupReport.value = data;
      return data;
    } catch {
      ElMessage.error("无法生成日志清理预览");
      return null;
    } finally {
      previewingLogs.value = false;
    }
  };

  const handleCleanupLogs = async () => {
    const preview =
      logCleanupReport.value?.dry_run === true
        ? logCleanupReport.value
        : await previewCleanupLogs();
    if (!preview) return;

    const databaseRecords =
      preview.task_runs + preview.system_jobs + preview.audit_logs;
    if (preview.files + databaseRecords === 0) {
      ElMessage.info("当前没有符合条件的日志");
      return;
    }

    try {
      await ElMessageBox.confirm(
        `将永久删除预览中的 ${preview.files} 个文件和 ${databaseRecords} 条记录，正在运行的任务不会受到影响。`,
        "确认清理日志",
        {
          confirmButtonText: "确认清理",
          cancelButtonText: "取消",
          type: "warning",
        },
      );
      cleaningLogs.value = true;
      const { data } = await settingsApi.cleanupLogs(cleanupDays.value);
      logCleanupReport.value = data;
      const cleanedRecords =
        data.task_runs + data.system_jobs + data.audit_logs;
      ElMessage.success(
        `日志清理完成：${data.files} 个文件，${cleanedRecords} 条记录`,
      );
    } catch (error) {
      if (error !== "cancel" && error !== "close") {
        ElMessage.error("日志清理失败，请检查服务日志");
      }
    } finally {
      cleaningLogs.value = false;
    }
  };

  watch(cleanupDays, () => {
    logCleanupReport.value = null;
  });

  onMounted(async () => {
    try {
      const { data } = await settingsApi.getSettings();
      const configured = Number(
        data.find((item) => item.key === "system.log_retention_days")?.value,
      );
      if (
        Number.isInteger(configured) &&
        configured >= 1 &&
        configured <= MAX_LOG_RETENTION_DAYS
      ) {
        cleanupDays.value = configured;
      }
    } catch {
      // 保留安全的默认值，设置页仍可正常使用。
    }
  });

  onScopeDispose(() => {
    clearPolling();
  });

  return {
    backingUp,
    backupOptions,
    cancelRestoreUpload,
    cleaningLogs,
    cleanupDays,
    handleBackup,
    handleCleanupLogs,
    handleRestore,
    logCleanupReport,
    maintenanceProgress,
    previewCleanupLogs,
    previewingLogs,
    restoring,
    restoreInputRef,
    triggerRestore,
    uploadingRestore,
  };
}
