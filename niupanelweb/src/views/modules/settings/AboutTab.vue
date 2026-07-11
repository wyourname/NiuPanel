<template>
  <div class="max-w-[1000px] mx-auto pb-10">
    <AboutOverviewCard
      :checking-update="checkingUpdate"
      :current-version="currentVersion"
      :update-channel="updateChannel"
      :updating-update-channel="updatingUpdateChannel"
      :uploading-update="uploadingUpdate"
      @check-update="handleCheckUpdate"
      @update-channel="handleUpdateChannelChange"
      @upload="triggerUpload"
    />

    <ReleaseManagementPanel />

    <input
      ref="fileInputRef"
      type="file"
      class="hidden"
      accept=".tar.gz"
      @change="handleFileUpload"
    />

    <SystemUpdateDialog
      v-model:visible="updateDialogVisible"
      :can-cancel="canCancel"
      :cancelling-update="cancellingUpdate"
      :executing-update="executingUpdate"
      :title="dialogTitle"
      :width="dialogWidth"
      :update-failed="updateFailed"
      :update-info="updateInfo"
      :update-progress="updateProgress"
      :update-status-message="updateStatusMessage"
      @cancel-update="handleCancelUpdate"
      @force-update="handleForceUpdate"
      @retry-update="retryUpdate"
      @start-update="startUpdate"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../../../stores/app";
import AboutOverviewCard from "./components/AboutOverviewCard.vue";
import SystemUpdateDialog from "./components/SystemUpdateDialog.vue";
import ReleaseManagementPanel from "./components/ReleaseManagementPanel.vue";
import { useSystemUpdate } from "./composables/useSystemUpdate";

const appStore = useAppStore();

const {
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
} = useSystemUpdate();

const dialogWidth = computed(() => {
  if (appStore.isMobile) return "92%";
  return executingUpdate.value || updateFailed.value ? "400px" : "550px";
});
</script>
