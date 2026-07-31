<template>
  <WorkspaceAppFrame v-if="!appStore.isMobile" content-class="overflow-hidden">
    <div class="h-full overflow-y-auto bg-base px-4 py-4 custom-scrollbar">
      <ExtensionManager />
    </div>
  </WorkspaceAppFrame>

  <PageShell v-else :padded="false">
    <div class="flex h-full min-h-0 flex-col overflow-hidden bg-base">
      <div class="mobile-dock-safe min-h-0 flex-1 overflow-y-auto px-3 py-3 custom-scrollbar">
        <ExtensionManager />
      </div>
    </div>
  </PageShell>
</template>

<script setup lang="ts">
import PageShell from "@/components/common/PageShell.vue";
import WorkspaceAppFrame from "@/components/workspace/WorkspaceAppFrame.vue";
import { useAppStore } from "@/stores/app";
import ExtensionManager from "./components/ExtensionManager.vue";

const appStore = useAppStore();
</script>
