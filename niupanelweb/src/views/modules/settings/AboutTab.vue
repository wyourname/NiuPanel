<template>
  <div class="max-w-[1000px] mx-auto pb-10">
    <AboutOverviewCard
      :checking-update="checkingUpdate"
      :current-version="currentVersion"
      :update-channel="updateChannel"
      :updating-update-channel="updatingUpdateChannel"
      @check-update="handleCheckUpdate"
      @update-channel="handleUpdateChannelChange"
    />

    <ReleaseManagementPanel />

    <SystemUpdateDialog
      v-model:visible="updateDialogVisible"
      :can-cancel="canCancel"
      :cancelling-update="cancellingUpdate"
      :executing-update="executingUpdate"
      :title="dialogTitle"
      :update-failed="updateFailed"
      :update-info="updateInfo"
      :update-progress="updateProgress"
      :update-status-message="updateStatusMessage"
      @cancel-update="handleCancelUpdate"
      @retry-update="retryUpdate"
      @start-update="startUpdate"
    />
  </div>
</template>

<script setup lang="ts">
import AboutOverviewCard from "./components/AboutOverviewCard.vue";
import SystemUpdateDialog from "./components/SystemUpdateDialog.vue";
import ReleaseManagementPanel from "./components/ReleaseManagementPanel.vue";
import { useSystemUpdate } from "./composables/useSystemUpdate";

const {
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
} = useSystemUpdate();

</script>
