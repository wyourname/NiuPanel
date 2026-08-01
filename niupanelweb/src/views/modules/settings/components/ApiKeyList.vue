<template>
  <div class="relative flex min-h-0 flex-1 flex-col overflow-hidden">
    <PullToRefresh v-if="isMobile" :on-refresh="onRefresh">
      <div
        v-loading="loading"
        class="mobile-dock-safe h-full overflow-y-auto custom-scrollbar"
        :aria-busy="loading"
      >
        <div v-if="keys.length" class="space-y-2.5 pb-3">
          <ApiKeyRecord
            v-for="row in keys"
            :key="row.id"
            :api-key="row"
            :mcp-tools="mcpTools"
            @delete="emit('delete', row)"
            @edit="emit('edit', row)"
          />
        </div>
        <ApiKeyEmptyState v-else-if="!loading" />
      </div>
    </PullToRefresh>

    <div
      v-else
      v-loading="loading"
      class="min-h-0 flex-1 overflow-y-auto pr-1 custom-scrollbar"
      :aria-busy="loading"
    >
      <div v-if="keys.length" class="space-y-2.5 pb-3">
        <ApiKeyRecord
          v-for="row in keys"
          :key="row.id"
          :api-key="row"
          :mcp-tools="mcpTools"
          @delete="emit('delete', row)"
          @edit="emit('edit', row)"
        />
      </div>
      <ApiKeyEmptyState v-else-if="!loading" />
    </div>
  </div>
</template>

<script setup lang="ts">
import PullToRefresh from "@/components/common/PullToRefresh.vue";
import type { ApiKey } from "@/api/keys";
import type { McpToolInfo } from "@/types";
import ApiKeyEmptyState from "./ApiKeyEmptyState.vue";
import ApiKeyRecord from "./ApiKeyRecord.vue";

defineProps<{
  isMobile: boolean;
  keys: ApiKey[];
  loading: boolean;
  mcpTools: McpToolInfo[];
  onRefresh: () => Promise<unknown> | unknown;
}>();

const emit = defineEmits<{
  (event: "delete", key: ApiKey): void;
  (event: "edit", key: ApiKey): void;
}>();
</script>
