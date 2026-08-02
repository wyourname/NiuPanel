<template>
  <div class="h-full min-h-0">
    <div class="flex h-full min-h-0 flex-col gap-2">
      <div class="flex min-h-8 shrink-0 items-center justify-between">
        <span class="text-sm font-bold text-default">自定义指令</span>
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
          v-for="row in commands"
          :key="row.id"
          class="rounded-md border border-light bg-card p-3"
        >
          <div class="flex min-w-0 items-center gap-2">
            <span class="min-w-0 flex-1 truncate font-mono text-[13px] font-bold text-default">
              /{{ row.name }}
            </span>
            <button
              type="button"
              class="h-11 w-11 shrink-0 rounded-md text-primary flex-center transition-colors hover:bg-soft"
              title="编辑指令"
              aria-label="编辑指令"
              @click="$emit('edit', row)"
            >
              <span class="i-ep-edit"></span>
            </button>
            <button
              type="button"
              class="h-11 w-11 shrink-0 rounded-md text-rose-600 flex-center transition-colors hover:bg-rose-500/10 dark:text-rose-300"
              title="删除指令"
              aria-label="删除指令"
              @click="$emit('delete', row)"
            >
              <span class="i-ep-delete"></span>
            </button>
          </div>
          <pre class="mt-2 whitespace-pre-wrap break-all rounded-md bg-subtle p-2.5 font-mono text-[12px] leading-5 text-secondary">{{ row.script }}</pre>
        </article>
        <div v-if="!commands.length" class="h-40 flex-col-center gap-2 text-muted">
          <span class="i-ep-chat-line-square text-2xl"></span>
          <span class="text-[12px]">暂无自定义指令</span>
        </div>
      </div>
      <div v-else class="min-h-0 flex-1 overflow-hidden border border-light/70">
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
