<template>
  <WorkspaceAppFrame
    v-if="!appStore.isMobile"
    content-class="overflow-hidden"
  >
    <template #toolbar>
      <TelegramToolbar
        v-model:active-tab="activeTab"
        :enabled="config.enabled"
        :is-mobile="appStore.isMobile"
        :latency="latency"
        :tab-items="tabItems"
        @file-action="handleFileAction"
        @open-settings="showSettings = true"
      />
    </template>

    <div class="h-full overflow-hidden px-3 py-3">
      <TelegramCommandsTab
        v-if="activeTab === 'commands'"
        :commands="commandsList"
        :is-mobile="appStore.isMobile"
        @create="openCmdDialog()"
        @delete="handleDeleteCmd"
        @edit="openCmdDialog"
      />
      <TelegramWorkflowsTab
        v-else-if="activeTab === 'workflows'"
        :action-type-label="actionTypeLabel"
        :event-type-label="eventTypeLabel"
        :is-mobile="appStore.isMobile"
        :workflows="workflowsList"
        @create="openWfDialog()"
        @delete="handleDeleteWf"
        @edit="openWfDialog"
      />
      <div v-else class="h-full flex-center text-[13px] font-bold text-muted">
        未选择项目
      </div>
    </div>
  </WorkspaceAppFrame>

  <div
    v-else
    class="flex flex-col h-full p-4 md:p-6"
    :class="[appStore.isMobile ? 'gap-1' : 'gap-4']"
  >
    <div
      class="w-full max-w-5xl mx-auto flex flex-col h-full"
      :class="[appStore.isMobile ? 'gap-1' : 'gap-4']"
    >
      <TelegramToolbar
        v-model:active-tab="activeTab"
        :enabled="config.enabled"
        :is-mobile="appStore.isMobile"
        :latency="latency"
        :tab-items="tabItems"
        @file-action="handleFileAction"
        @open-settings="showSettings = true"
      />

      <div class="flex-1 overflow-hidden">
        <TelegramCommandsTab
          v-if="activeTab === 'commands'"
          :commands="commandsList"
          :is-mobile="appStore.isMobile"
          @create="openCmdDialog()"
          @delete="handleDeleteCmd"
          @edit="openCmdDialog"
        />
        <TelegramWorkflowsTab
          v-else-if="activeTab === 'workflows'"
          :action-type-label="actionTypeLabel"
          :event-type-label="eventTypeLabel"
          :is-mobile="appStore.isMobile"
          :workflows="workflowsList"
          @create="openWfDialog()"
          @delete="handleDeleteWf"
          @edit="openWfDialog"
        />
        <div v-else class="h-full flex-center p-10 text-center">
          <div class="max-w-sm">
            <div class="i-ep-info-filled text-4xl text-muted opacity-20 mb-4 mx-auto"></div>
            <p class="text-xs text-muted leading-relaxed">
              请选择上方标签页进行管理。机器人支持实时指令处理、自动化工作流触发以及文件双向传输。
            </p>
          </div>
        </div>
      </div>
    </div>
  </div>

  <TelegramCommandDialog
    v-model:form="cmdForm"
    v-model:visible="showCmdDialog"
    :is-mobile="appStore.isMobile"
    @save="saveCmd"
  />

  <TelegramWorkflowDialog
    v-model:form="wfForm"
    v-model:visible="showWfDialog"
    :action-config-label="actionConfigLabel"
    :action-config-placeholder="actionConfigPlaceholder"
    :is-mobile="appStore.isMobile"
    @save="saveWf"
  />

  <input
    ref="localFileInput"
    type="file"
    class="hidden"
    @change="onLocalFileChange"
  />

  <TelegramServerFileDialog
    v-model:visible="showServerFileSelector"
    :current-path="currentPath"
    :format-size="formatSize"
    :get-file-icon-class="getFileIconClass"
    :is-mobile="appStore.isMobile"
    :server-files="serverFiles"
    @back="goBack"
    @row-click="handleFileRowClick"
  />

  <TelegramSettingsDrawer
    v-model:form="form"
    v-model:visible="showSettings"
    :is-mobile="appStore.isMobile"
    :saving="saving"
    :testing="testing"
    @save="handleSave"
    @test="handleTest"
  />
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import WorkspaceAppFrame from "../../../components/workspace/WorkspaceAppFrame.vue";
import { useAppStore } from "../../../stores/app";
import TelegramCommandDialog from "./components/TelegramCommandDialog.vue";
import TelegramCommandsTab from "./components/TelegramCommandsTab.vue";
import TelegramServerFileDialog from "./components/TelegramServerFileDialog.vue";
import TelegramSettingsDrawer from "./components/TelegramSettingsDrawer.vue";
import TelegramToolbar, {
  type TelegramTabItem,
} from "./components/TelegramToolbar.vue";
import TelegramWorkflowDialog from "./components/TelegramWorkflowDialog.vue";
import TelegramWorkflowsTab from "./components/TelegramWorkflowsTab.vue";
import { useTelegramCommands } from "./composables/useTelegramCommands";
import { useTelegramConfig } from "./composables/useTelegramConfig";
import { useTelegramFileTransfer } from "./composables/useTelegramFileTransfer";
import { useTelegramWorkflows } from "./composables/useTelegramWorkflows";

const appStore = useAppStore();

const tabItems = [
  { label: "指令", value: "commands" },
  { label: "自动化", value: "workflows" },
] as const satisfies readonly TelegramTabItem[];

type TelegramTab = (typeof tabItems)[number]["value"];

const activeTab = ref<TelegramTab>("commands");

const {
  config,
  form,
  handleSave,
  handleTest,
  latency,
  loadConfig,
  refreshLatency,
  saving,
  showSettings,
  testing,
} = useTelegramConfig();

const {
  cmdForm,
  commandsList,
  handleDeleteCmd,
  loadCommands,
  openCmdDialog,
  saveCmd,
  showCmdDialog,
} = useTelegramCommands();

const {
  actionConfigLabel,
  actionConfigPlaceholder,
  actionTypeLabel,
  eventTypeLabel,
  handleDeleteWf,
  loadWorkflows,
  openWfDialog,
  saveWf,
  showWfDialog,
  wfForm,
  workflowsList,
} = useTelegramWorkflows();

const {
  currentPath,
  formatSize,
  getFileIconClass,
  goBack,
  handleFileAction,
  handleFileRowClick,
  localFileInput,
  onLocalFileChange,
  serverFiles,
  showServerFileSelector,
} = useTelegramFileTransfer({
  getAdminChatId: () => config.admin_chat_id,
});

let latencyTimer: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  void Promise.all([
    loadConfig(),
    loadCommands(),
    loadWorkflows(),
    refreshLatency(),
  ]);
  latencyTimer = setInterval(() => {
    void refreshLatency();
  }, 30000);
});

onUnmounted(() => {
  if (latencyTimer) clearInterval(latencyTimer);
});
</script>
