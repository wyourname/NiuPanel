<template>
  <ShareImportSourceDesktopTable
    v-if="!isMobile"
    :groups="groups"
    @copy-url="emit('copy-url', $event)"
    @delete="forwardDelete"
    @update="emit('update', $event)"
  />

  <ShareImportSourceMobileList
    v-else
    :expanded-groups="expandedGroups"
    :groups="groups"
    @copy-url="emit('copy-url', $event)"
    @delete="forwardDelete"
    @toggle-group="toggleGroup"
    @update="emit('update', $event)"
  />
</template>

<script setup lang="ts">
import { ref } from "vue";
import type { ImportSourceGroup } from "@/types";
import ShareImportSourceDesktopTable from "./ShareImportSourceDesktopTable.vue";
import ShareImportSourceMobileList from "./ShareImportSourceMobileList.vue";
import {
  getImportSourceGroupKey,
  type DeleteTargetType,
} from "./shareImportHistoryTypes";

defineProps<{
  groups: ImportSourceGroup[];
  isMobile: boolean;
}>();

const emit = defineEmits<{
  (event: "copy-url", url: string): void;
  (event: "delete", id: number | string, type: DeleteTargetType): void;
  (event: "update", url: string): void;
}>();

const expandedGroups = ref(new Set<string>());

const forwardDelete = (id: number | string, type: DeleteTargetType) => {
  emit("delete", id, type);
};

const toggleGroup = (group: ImportSourceGroup) => {
  const key = getImportSourceGroupKey(group);
  const nextExpandedGroups = new Set(expandedGroups.value);
  if (nextExpandedGroups.has(key)) {
    nextExpandedGroups.delete(key);
  } else {
    nextExpandedGroups.add(key);
  }
  expandedGroups.value = nextExpandedGroups;
};
</script>
