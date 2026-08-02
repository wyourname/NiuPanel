<template>
  <div class="flex h-full min-h-0 flex-col gap-3">
    <div class="flex min-h-11 shrink-0 items-center justify-between gap-3 px-1">
      <span class="text-[11px] font-bold text-muted">
        {{ logs.length }} 条记录
      </span>
      <el-button
        link
        type="primary"
        @click="resetAndLoad"
        :loading="loading"
        class="!text-[13px] font-medium"
        :class="{ '!h-11 !px-2': isMobile }"
      >
        <div class="i-ep-refresh mr-1.5" :class="{ 'animate-spin': loading }"></div>刷新
      </el-button>
    </div>

    <div v-if="!isMobile" ref="scrollContainerRef" class="min-h-0 flex-1 overflow-y-auto rounded-md border border-light bg-card custom-scrollbar">
      <div
        v-for="row in logs"
        :key="row.id"
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
            <span class="text-[11.5px] font-medium text-default leading-tight">{{ formatActorText(row) }}</span>
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

    <div
      v-else
      ref="scrollContainerRefMobile"
      class="audit-mobile-list min-h-0 flex-1 overflow-x-hidden overflow-y-auto pb-3 custom-scrollbar"
    >
      <div v-if="loading && logs.length === 0" class="space-y-2.5" aria-live="polite">
        <article
          v-for="item in 3"
          :key="item"
          class="animate-pulse rounded-lg border border-light bg-card p-3"
          aria-hidden="true"
        >
          <div class="flex items-center justify-between gap-3">
            <div class="h-5 w-20 rounded bg-soft"></div>
            <div class="h-3 w-24 rounded bg-soft"></div>
          </div>
          <div class="mt-3 h-11 rounded-md bg-soft/80"></div>
          <div class="mt-3 h-12 rounded-md bg-soft/60"></div>
          <div class="mt-3 h-8 rounded-md bg-soft/50"></div>
        </article>
      </div>

      <div v-else-if="logs.length > 0" class="space-y-2.5">
        <article
          v-for="row in logs"
          :key="row.id"
          class="audit-mobile-card min-w-0 overflow-hidden rounded-lg border border-light bg-card p-3 shadow-sm"
        >
          <header class="flex min-w-0 items-start justify-between gap-3">
            <span
              class="inline-flex max-w-[58%] items-center truncate rounded-md px-2 py-1 text-[11px] font-semibold"
              :class="getActionStyle(row.action)"
            >
              {{ formatActionText(row.action) }}
            </span>
            <time
              :datetime="row.created_at"
              class="shrink-0 text-right font-mono tabular-nums"
            >
              <span class="block text-[10px] text-muted">{{ formatDateOnly(row.created_at) }}</span>
              <span class="mt-0.5 block text-[12px] font-semibold text-secondary">{{ formatTime(row.created_at) }}</span>
            </time>
          </header>

          <button
            type="button"
            class="mt-3 flex min-h-11 w-full min-w-0 items-center gap-2 rounded-md border border-light/70 bg-subtle/45 px-3 py-2 text-left transition-colors"
            :class="
              canOpenResource(row)
                ? 'cursor-pointer active:bg-primary/10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/35'
                : 'cursor-default'
            "
            :disabled="!canOpenResource(row)"
            :aria-label="canOpenResource(row) ? `打开${formatResourceText(row.resource)} ${row.resource_id}` : undefined"
            @click="openResource(row)"
          >
            <span class="h-8 w-8 shrink-0 rounded-md bg-primary/10 text-primary flex-center">
              <span
                :class="row.resource.trim().toLowerCase() === 'task' ? 'i-ep-document-checked' : 'i-ep-collection-tag'"
                class="text-[14px]"
              ></span>
            </span>
            <span class="min-w-0 flex-1">
              <span class="block text-[9px] font-semibold uppercase tracking-wide text-muted">关联资源</span>
              <span class="mt-0.5 flex min-w-0 items-baseline gap-1.5">
                <span class="shrink-0 text-[12px] font-bold text-default">{{ formatResourceText(row.resource) }}</span>
                <span
                  v-if="row.resource_id"
                  class="min-w-0 break-all font-mono text-[10.5px] leading-4 text-secondary"
                >
                  #{{ row.resource_id }}
                </span>
              </span>
            </span>
            <span v-if="canOpenResource(row)" class="i-ep-arrow-right shrink-0 text-[13px] text-primary"></span>
          </button>

          <p
            class="mt-3 break-words rounded-md bg-subtle/35 px-3 py-2.5 text-[12px] leading-5"
            :class="row.details ? 'text-secondary' : 'text-muted'"
          >
            {{ row.details || "无补充详情" }}
          </p>

          <footer class="mt-3 flex min-w-0 items-center gap-2 border-t border-light/60 pt-2.5">
            <span
              class="h-7 w-7 shrink-0 rounded-md flex-center"
              :class="
                row.actor_type === 'User'
                  ? 'bg-indigo-100 text-indigo-500 dark:bg-indigo-900/40'
                  : 'bg-amber-100 text-amber-500 dark:bg-amber-900/40'
              "
            >
              <span v-if="row.actor_type === 'User'" class="i-ep-user text-[11px]"></span>
              <span v-else class="i-ep-key text-[11px]"></span>
            </span>
            <span class="min-w-0 flex-1">
              <span class="block truncate text-[11.5px] font-semibold text-default">{{ formatActorText(row) }}</span>
              <span class="block text-[9px] font-medium text-muted">操作主体</span>
            </span>
            <span
              class="max-w-[44%] break-all text-right font-mono text-[10.5px] leading-4 text-muted"
              :title="row.ip_address || '内部调用'"
            >
              {{ row.ip_address || "内部调用" }}
            </span>
          </footer>
        </article>
      </div>

      <div v-if="logs.length > 0" class="py-5 text-center" aria-live="polite">
        <div v-if="loadingMore" class="flex items-center justify-center gap-2 text-xs text-muted">
          <div class="i-ep-loading animate-spin"></div>
          加载中...
        </div>
        <div v-else-if="noMore" class="text-[10.5px] text-muted/50">已加载全部记录</div>
      </div>

      <div v-else-if="!loading" class="py-12 text-center">
        <div class="mx-auto mb-2.5 h-12 w-12 rounded-md bg-slate-100 flex-center dark:bg-slate-800/30">
          <div class="i-ep-document text-xl text-muted/25"></div>
        </div>
        <p class="text-[12.5px] text-muted/60">暂无审计记录</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useAuditLogList } from "./composables/useAuditLogList";

const {
  canOpenResource,
  formatActorText,
  formatActionText,
  formatDateOnly,
  formatResourceText,
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
