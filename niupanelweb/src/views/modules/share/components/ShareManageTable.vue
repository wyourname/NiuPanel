<template>
  <div class="flex-1 flex flex-col overflow-hidden">
    <ShareManageDesktopTable
      v-if="!appStore.isMobile"
      :data="data"
      :loading="loading"
      @copy-link="emit('copy-link', $event)"
      @delete="emit('delete', $event)"
      @edit="emit('edit', $event)"
      @update-content="emit('update-content', $event)"
    />
    <ShareManageMobileList
      v-else
      :data="data"
      :loading="loading"
      @copy-link="emit('copy-link', $event)"
      @delete="emit('delete', $event)"
      @edit="emit('edit', $event)"
      @update-content="emit('update-content', $event)"
    />
  </div>
</template>

<script setup lang="ts">
import { useAppStore } from "../../../../stores/app";
import type { StationFile } from "@/types";
import ShareManageDesktopTable from "./ShareManageDesktopTable.vue";
import ShareManageMobileList from "./ShareManageMobileList.vue";

defineProps<{
  data: StationFile[];
  loading?: boolean;
}>();

const emit = defineEmits<{
  (event: "copy-link", token: string): void;
  (event: "delete", row: StationFile): void;
  (event: "edit", row: StationFile): void;
  (event: "update-content", row: StationFile): void;
}>();

const appStore = useAppStore();
</script>
