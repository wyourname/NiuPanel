<template>
  <div class="h-full overflow-y-auto custom-scrollbar">
    <div class="flex h-full min-h-0 flex-col gap-2">
      <div class="flex min-h-8 shrink-0 items-center justify-between">
        <span class="text-sm font-bold text-default">自定义指令</span>
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
          :data="commands"
          size="small"
          class="om-table"
          highlight-current-row
        >
          <el-table-column prop="name" label="指令">
            <template #default="{ row }">
              <span class="font-mono text-xs font-bold text-default">
                /{{ row.name }}
              </span>
            </template>
          </el-table-column>
          <el-table-column
            v-if="!isMobile"
            prop="script"
            label="脚本"
            show-overflow-tooltip
          >
            <template #default="{ row }">
              <span class="font-mono text-[10px] text-muted truncate">
                {{ row.script }}
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
import type { TelegramCommand } from "@/api/telegram";

defineProps<{
  commands: TelegramCommand[];
  isMobile: boolean;
}>();

defineEmits<{
  (e: "create"): void;
  (e: "delete", command: TelegramCommand): void;
  (e: "edit", command: TelegramCommand): void;
}>();
</script>
