<template>
  <div class="h-full flex flex-col overflow-hidden">
    <div v-if="loading" class="flex-center py-20">
      <div class="i-ep-loading animate-spin text-3xl opacity-20 text-primary"></div>
    </div>

    <div v-else-if="data.length === 0" class="flex-1 flex-center flex-col p-10 text-center">
      <div class="mb-3 text-[13px] font-bold text-default">暂无脚本</div>
      <button
        type="button"
        class="h-8 rounded-lg bg-primary px-3 text-[12px] font-bold text-white transition-colors hover:bg-primary/90"
        @click="$emit('open-sources')"
      >
        管理订阅源
      </button>
    </div>

    <div v-else class="flex-1 overflow-y-auto custom-scrollbar">
      <div class="divide-y divide-light/80 border-y border-light/70">
        <div
          v-for="item in data"
          :key="item.script.url"
          class="group flex min-h-[68px] items-center gap-3 px-4 py-3 transition-colors hover:bg-soft/55"
        >
          <div class="h-10 w-10 shrink-0 rounded-lg border border-light/70 bg-base/70 flex-center">
            <div :class="item.script.icon || 'i-ep-document'" class="text-lg text-primary"></div>
          </div>

          <div class="min-w-0 flex-1">
            <div class="flex min-w-0 items-center gap-2">
              <span class="truncate text-sm font-bold text-default group-hover:text-primary">
              {{ item.script.name }}
              </span>
              <span class="shrink-0 rounded-md border border-light/70 bg-base/60 px-1.5 py-0.5 text-[9px] font-bold text-muted">
                v{{ item.script.version }}
              </span>
              <span
                v-if="item.script.is_encrypted"
                class="i-ep-lock shrink-0 text-xs text-warning"
                title="端到端加密包"
              ></span>
            </div>
            <div class="mt-1 flex min-w-0 items-center gap-2 text-[10px] font-bold text-muted">
              <span class="truncate">{{ item.source_name }}</span>
              <span
                v-for="tag in (item.script.tags || []).slice(0, 3)"
                :key="tag"
                class="shrink-0 rounded-md bg-base/70 px-1.5 py-0.5 text-secondary"
              >
                {{ tag }}
              </span>
            </div>
          </div>

          <button
            type="button"
            class="h-8 shrink-0 rounded-lg bg-primary px-3 text-[12px] font-bold text-white transition-colors hover:bg-primary/90"
            @click="$emit('install', item.script.url)"
          >
            安装
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { MarketScriptAggregated } from "@/types";

defineProps<{
  data: MarketScriptAggregated[];
  loading: boolean;
}>();

defineEmits<{
  (event: "install", url: string): void;
  (event: "open-sources"): void;
}>();
</script>
