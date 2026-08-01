<template>
  <div class="min-h-0 flex-1 overflow-y-auto p-3 custom-scrollbar">
    <div
      v-if="activeGroup === 'all'"
      class="space-y-3"
    >
      <div
        role="button"
        tabindex="0"
        class="flex cursor-pointer items-center justify-between gap-4 rounded-md border px-4 py-3 text-left transition-colors"
        :class="
          selectedPerms.includes('*:*')
            ? 'border-rose-400/45 bg-rose-50/75 dark:border-rose-400/30 dark:bg-rose-500/10'
            : 'border-slate-900/8 bg-slate-950/[0.018] hover:bg-white/70 dark:border-white/8 dark:bg-white/[0.025] dark:hover:bg-white/6'
        "
        @click="emit('toggle-super-user')"
        @keydown.enter.prevent="emit('toggle-super-user')"
        @keydown.space.prevent="emit('toggle-super-user')"
      >
        <div class="flex min-w-0 items-center gap-3">
          <div
            class="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border"
            :class="
              selectedPerms.includes('*:*')
                ? 'border-rose-300/60 bg-white/80 text-rose-600 dark:border-rose-400/30 dark:bg-white/8 dark:text-rose-300'
                : 'border-slate-900/8 bg-white/70 text-secondary dark:border-white/8 dark:bg-white/6'
            "
          >
            <div class="i-ep-lock text-base"></div>
          </div>
          <div class="min-w-0">
            <div class="text-sm font-bold text-default">最高权限</div>
            <div class="mt-0.5 text-[11px] text-secondary">
              完整授权会覆盖所有细分权限。
            </div>
          </div>
        </div>
        <el-checkbox
          :model-value="selectedPerms.includes('*:*')"
          size="large"
          @click.stop="emit('toggle-super-user')"
        />
      </div>

      <div class="rounded-md border border-slate-900/8 bg-slate-950/[0.018] px-4 py-3 text-[12px] leading-relaxed text-secondary dark:border-white/8 dark:bg-white/[0.025]">
        MCP 与 HTTP API 共用这套权限。建议只给外部集成所需的最小权限；需要全量 SDK 管理时再启用最高权限。
      </div>
    </div>

    <div v-else-if="currentGroupMeta" class="min-h-full">
      <div class="mb-2 flex h-9 items-center justify-between gap-3">
        <div class="flex min-w-0 items-center gap-2">
          <div
            class="flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-white"
            :class="currentGroupMeta.color"
          >
            <div :class="currentGroupMeta.icon" class="text-sm"></div>
          </div>
          <div class="min-w-0">
            <h4 class="truncate text-[13px] font-bold text-default">
              {{ currentGroupMeta.title }}
            </h4>
          </div>
        </div>

        <div
          role="button"
          tabindex="0"
          class="flex h-7 cursor-pointer items-center gap-1.5 rounded-md border border-slate-900/8 bg-white/70 px-2 text-[11px] font-bold text-secondary transition-colors hover:border-primary/30 hover:text-default dark:border-white/8 dark:bg-white/6"
          @click="emit('group-select-all')"
          @keydown.enter.prevent="emit('group-select-all')"
          @keydown.space.prevent="emit('group-select-all')"
        >
          <span>{{ currentGroupSelectedCount }}/{{ currentGroupTotal }}</span>
          <span>全选</span>
          <el-checkbox
            :model-value="isGroupAllSelected"
            :indeterminate="isGroupIndeterminate"
            @click.stop="emit('group-select-all')"
          />
        </div>
      </div>

      <el-checkbox-group
        :model-value="selectedPerms"
        class="grid grid-cols-2 gap-2 pb-3"
        @update:model-value="handleSelectedPermsChange"
      >
        <div
          v-for="perm in currentGroupMeta.perms"
          :key="perm.value"
          class="flex min-h-[48px] cursor-pointer items-center rounded-md border px-3 py-2 transition-colors"
          :class="
            isPermissionSelected(perm.value)
              ? 'border-sky-300/80 bg-sky-50 shadow-sm dark:border-sky-400/35 dark:bg-sky-400/10'
              : 'border-slate-200 bg-white/86 hover:border-slate-300 hover:bg-white dark:border-white/10 dark:bg-[#101b29] dark:hover:bg-[#142235]'
          "
          @click="emit('toggle-perm', perm.value)"
        >
          <el-checkbox
            :value="perm.value"
            class="api-permission-checkbox min-w-0 flex-1 !mr-0"
            @click.stop
          >
            <span class="flex min-w-0 flex-col">
              <span
                class="truncate text-[12px] font-bold"
                :class="
                  isPermissionSelected(perm.value)
                    ? 'text-slate-950 dark:text-sky-50'
                    : 'text-slate-900 dark:text-slate-100'
                "
              >
                {{ perm.label }}
              </span>
              <span
                class="mt-0.5 truncate font-mono text-[10px]"
                :class="
                  isPermissionSelected(perm.value)
                    ? 'text-sky-700 dark:text-sky-200'
                    : 'text-slate-600 dark:text-slate-400'
                "
              >
                {{ perm.value }}
              </span>
              <span
                v-if="mcpToolCount(perm.value)"
                class="mt-0.5 truncate text-[10px] text-emerald-700 dark:text-emerald-300"
              >
                MCP 可调用 {{ mcpToolCount(perm.value) }} 个工具
              </span>
            </span>
          </el-checkbox>
        </div>
      </el-checkbox-group>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { McpToolInfo } from "@/types";
import { getMcpToolCountForPermission } from "../utils/mcpPermissions";
import {
  getPermissionGroup,
  type ApiPermissionNavGroupId,
} from "../utils/apiKeyPermissions";

const props = defineProps<{
  activeGroup: ApiPermissionNavGroupId;
  isGroupAllSelected: boolean;
  isGroupIndeterminate: boolean;
  mcpTools: McpToolInfo[];
  selectedPerms: string[];
}>();

const emit = defineEmits<{
  (event: "group-select-all"): void;
  (event: "toggle-perm", permission: string): void;
  (event: "toggle-super-user"): void;
  (event: "update:selectedPerms", permissions: string[]): void;
}>();

const currentGroupMeta = computed(() => getPermissionGroup(props.activeGroup));

const currentGroupTotal = computed(() => currentGroupMeta.value?.perms.length ?? 0);

const currentGroupSelectedCount = computed(() => {
  const meta = currentGroupMeta.value;
  if (!meta) return 0;
  return meta.perms.filter((permission) =>
    props.selectedPerms.includes(permission.value),
  ).length;
});

const isPermissionSelected = (permission: string) =>
  props.selectedPerms.includes(permission);

const mcpToolCount = (permission: string) =>
  getMcpToolCountForPermission(permission, props.mcpTools);

const handleSelectedPermsChange = (permissions: unknown) => {
  emit(
    "update:selectedPerms",
    Array.isArray(permissions)
      ? permissions.filter(
          (permission): permission is string => typeof permission === "string",
        )
      : [],
  );
};
</script>

<style scoped>
:deep(.api-permission-checkbox) {
  height: auto;
}

:deep(.api-permission-checkbox .el-checkbox__label) {
  min-width: 0;
  flex: 1;
}
</style>
