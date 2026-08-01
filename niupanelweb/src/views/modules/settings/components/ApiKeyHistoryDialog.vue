<template>
  <ResponsiveDialog
    v-model:visible="visibleValue"
    title="API 密钥历史"
    width="860px"
    append-to-body
    destroy-on-close
    custom-class="cloud-dialog"
  >
    <div class="flex h-full min-h-[320px] flex-col bg-card md:h-[min(640px,72vh)]">
      <div class="shrink-0 border-b border-light px-4 py-3">
        <p class="text-[11px] leading-5 text-secondary">查看、编辑或吊销已创建的 API Key；建议按客户端分别创建并只授予所需权限。</p>
      </div>
      <div class="min-h-0 flex flex-1 flex-col px-4 pt-3">
        <ApiKeyList
          :is-mobile="isMobile"
          :keys="keys"
          :loading="loading"
          :mcp-tools="mcpTools"
          :on-refresh="onRefresh"
          @delete="emit('delete', $event)"
          @edit="emit('edit', $event)"
        />
      </div>
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { ApiKey } from "@/api/keys";
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";
import type { McpToolInfo } from "@/types";
import ApiKeyList from "./ApiKeyList.vue";

const props = defineProps<{
  visible: boolean;
  isMobile: boolean;
  keys: ApiKey[];
  loading: boolean;
  mcpTools: McpToolInfo[];
  onRefresh: () => Promise<unknown> | unknown;
}>();

const emit = defineEmits<{
  (event: "update:visible", value: boolean): void;
  (event: "delete", key: ApiKey): void;
  (event: "edit", key: ApiKey): void;
}>();

const visibleValue = computed({
  get: () => props.visible,
  set: (value) => emit("update:visible", value),
});
</script>
