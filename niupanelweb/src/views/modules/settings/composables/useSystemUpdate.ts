import { computed, onMounted, onScopeDispose, ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import * as settingsApi from "@/api/settings";
import { createUploadFormData } from "@/api/upload";
import { useUploadTransfer } from "@/composables/useUploadTransfer";
import type { SettingItem, UpdateChannel, UpdateInfo } from "@/types";

const RELOAD_DELAY_MS = 5000;
const POLL_INTERVAL_MS = 1000;

const getErrorMessage = (error: unknown, fallback: string) =>
  error instanceof Error ? error.message : fallback;

const createFallbackUpdateInfo = (channel: UpdateChannel): UpdateInfo => ({
  body: "",
  channel,
  prerelease: channel === "preview",
  size: 0,
  tag_name: "Latest",
  update_available: true,
});

export function useSystemUpdate() {
  const currentVersion = ref("");
  const checkingUpdate = ref(false);
  const updateInfo = ref<UpdateInfo | null>(null);
  const updateDialogVisible = ref(false);
  const executingUpdate = ref(false);
  const updateStatusMessage = ref("");
  const cancellingUpdate = ref(false);
  const currentState = ref("");
  const updateFailed = ref(false);
  const fileInputRef = ref<HTMLInputElement | null>(null);
  const updateChannel = ref<UpdateChannel>("stable");
  const updatingUpdateChannel = ref(false);
  const {
    cancel: cancelUpdateUpload,
    progress: updateProgress,
    run: runUpdateUpload,
    uploading: uploadingUpdate,
  } = useUploadTransfer();

  let updatePollTimer: ReturnType<typeof setInterval> | null = null;

  const dialogTitle = computed(() => {
    if (updateFailed.value) return "更新失败";
    if (executingUpdate.value) return "正在更新 Core";
    return updateInfo.value?.update_available ? "发现新的 Core 版本" : "Core 版本详情";
  });

  const channelLabel = computed(() =>
    updateChannel.value === "preview" ? "预览版本" : "正式版本",
  );

  const canCancel = computed(() => {
    return executingUpdate.value
      && (uploadingUpdate.value || currentState.value === "Downloading");
  });

  const clearUpdatePollTimer = () => {
    if (updatePollTimer) clearInterval(updatePollTimer);
    updatePollTimer = null;
  };

  const scheduleReload = () => {
    setTimeout(() => window.location.reload(), RELOAD_DELAY_MS);
  };

  const handleCheckUpdate = async () => {
    checkingUpdate.value = true;
    try {
      const res = await settingsApi.checkUpdate();
      updateInfo.value = res.data;
      if (res.data.channel === "stable" || res.data.channel === "preview") {
        updateChannel.value = res.data.channel;
      }
      updateDialogVisible.value = true;
      executingUpdate.value = false;
      updateFailed.value = false;
      if (res.data.update_available) {
        ElMessage.success(`发现新的${channelLabel.value}`);
      }
    } finally {
      checkingUpdate.value = false;
    }
  };

  const startPollingUpdateStatus = () => {
    clearUpdatePollTimer();
    updatePollTimer = setInterval(async () => {
      try {
        const res = await settingsApi.getUpdateStatus();
        currentState.value = res.data.state;
        updateStatusMessage.value = res.data.message || res.data.state;

        if (res.data.state === "Downloading") {
          updateProgress.value = res.data.progress;
        } else if (res.data.state === "Restarting") {
          updateProgress.value = 100;
          clearUpdatePollTimer();
          scheduleReload();
        } else if (res.data.state === "Installing") {
          updateProgress.value = 100;
        } else if (res.data.state === "Error") {
          clearUpdatePollTimer();
          executingUpdate.value = false;
          updateFailed.value = true;
          updateStatusMessage.value = res.data.error || res.data.message || "更新失败";
        } else if (res.data.state === "Idle" && executingUpdate.value) {
          clearUpdatePollTimer();
          executingUpdate.value = false;
        }
      } catch {
        if (currentState.value === "Restarting") {
          clearUpdatePollTimer();
          scheduleReload();
        }
      }
    }, POLL_INTERVAL_MS);
  };

  const startUpdate = async () => {
    executingUpdate.value = true;
    updateStatusMessage.value = "正在初始化...";
    updateProgress.value = 0;

    try {
      await settingsApi.executeUpdate();
      startPollingUpdateStatus();
    } catch {
      executingUpdate.value = false;
      updateFailed.value = true;
      updateStatusMessage.value = "请求更新失败";
    }
  };

  const handleForceUpdate = () => {
    ElMessageBox.confirm(
      "将从 GitHub 下载当前通道的 Core 包并交给 launcher 重新安装。确定继续吗？",
      "重新安装 Core",
      {
        confirmButtonText: "确定更新",
        cancelButtonText: "取消",
        type: "warning",
      },
    )
      .then(() => {
        if (!updateInfo.value) {
          updateInfo.value = createFallbackUpdateInfo(updateChannel.value);
        }
        void startUpdate();
      })
      .catch(() => {});
  };

  const retryUpdate = () => {
    updateFailed.value = false;
    void startUpdate();
  };

  const triggerUpload = () => {
    fileInputRef.value?.click();
  };

  const handleFileUpload = async (event: Event) => {
    const target = event.target;
    if (!(target instanceof HTMLInputElement)) return;
    if (!target.files || target.files.length === 0) return;

    const file = target.files[0];
    target.value = "";

    updateDialogVisible.value = true;
    executingUpdate.value = true;
    updateFailed.value = false;
    updateStatusMessage.value = "正在上传 Core 包...";
    updateProgress.value = 0;
    cancellingUpdate.value = false;
    currentState.value = "Uploading";

    try {
      const formData = createUploadFormData([["file", file]]);
      const result = await runUpdateUpload(
        (options) => settingsApi.uploadUpdate(formData, options),
        { initialTotalBytes: file.size },
      );
      if (result.cancelled) {
        executingUpdate.value = false;
        updateFailed.value = false;
        updateProgress.value = 0;
        updateStatusMessage.value = "Core 包上传已取消";
        ElMessage.info("Core 包上传已取消");
        return;
      }

      startPollingUpdateStatus();
    } catch (error: unknown) {
      executingUpdate.value = false;
      updateFailed.value = true;
      updateStatusMessage.value = getErrorMessage(error, "上传 Core 包失败");
    }
  };

  const handleCancelUpdate = async () => {
    cancellingUpdate.value = true;
    try {
      if (uploadingUpdate.value) {
        cancelUpdateUpload();
        return;
      }
      await settingsApi.cancelUpdate();
      ElMessage.info("正在请求取消...");
    } catch (error: unknown) {
      ElMessage.error(getErrorMessage(error, "取消失败"));
    } finally {
      cancellingUpdate.value = false;
    }
  };

  const loadVersion = async () => {
    try {
      const res = await settingsApi.getVersion();
      currentVersion.value = res.data;
    } catch {
    }
  };

  const loadUpdateChannel = async () => {
    try {
      const res = await settingsApi.getSettings();
      const channel = findSettingValue(res.data || [], "system.update_channel");
      if (channel === "stable" || channel === "preview") {
        updateChannel.value = channel;
      }
    } catch {
    }
  };

  const handleUpdateChannelChange = async (channel: UpdateChannel) => {
    if (channel === updateChannel.value) return;
    const previous = updateChannel.value;
    updateChannel.value = channel;
    updateInfo.value = null;
    updateDialogVisible.value = false;
    updatingUpdateChannel.value = true;
    try {
      await settingsApi.updateUpdateChannel(channel);
      ElMessage.success(`已切换到${channel === "preview" ? "预览版本" : "正式版本"}`);
    } catch (error: unknown) {
      updateChannel.value = previous;
      ElMessage.error(getErrorMessage(error, "更新通道切换失败"));
    } finally {
      updatingUpdateChannel.value = false;
    }
  };

  onMounted(() => {
    void loadVersion();
    void loadUpdateChannel();
  });

  onScopeDispose(() => {
    clearUpdatePollTimer();
  });

  return {
    canCancel,
    cancellingUpdate,
    checkingUpdate,
    currentVersion,
    dialogTitle,
    executingUpdate,
    fileInputRef,
    handleCancelUpdate,
    handleCheckUpdate,
    handleFileUpload,
    handleForceUpdate,
    handleUpdateChannelChange,
    retryUpdate,
    startUpdate,
    triggerUpload,
    updateChannel,
    updateDialogVisible,
    updateFailed,
    updateInfo,
    updateProgress,
    updateStatusMessage,
    updatingUpdateChannel,
    uploadingUpdate,
  };
}

const findSettingValue = (settings: SettingItem[], key: string) =>
  settings.find((item) => item.key === key)?.value;
