import { computed, onMounted, onScopeDispose, ref } from "vue";
import { ElMessage } from "element-plus";
import * as settingsApi from "@/api/settings";
import type { SettingItem, UpdateChannel, UpdateInfo } from "@/types";

const RELOAD_DELAY_MS = 5000;
const POLL_INTERVAL_MS = 1000;

const getErrorMessage = (error: unknown, fallback: string) =>
  error instanceof Error ? error.message : fallback;

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
  const updateChannel = ref<UpdateChannel>("stable");
  const updatingUpdateChannel = ref(false);
  const updateProgress = ref(0);

  let updatePollTimer: ReturnType<typeof setInterval> | null = null;

  const dialogTitle = computed(() => {
    if (updateFailed.value) return "更新失败";
    if (executingUpdate.value) return "正在更新 Panel";
    return updateInfo.value?.update_available ? "发现新的 Panel 版本" : "Panel 版本详情";
  });

  const channelLabel = computed(() =>
    updateChannel.value === "preview" ? "预览版本" : "正式版本",
  );

  const canCancel = computed(() => {
    return executingUpdate.value
      && currentState.value === "Downloading";
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
    if (!updateInfo.value?.launcher_managed) {
      ElMessage.warning("当前为直接启动模式；请改用 niupanel-launcher，或通过更新 Docker 镜像完成 Panel 更新。");
      return;
    }
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

  const retryUpdate = () => {
    updateFailed.value = false;
    void startUpdate();
  };

  const handleCancelUpdate = async () => {
    cancellingUpdate.value = true;
    try {
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
      ElMessage.success(
        `已切换到${channel === "preview" ? "预览" : "正式"}通道；不会自动安装更新`,
      );
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
    handleCancelUpdate,
    handleCheckUpdate,
    handleUpdateChannelChange,
    retryUpdate,
    startUpdate,
    updateChannel,
    updateDialogVisible,
    updateFailed,
    updateInfo,
    updateProgress,
    updateStatusMessage,
    updatingUpdateChannel,
  };
}

const findSettingValue = (settings: SettingItem[], key: string) =>
  settings.find((item) => item.key === key)?.value;
