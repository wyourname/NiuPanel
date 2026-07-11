<template>
  <div class="space-y-4">
    <div class="flex min-h-8 items-center justify-between gap-3 px-1">
      <span class="text-[11px] font-bold text-muted">
        {{ logs.length }} 条记录
      </span>
      <el-button
        link
        type="primary"
        @click="resetAndLoad"
        :loading="loading"
        class="!text-[13px] font-medium"
      >
        <div class="i-ep-refresh mr-1.5" :class="{ 'animate-spin': loading }"></div>刷新
      </el-button>
    </div>

    <div v-if="!isMobile" ref="scrollContainerRef" class="overflow-y-auto rounded-md border border-light bg-card custom-scrollbar" style="max-height: calc(100vh - 220px)">
      <div
        v-for="(row, index) in logs"
        :key="index"
        class="group flex select-none items-center gap-4 border-b border-light/60 px-4 py-3 transition-colors last:border-b-0 hover:bg-subtle/60"
      >
        <div class="shrink-0 w-[120px]">
          <span class="text-[13px] font-semibold text-default font-mono tabular-nums">{{ formatTime(row.created_at) }}</span>
          <span class="text-[10.5px] text-muted/50 ml-1.5 font-mono">{{ formatDateOnly(row.created_at) }}</span>
        </div>

        <div
          class="shrink-0 inline-flex items-center px-2 py-0.5 rounded-md text-[11px] font-medium"
          :class="getActionStyle(row.action)"
        >{{ formatActionText(row.action) }}</div>

        <div
          v-if="row.resource === 'task' && row.resource_id"
          class="shrink-0 cursor-pointer hover:opacity-80 transition-opacity"
          @click="openResource(row)"
        >
          <span class="text-[12.5px] font-medium text-blue-500">{{ row.resource }}</span>
          <span class="text-[11px] text-blue-400/60 font-mono ml-0.5">#{{ row.resource_id }}</span>
        </div>
        <div v-else class="shrink-0">
          <span class="text-[12.5px] text-muted/70 font-medium">{{ row.resource }}</span>
          <span v-if="row.resource_id" class="text-[11px] text-muted/40 font-mono ml-0.5">#{{ row.resource_id }}</span>
        </div>

        <div class="flex-1 min-w-0">
          <span class="text-[12.5px] text-muted/50 truncate block">{{ row.details || '-' }}</span>
        </div>

        <div class="shrink-0 flex items-center gap-2 opacity-70 group-hover:opacity-100 transition-opacity">
          <div
            class="h-6 w-6 shrink-0 rounded-md flex-center"
            :class="
              row.actor_type === 'User'
                ? 'bg-indigo-100 dark:bg-indigo-900/40 text-indigo-500'
                : 'bg-amber-100 dark:bg-amber-900/40 text-amber-500'
            "
          >
            <div v-if="row.actor_type === 'User'" class="i-ep-user text-[11px]"></div>
            <div v-else class="i-ep-key text-[11px]"></div>
          </div>
          <div class="flex flex-col">
            <span class="text-[11.5px] font-medium text-default leading-tight">{{
              row.actor_type === 'User'
                ? row.user_id ? `用户 ${row.user_id}` : '系统'
                : `密钥 ${row.user_id}`
            }}</span>
            <span class="text-[9.5px] text-muted/45 font-mono leading-tight">{{ row.ip_address || '内部调用' }}</span>
          </div>
        </div>
      </div>

      <div class="py-6 text-center">
        <div v-if="loadingMore" class="text-xs text-muted flex items-center justify-center gap-2">
          <div class="i-ep-loading animate-spin"></div>加载中...
        </div>
        <div v-if="noMore && logs.length > 0" class="text-[10.5px] text-muted/35">已加载全部记录</div>
      </div>

      <div v-if="!loading && logs.length === 0" class="py-16 text-center">
        <div class="mx-auto mb-3 h-12 w-12 rounded-md bg-slate-100 flex-center dark:bg-slate-800/30">
          <div class="i-ep-document text-2xl text-muted/25"></div>
        </div>
        <p class="text-[13px] text-muted/50">暂无审计记录</p>
      </div>
    </div>

    <div v-else ref="scrollContainerRefMobile" class="-mx-4 overflow-y-auto border-y border-light bg-card pb-4 custom-scrollbar" style="max-height: calc(100vh - 200px)">
      <div
        v-for="(row, index) in logs"
        :key="index"
        class="space-y-2.5 border-b border-light/60 p-4 last:border-b-0"
      >
        <div class="flex items-center justify-between">
          <div
            class="inline-flex items-center px-2 py-0.5 rounded-md text-[11px] font-medium"
            :class="getActionStyle(row.action)"
          >{{ formatActionText(row.action) }}</div>
          <span class="text-[11px] text-muted/40 font-mono tabular-nums">{{ formatTime(row.created_at) }} {{ formatDateOnly(row.created_at) }}</span>
        </div>

        <div class="flex items-center gap-2 text-[12.5px]">
          <span class="font-medium text-default">{{ row.resource }}</span>
          <span v-if="row.resource_id" class="text-muted/40 font-mono">#{{ row.resource_id }}</span>
        </div>

        <p v-if="row.details" class="text-[12px] text-muted/55 leading-relaxed line-clamp-2">{{ row.details }}</p>

        <div class="flex items-center gap-2 pt-1 border-t border-light/20">
          <div
            class="h-5 w-5 shrink-0 rounded-md flex-center"
            :class="
              row.actor_type === 'User'
                ? 'bg-indigo-100 dark:bg-indigo-900/40 text-indigo-500'
                : 'bg-amber-100 dark:bg-amber-900/40 text-amber-500'
            "
          >
            <div v-if="row.actor_type === 'User'" class="i-ep-user text-[10px]"></div>
            <div v-else class="i-ep-key text-[10px]"></div>
          </div>
          <span class="text-[11.5px] text-muted/70">{{
            row.actor_type === 'User'
              ? row.user_id ? `用户 ${row.user_id}` : '系统'
              : `密钥 ${row.user_id}`
          }}</span>
          <span class="text-[10px] text-muted/35 font-mono ml-auto">{{ row.ip_address || '内部调用' }}</span>
        </div>
      </div>

      <div class="py-6 text-center">
        <div v-if="loadingMore" class="text-xs text-muted flex items-center justify-center gap-2">
          <div class="i-ep-loading animate-spin"></div>加载中...
        </div>
        <div v-if="noMore && logs.length > 0" class="text-[10.5px] text-muted/35">已加载全部记录</div>
      </div>

      <div v-if="!loading && logs.length === 0" class="py-12 text-center">
        <div class="mx-auto mb-2.5 h-12 w-12 rounded-md bg-slate-100 flex-center dark:bg-slate-800/30">
          <div class="i-ep-document text-xl text-muted/25"></div>
        </div>
        <p class="text-[12.5px] text-muted/50">暂无审计记录</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useAuditLogList } from "./composables/useAuditLogList";

const {
  formatActionText,
  formatDateOnly,
  formatTime,
  getActionStyle,
  isMobile,
  loading,
  loadingMore,
  logs,
  noMore,
  openResource,
  resetAndLoad,
  scrollContainerRef,
  scrollContainerRefMobile,
} = useAuditLogList();
</script>
