<template>
  <div
    v-if="isMobile"
    class="grid h-10 shrink-0 grid-flow-col auto-cols-[minmax(92px,1fr)] gap-1 overflow-x-auto border-b border-slate-900/8 bg-slate-950/[0.02] px-2 py-1.5 no-scrollbar dark:border-white/8 dark:bg-white/[0.03]"
  >
    <button
      v-for="item in navGroups"
      :key="item.id"
      type="button"
      class="flex h-7 cursor-pointer items-center justify-center gap-1.5 whitespace-nowrap rounded-md border px-2 text-[11px] font-bold transition-colors"
      :class="
        activeGroup === item.id
          ? 'border-primary/35 bg-white text-default shadow-sm dark:bg-white/10'
          : 'border-transparent text-secondary hover:bg-white/70 dark:hover:bg-white/6'
      "
      @click="emit('update:activeGroup', item.id)"
    >
      <div :class="item.icon" class="text-sm"></div>
      <span>{{ item.label.replace("管理", "") }}</span>
    </button>
  </div>

  <div
    v-else
    class="flex w-[184px] shrink-0 flex-col gap-1 border-r border-slate-900/8 bg-slate-950/[0.018] p-2 dark:border-white/8 dark:bg-white/[0.025]"
  >
    <button
      v-for="item in navGroups"
      :key="item.id"
      type="button"
      class="grid h-9 cursor-pointer grid-cols-[18px_minmax(0,1fr)_auto] items-center gap-2 rounded-md border px-2.5 text-left transition-colors"
      :class="
        activeGroup === item.id
          ? 'border-primary/35 bg-white text-default shadow-sm dark:bg-white/8'
          : 'border-transparent text-secondary hover:bg-white/70 dark:hover:bg-white/6'
      "
      @click="emit('update:activeGroup', item.id)"
    >
      <div
        :class="item.icon"
        class="text-[15px]"
      ></div>
      <span class="truncate text-[12px] font-bold">{{ item.label }}</span>
      <span
        v-if="selectedCount(item.id) > 0"
        class="rounded-md bg-slate-950/6 px-1.5 py-0.5 text-[10px] font-bold text-secondary dark:bg-white/8 dark:text-white/70"
      >
        {{ selectedCountLabel(item.id) }}
      </span>
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import {
  getPermissionGroup,
  navGroups,
  type ApiPermissionNavGroupId,
} from "../utils/apiKeyPermissions";

const props = defineProps<{
  activeGroup: ApiPermissionNavGroupId;
  isMobile: boolean;
  selectedPerms: string[];
}>();

const emit = defineEmits<{
  (event: "update:activeGroup", group: ApiPermissionNavGroupId): void;
}>();

const selectedSet = computed(() => new Set(props.selectedPerms));

const selectedCount = (group: ApiPermissionNavGroupId) => {
  if (group === "all") return selectedSet.value.has("*:*") ? 1 : 0;
  const meta = getPermissionGroup(group);
  if (!meta) return 0;
  return meta.perms.filter((permission) =>
    selectedSet.value.has(permission.value),
  ).length;
};

const selectedCountLabel = (group: ApiPermissionNavGroupId) => {
  if (group === "all") return "已开";
  return selectedCount(group);
};
</script>

<style scoped>
.no-scrollbar::-webkit-scrollbar {
  display: none;
}
</style>
