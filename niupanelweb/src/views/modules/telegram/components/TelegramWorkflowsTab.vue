<template>
  <div class="h-full min-h-0">
    <div class="flex h-full min-h-0 flex-col gap-2">
      <div class="flex min-h-8 shrink-0 items-center justify-between">
        <span class="text-sm font-bold text-default">事件触发</span>
        <button
          class="rounded-md bg-primary px-3 text-[11px] font-bold text-white flex-center gap-1.5 transition-colors hover:bg-primary/90"
          :class="isMobile ? 'h-11' : 'h-8'"
          @click="$emit('create')"
        >
          <div class="i-ep-plus text-sm"></div>
          新建
        </button>
      </div>
      <div v-if="isMobile" class="min-h-0 flex-1 space-y-2 overflow-y-auto pb-2 custom-scrollbar">
        <article
          v-for="row in workflows"
          :key="row.id"
          class="rounded-md border border-light bg-card p-3"
        >
          <div class="flex min-w-0 items-start gap-2">
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <span class="rounded-md border border-primary/20 bg-soft px-2 py-1 text-[12px] font-bold text-primary">
                  {{ eventTypeLabel(row.event_type) }}
                </span>
                <span class="rounded-md border border-light bg-subtle px-2 py-1 text-[12px] font-semibold text-secondary">
                  {{ actionTypeLabel(row.action_type) }}
                </span>
              </div>
              <p class="mt-2 line-clamp-3 break-all font-mono text-[12px] leading-5 text-muted">
                {{ row.config_json || "无附加配置" }}
              </p>
            </div>
            <button
              type="button"
              class="h-11 w-11 shrink-0 rounded-md text-primary flex-center transition-colors hover:bg-soft"
              title="编辑自动化"
              aria-label="编辑自动化"
              @click="$emit('edit', row)"
            >
              <span class="i-ep-edit"></span>
            </button>
            <button
              type="button"
              class="h-11 w-11 shrink-0 rounded-md text-rose-600 flex-center transition-colors hover:bg-rose-500/10 dark:text-rose-300"
              title="删除自动化"
              aria-label="删除自动化"
              @click="$emit('delete', row)"
            >
              <span class="i-ep-delete"></span>
            </button>
          </div>
        </article>
        <div v-if="!workflows.length" class="h-40 flex-col-center gap-2 text-muted">
          <span class="i-ep-connection text-2xl"></span>
          <span class="text-[12px]">暂无自动化规则</span>
        </div>
      </div>
      <div v-else class="min-h-0 flex-1 overflow-hidden border border-light/70">
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
