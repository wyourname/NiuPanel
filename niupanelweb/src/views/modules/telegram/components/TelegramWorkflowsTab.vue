<template>
  <div class="h-full overflow-y-auto custom-scrollbar">
    <div class="flex h-full min-h-0 flex-col gap-2">
      <div class="flex min-h-8 shrink-0 items-center justify-between">
        <span class="text-sm font-bold text-default">事件触发</span>
        <button
          class="h-8 rounded-md bg-primary px-3 text-[11px] font-bold text-white flex-center gap-1.5 transition-colors hover:bg-primary/90"
          @click="$emit('create')"
        >
          <div class="i-ep-plus text-sm"></div>
          新建
        </button>
      </div>
      <div
        class="min-h-0 flex-1 overflow-hidden"
        :class="isMobile ? 'rounded-md border border-light bg-card' : 'border border-light/70'"
      >
        <el-table
          :data="workflows"
          size="small"
          class="om-table"
          highlight-current-row
        >
          <el-table-column prop="event_type" label="触发条件">
            <template #default="{ row }">
              <span class="font-bold text-xs text-primary px-2 py-0.5 rounded border">
                {{ eventTypeLabel(row.event_type) }}
              </span>
            </template>
          </el-table-column>
          <el-table-column v-if="!isMobile" prop="action_type" label="动作">
            <template #default="{ row }">
              <span
                class="font-bold text-[10px] text-default bg-base px-2 py-0.5 rounded border border-light"
              >
                {{ actionTypeLabel(row.action_type) }}
              </span>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="100" align="right">
            <template #default="{ row }">
              <el-button link type="primary" size="small" @click="$emit('edit', row)">
                编辑
              </el-button>
              <el-button link type="danger" size="small" @click="$emit('delete', row)">
                删除
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type {
  TelegramWorkflow,
  TelegramWorkflowActionType,
  TelegramWorkflowEventType,
} from "@/api/telegram";

defineProps<{
  actionTypeLabel: (value: TelegramWorkflowActionType) => string;
  eventTypeLabel: (value: TelegramWorkflowEventType) => string;
  isMobile: boolean;
  workflows: TelegramWorkflow[];
}>();

defineEmits<{
  (e: "create"): void;
  (e: "delete", workflow: TelegramWorkflow): void;
  (e: "edit", workflow: TelegramWorkflow): void;
}>();
</script>
