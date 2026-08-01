<template>
  <ResponsiveDialog
    v-model:visible="visibleValue"
    :title="isEdit ? '编辑 API 访问密钥' : '配置 API 访问密钥'"
    width="800px"
    append-to-body
    destroy-on-close
    custom-class="cloud-dialog"
  >
    <div class="flex h-full flex-col overflow-hidden bg-white dark:bg-[#0d1724] md:h-[520px]">
      <ApiKeyBasicFields
        :form="form"
        :is-edit="isEdit"
        @update:expires-at="form.expires_at = $event"
        @update:expires-in-days="form.expires_in_days = $event"
        @update:name="form.name = $event"
      />

      <div class="flex min-h-0 flex-1 flex-col overflow-hidden md:flex-row">
        <ApiKeyPermissionNav
          :active-group="activeGroup"
          :is-mobile="isMobile"
          :selected-perms="selectedPerms"
          @update:active-group="emit('update:activeGroup', $event)"
        />

        <ApiKeyPermissionPanel
          :active-group="activeGroup"
          :is-group-all-selected="isGroupAllSelected"
          :is-group-indeterminate="isGroupIndeterminate"
          :mcp-tools="mcpTools"
          :selected-perms="selectedPerms"
          @group-select-all="emit('group-select-all')"
          @toggle-perm="emit('toggle-perm', $event)"
          @toggle-super-user="emit('toggle-super-user')"
          @update:selected-perms="emit('update:selectedPerms', $event)"
        />
      </div>

      <ApiKeyPermissionFooter
        :is-edit="isEdit"
        :is-mobile="isMobile"
        :selected-count="selectedPerms.length"
        :submitting="submitting"
        @cancel="visibleValue = false"
        @submit="emit('submit')"
      />
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";
import type { McpToolInfo } from "@/types";
import ApiKeyBasicFields from "./ApiKeyBasicFields.vue";
import ApiKeyPermissionFooter from "./ApiKeyPermissionFooter.vue";
import ApiKeyPermissionNav from "./ApiKeyPermissionNav.vue";
import ApiKeyPermissionPanel from "./ApiKeyPermissionPanel.vue";
import {
  type ApiKeyFormState,
  type ApiPermissionNavGroupId,
} from "../utils/apiKeyPermissions";

const props = defineProps<{
  activeGroup: ApiPermissionNavGroupId;
  form: ApiKeyFormState;
  isEdit: boolean;
  isMobile: boolean;
  isGroupAllSelected: boolean;
  isGroupIndeterminate: boolean;
  mcpTools: McpToolInfo[];
  selectedPerms: string[];
  submitting: boolean;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "group-select-all"): void;
  (event: "submit"): void;
  (event: "toggle-perm", permission: string): void;
  (event: "toggle-super-user"): void;
  (event: "update:activeGroup", group: ApiPermissionNavGroupId): void;
  (event: "update:selectedPerms", permissions: string[]): void;
  (event: "update:visible", visible: boolean): void;
}>();

const visibleValue = computed({
  get: () => props.visible,
  set: (visible: boolean) => emit("update:visible", visible),
});
</script>
