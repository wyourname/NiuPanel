<template>
  <div
    class="flex items-center gap-3 cursor-pointer select-none py-1 group/name overflow-hidden w-full flex-1"
  >
    <div
      :class="[getIconClass(row)]"
      class="text-[22px] shrink-0 transition-transform group-hover/name:scale-110"
    ></div>
    <span
      class="truncate text-[13px] font-bold text-default transition-colors group-hover/name:text-primary"
    >
      {{ row.name }}
    </span>
  </div>
</template>

<script setup lang="ts">
import type { FileItem } from "../../../../composables/useFileOperations";

defineProps<{
  getIconClass: (row: FileItem) => string;
  row: FileItem;
}>();
</script>
