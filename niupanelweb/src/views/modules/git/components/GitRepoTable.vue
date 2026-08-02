<template>
  <div class="relative flex min-h-0 flex-1 flex-col overflow-hidden bg-card">
    <div v-if="isMobile" v-loading="loading" class="min-h-0 flex-1 overflow-y-auto">
      <div
        v-if="repos.length === 0 && !loading"
        class="h-full flex-center flex-col px-6 text-center text-muted"
      >
        <div class="i-ep-share mb-2 text-3xl opacity-30"></div>
        <div class="text-sm font-semibold text-default">暂无 Git 仓库</div>
        <div class="mt-1 text-xs">添加仓库后可同步脚本并导入任务</div>
      </div>

      <article
        v-for="row in repos"
        :key="row.id"
        class="border-b border-light px-3 py-3 last:border-b-0"
      >
        <div class="flex min-w-0 items-start gap-3">
          <div class="h-9 w-9 shrink-0 rounded-md bg-subtle text-secondary flex-center">
            <div class="i-ep-share text-[16px]"></div>
          </div>
          <div class="min-w-0 flex-1">
            <div class="flex min-w-0 items-center gap-2">
              <span class="truncate text-sm font-semibold text-default">{{ row.name }}</span>
              <span class="shrink-0 rounded bg-subtle px-1.5 py-0.5 font-mono text-[11px] text-secondary">
                {{ row.branch }}
              </span>
            </div>
            <div class="mt-1 truncate font-mono text-[11px] text-muted">
              {{ row.repo_url }}
            </div>
            <div class="mt-2 flex flex-wrap items-center gap-2 text-[11px] text-secondary">
              <span class="flex items-center gap-1.5">
                <span class="h-2 w-2 rounded-full" :class="getStatusColor(row.last_sync_status)"></span>
                {{ statusLabel(row.last_sync_status) }}
              </span>
              <span v-if="row.last_sync_at" class="text-muted">{{ formatDate(row.last_sync_at) }}</span>
              <span v-if="row.auto_sync" class="rounded bg-emerald-500/10 px-1.5 py-0.5 text-emerald-700 dark:text-emerald-300">
                自动同步
              </span>
            </div>
          </div>
        </div>

        <div class="mt-3 flex items-center justify-end gap-1 border-t border-light pt-2">
          <button type="button" class="repo-action" title="扫描并导入任务" aria-label="扫描并导入任务" @click="$emit('scan', row)">
            <div class="i-ep-magic-stick"></div>
          </button>
          <button type="button" class="repo-action" title="浏览文件" aria-label="浏览仓库文件" @click="$emit('browse', row)">
            <div class="i-ep-folder"></div>
          </button>
          <button type="button" class="repo-action text-primary" title="立即同步" aria-label="立即同步仓库" @click="$emit('sync', row)">
            <div class="i-ep-refresh" :class="{ 'animate-spin': syncingId === row.id }"></div>
          </button>
          <button type="button" class="repo-action" title="编辑" aria-label="编辑仓库" @click="$emit('edit', row)">
            <div class="i-ep-edit"></div>
          </button>
          <button type="button" class="repo-action text-rose-600 dark:text-rose-300" title="删除" aria-label="删除仓库" @click="$emit('delete', row)">
            <div class="i-ep-delete"></div>
          </button>
        </div>
      </article>
    </div>

    <el-table
      v-else
      v-loading="loading"
      :data="repos"
      height="100%"
      stripe
      class="settings-table"
    >
      <el-table-column label="仓库信息" min-width="250">
        <template #default="{ row }">
          <div class="flex flex-col gap-1 py-1">
            <span class="text-sm font-semibold text-default">{{ row.name }}</span>
            <div class="flex items-center gap-2 overflow-hidden">
              <div class="i-ep-link shrink-0 text-[11px] text-muted"></div>
              <span class="truncate select-all font-mono text-[11px] text-muted">{{ row.repo_url }}</span>
            </div>
            <div class="flex items-center gap-2">
              <el-tag size="small" effect="plain" class="h-5 px-1.5 !text-[10px]">
                <div class="i-ep-share mr-1"></div>
                {{ row.branch }}
              </el-tag>
              <el-tag v-if="row.auto_sync" type="success" size="small" effect="plain" class="h-5 px-1.5 !text-[10px]">
                自动同步
              </el-tag>
            </div>
          </div>
        </template>
      </el-table-column>

      <el-table-column label="同步状态" width="200">
        <template #default="{ row }">
          <div class="flex flex-col gap-1.5 py-1">
            <div class="flex items-center gap-2">
              <div class="h-2 w-2 rounded-full" :class="getStatusColor(row.last_sync_status)"></div>
              <span class="text-xs font-semibold">{{ statusLabel(row.last_sync_status) }}</span>
            </div>
            <div v-if="row.last_sync_at" class="font-mono text-[11px] text-muted">
              {{ formatDate(row.last_sync_at) }}
            </div>
          </div>
        </template>
      </el-table-column>

      <el-table-column label="当前 Commit" width="130">
        <template #default="{ row }">
          <span v-if="row.current_commit" class="rounded border border-light bg-subtle px-1.5 py-0.5 font-mono text-[11px]">
            {{ row.current_commit }}
          </span>
          <span v-else class="text-muted opacity-30">-</span>
        </template>
      </el-table-column>

      <el-table-column label="操作" width="220" align="right" fixed="right">
        <template #default="{ row }">
          <div class="flex justify-end gap-1">
            <el-tooltip content="扫描并导入任务" placement="top">
              <el-button size="small" type="success" link aria-label="扫描并导入任务" @click="$emit('scan', row)">
                <div class="i-ep-magic-stick text-lg"></div>
              </el-button>
            </el-tooltip>
            <el-tooltip content="浏览文件" placement="top">
              <el-button size="small" type="info" link aria-label="浏览仓库文件" @click="$emit('browse', row)">
                <div class="i-ep-folder text-lg"></div>
              </el-button>
            </el-tooltip>
            <el-tooltip content="立即同步" placement="top">
              <el-button size="small" type="primary" link :loading="syncingId === row.id" aria-label="立即同步仓库" @click="$emit('sync', row)">
                <div class="i-ep-refresh text-lg"></div>
              </el-button>
            </el-tooltip>
            <el-button size="small" link aria-label="编辑仓库" @click="$emit('edit', row)">
              <div class="i-ep-edit text-lg"></div>
            </el-button>
            <el-button size="small" type="danger" link aria-label="删除仓库" @click="$emit('delete', row)">
              <div class="i-ep-delete text-lg"></div>
            </el-button>
          </div>
        </template>
      </el-table-column>

      <template #empty>
        <div class="flex min-h-[360px] flex-col items-center justify-center px-6 text-center">
          <div class="h-11 w-11 rounded-md bg-subtle text-muted flex-center">
            <div class="i-ep-share text-xl opacity-50"></div>
          </div>
          <div class="mt-3 text-sm font-semibold text-default">暂无 Git 仓库</div>
          <div class="mt-1 text-xs text-muted">添加仓库后可同步脚本并导入任务</div>
        </div>
      </template>
    </el-table>
  </div>
</template>

<script setup lang="ts">
import type { GitRepo } from "@/api/git";
import { formatDate } from "@/utils/format";

defineProps<{
  getStatusColor: (status: string | null) => string;
  isMobile: boolean;
  loading: boolean;
  repos: GitRepo[];
  syncingId: number | null;
}>();

defineEmits<{
  (e: "browse", row: GitRepo): void;
  (e: "delete", row: GitRepo): void;
  (e: "edit", row: GitRepo): void;
  (e: "scan", row: GitRepo): void;
  (e: "sync", row: GitRepo): void;
}>();

const statusLabel = (status: string | null) => {
  if (!status) return "尚未同步";
  const normalized = status.toLowerCase();
  if (normalized === "success") return "同步成功";
  if (normalized === "failed" || normalized === "error") return "同步失败";
  if (normalized === "syncing" || normalized === "running") return "同步中";
  return status;
};
</script>

<style scoped>
.repo-action {
  display: inline-flex;
  width: 44px;
  height: 44px;
  cursor: pointer;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  transition: color 0.16s ease, background-color 0.16s ease;
}

.repo-action:hover {
  color: var(--text-default);
  background: var(--bg-soft);
}
</style>
