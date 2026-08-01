<template>
  <WorkspaceAppFrame
    v-if="!appStore.isMobile"
    content-class="overflow-hidden"
  >
    <template #toolbar>
      <SharePageHeader
        v-model:active-tab="activeTab"
        :is-mobile="appStore.isMobile"
        @open-import="importDialogVisible = true"
        @open-sources="marketSourceDialogVisible = true"
      />
    </template>

    <div v-if="activeTab === 'market'" class="h-full overflow-hidden">
      <ShareMarketTable
        :data="marketScripts"
        :loading="loadingMarket"
        @install="handleInstallFromMarket"
        @open-sources="marketSourceDialogVisible = true"
      />
    </div>

    <div v-if="activeTab === 'import'" class="h-full overflow-hidden">
      <ShareImportHistory ref="historyRef" @update="handleReimport" />
    </div>

    <div v-else-if="activeTab === 'manage'" class="h-full overflow-hidden">
      <ShareStationPanel
        :config-form="configForm"
        :is-mobile="appStore.isMobile"
        :loading-list="loadingList"
        :on-refresh="fetchStationList"
        :saving-config="savingConfig"
        :station-list="stationList"
        :station-stats="stationStats"
        @configure="showStationConfig"
        @copy-link="copyLink"
        @delete="handleDelete"
        @edit="openEditDialog"
        @save-config="handleSaveConfig"
        @update-content="handleUpdateContent"
      />
    </div>
  </WorkspaceAppFrame>

  <div v-else class="flex flex-col h-full bg-base p-4 md:p-3 relative overflow-hidden" :class="[appStore.isMobile ? 'gap-1' : 'gap-4']">
    <div class="flex-1 overflow-hidden relative flex flex-col">
      <div class="w-full max-w-5xl mx-auto flex flex-col h-full" :class="[appStore.isMobile ? '' : 'gap-4']">
        <SharePageHeader
          v-model:active-tab="activeTab"
          :is-mobile="appStore.isMobile"
          @open-import="importDialogVisible = true"
          @open-sources="marketSourceDialogVisible = true"
        />

        <div class="relative flex flex-1 flex-col overflow-hidden bg-card sm:border sm:border-b-0 sm:border-light"
             :class="[appStore.isMobile ? 'mt-2' : '']">
          <div v-if="activeTab === 'market'" class="h-full flex flex-col overflow-hidden bg-subtle dark:bg-subtle">
            <ShareMarketTable
              :data="marketScripts"
              :loading="loadingMarket"
              @install="handleInstallFromMarket"
              @open-sources="marketSourceDialogVisible = true"
            />
          </div>

          <div v-if="activeTab === 'import'" class="h-full flex flex-col overflow-hidden">
            <div class="flex-1 flex flex-col overflow-hidden relative bg-subtle dark:bg-subtle">
              <div class="flex-1 overflow-hidden relative min-h-0">
                <ShareImportHistory ref="historyRef" @update="handleReimport" />
              </div>
            </div>
          </div>

          <div v-else-if="activeTab === 'manage'" class="h-full flex flex-col overflow-hidden">
            <ShareStationPanel
              :config-form="configForm"
              :is-mobile="appStore.isMobile"
              :loading-list="loadingList"
              :on-refresh="fetchStationList"
              :saving-config="savingConfig"
              :station-list="stationList"
              :station-stats="stationStats"
              @configure="showStationConfig"
              @copy-link="copyLink"
              @delete="handleDelete"
              @edit="openEditDialog"
              @save-config="handleSaveConfig"
              @update-content="handleUpdateContent"
            />
          </div>
        </div>
      </div>
    </div>

  </div>

  <MarketSourceDialog
    v-model="marketSourceDialogVisible"
    @update-scripts="fetchMarketScripts"
  />

  <ShareEditDialog
    v-model="editDialogVisible"
    :share="currentShare"
    @success="fetchStationList"
  />

  <ShareImportDialog
    ref="importDialogRef"
    v-model="importDialogVisible"
    :is-mobile="appStore.isMobile"
    @success="handleImportSuccess"
  />
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import WorkspaceAppFrame from "../../../components/workspace/WorkspaceAppFrame.vue";
import { useAppStore } from "../../../stores/app";

import ShareEditDialog from "./components/ShareEditDialog.vue";
import ShareImportHistory from "./components/ShareImportHistory.vue";
import ShareImportDialog from "./components/ShareImportDialog.vue";
import ShareMarketTable from "./components/ShareMarketTable.vue";
import MarketSourceDialog from "./components/MarketSourceDialog.vue";
import SharePageHeader from "./components/SharePageHeader.vue";
import ShareStationPanel from "./components/ShareStationPanel.vue";
import { useShareMarket } from "./composables/useShareMarket";
import { useShareStationManagement } from "./composables/useShareStationManagement";

const appStore = useAppStore();

type ShareTab = "market" | "import" | "manage";

const activeTab = ref<ShareTab>("market");

type ShareImportWizardExpose = {
  setImportUrl: (url: string, isReimport: boolean) => void;
};

type ShareImportHistoryExpose = {
  refresh: () => void;
};

// Refs
const importDialogRef = ref<ShareImportWizardExpose | null>(null);
const historyRef = ref<ShareImportHistoryExpose | null>(null);
const importDialogVisible = ref(false);
const marketSourceDialogVisible = ref(false);

const { fetchMarketScripts, loadingMarket, marketScripts } = useShareMarket();

const handleInstallFromMarket = (url: string) => {
  importDialogVisible.value = true;
  setTimeout(() => {
    importDialogRef.value?.setImportUrl(url, false);
  }, 200);
};

const {
  configForm,
  copyLink,
  currentShare,
  editDialogVisible,
  fetchStationList,
  handleDelete,
  handleSaveConfig,
  handleUpdateContent,
  loadingList,
  openEditDialog,
  savingConfig,
  showStationConfig,
  stationList,
  stationStats,
} = useShareStationManagement();

// Import Actions
const handleImportSuccess = () => {
  importDialogVisible.value = false;
  if (historyRef.value) historyRef.value.refresh();
};

const handleReimport = (url: string) => {
  importDialogVisible.value = true;
  setTimeout(() => {
    importDialogRef.value?.setImportUrl(url, true);
  }, 200);
};

watch(activeTab, (val) => {
  if (val === "market") fetchMarketScripts();
  if (val === "manage") fetchStationList();
  if (val === "import" && historyRef.value) historyRef.value.refresh();
});
onMounted(() => {
  if (activeTab.value === "market") fetchMarketScripts();
  if (activeTab.value === "manage") fetchStationList();
});
</script>
