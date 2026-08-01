<template>
  <div class="mx-auto max-w-[1120px] space-y-4 pb-6">
    <section class="module-panel overflow-hidden">
      <header class="flex items-center justify-between gap-3 border-b border-light px-4 py-3">
        <div class="flex min-w-0 items-center gap-3">
          <span class="h-9 w-9 shrink-0 rounded-md bg-primary/10 text-primary flex-center">
            <span class="i-carbon-tools text-[17px]"></span>
          </span>
          <div class="min-w-0">
            <h2 class="truncate text-[14px] font-bold text-default">数据保护</h2>
            <p class="mt-0.5 truncate text-[11px] text-muted">备份重要配置，必要时从受信任的备份包恢复。</p>
          </div>
        </div>
        <span
          class="inline-flex shrink-0 items-center gap-1.5 rounded-md px-2 py-1 text-[10px] font-semibold"
          :class="isMaintaining ? 'bg-primary/10 text-primary' : 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-300'"
        >
          <span class="h-1.5 w-1.5 rounded-full" :class="isMaintaining ? 'bg-primary animate-pulse' : 'bg-emerald-500'"></span>
          {{ isMaintaining ? "维护进行中" : "维护就绪" }}
        </span>
      </header>

      <div class="space-y-4 p-4">
        <div
          v-if="isMaintaining"
          class="rounded-md border border-primary/20 bg-primary/[0.045] px-3 py-3"
          role="status"
          aria-live="polite"
        >
          <div class="mb-2 flex items-center justify-between gap-3">
            <span class="min-w-0 truncate text-[11px] font-semibold text-primary">{{ maintenanceActivityMessage }}</span>
            <span class="shrink-0 font-mono text-[11px] font-bold text-primary">{{ cleaningLogs ? "处理中" : `${maintenanceProgress.progress}%` }}</span>
          </div>
          <el-progress
            v-if="!cleaningLogs"
            :percentage="maintenanceProgress.progress"
            :color="restoring ? '#f97316' : undefined"
            :show-text="false"
            :stroke-width="6"
            striped
            striped-flow
          />
          <p class="mt-2 text-[10px] leading-4 text-secondary">
            {{ restoring ? "恢复完成后页面会自动刷新，请勿关闭页面或重复发起维护操作。" : cleaningLogs ? "正在移除预览中确认的过期日志与历史记录。" : "任务完成后将自动开始下载备份包。" }}
          </p>
        </div>

        <div class="grid gap-4 lg:grid-cols-2">
          <section class="overflow-hidden rounded-md border border-light bg-card">
            <header class="flex items-start justify-between gap-3 border-b border-light bg-subtle/45 px-4 py-3">
              <div class="flex min-w-0 items-start gap-2.5">
                <span class="mt-0.5 h-8 w-8 shrink-0 rounded-md bg-primary/10 text-primary flex-center">
                  <span class="i-ep-download text-[15px]"></span>
                </span>
                <div class="min-w-0">
                  <h3 class="text-[13px] font-bold text-default">创建备份</h3>
                  <p class="mt-0.5 text-[10px] leading-4 text-muted">生成 `.tar.gz` 归档，并在完成后下载到本地。</p>
                </div>
              </div>
              <span class="shrink-0 rounded-md bg-card px-2 py-1 text-[10px] font-semibold text-secondary">
                {{ selectedBackupCount }}/5 项
              </span>
            </header>

            <div class="space-y-4 p-4">
              <fieldset :disabled="backingUp || restoring" class="grid grid-cols-1 gap-x-3 gap-y-2 sm:grid-cols-2">
                <legend class="sr-only">选择备份内容</legend>
                <el-checkbox v-model="backupOptions.tasks" label="任务与脚本" size="small" class="!mr-0" />
                <el-checkbox v-model="backupOptions.variables" label="环境变量" size="small" class="!mr-0" />
                <el-checkbox v-model="backupOptions.settings" label="系统设置" size="small" class="!mr-0" />
                <el-checkbox v-model="backupOptions.environments" label="运行环境元数据" size="small" class="!mr-0" />
                <el-checkbox v-model="backupOptions.telegram" label="Telegram 机器人配置" size="small" class="!mr-0" />
              </fieldset>

              <div class="flex items-start gap-2 rounded-md border border-amber-500/20 bg-amber-500/[0.045] px-3 py-2 text-[10px] leading-4 text-amber-800 dark:text-amber-200">
                <span class="i-ep-warning-filled mt-0.5 shrink-0 text-[11px]"></span>
                <p>变量、系统设置和 Telegram 配置可能含有敏感明文，请仅保存到受信任的位置。</p>
              </div>

              <el-button
                type="primary"
                class="!mx-0 !h-9 !w-full !rounded-md font-bold"
                :disabled="restoring || cleaningLogs"
                :loading="backingUp"
                @click="handleBackup"
              >
                <span class="i-ep-download mr-2"></span>
                {{ backingUp ? "正在生成备份..." : "生成备份并下载" }}
              </el-button>
            </div>
          </section>

          <section class="overflow-hidden rounded-md border border-orange-500/25 bg-card">
            <header class="flex items-start justify-between gap-3 border-b border-orange-500/15 bg-orange-500/[0.045] px-4 py-3">
              <div class="flex min-w-0 items-start gap-2.5">
                <span class="mt-0.5 h-8 w-8 shrink-0 rounded-md bg-orange-500/10 text-orange-600 flex-center dark:text-orange-300">
                  <span class="i-ep-upload text-[15px]"></span>
                </span>
                <div class="min-w-0">
                  <h3 class="text-[13px] font-bold text-default">从备份恢复</h3>
                  <p class="mt-0.5 text-[10px] leading-4 text-muted">上传此前导出的 `.tar.gz` 文件，恢复会覆盖其中的现有数据。</p>
                </div>
              </div>
              <span class="shrink-0 rounded-md bg-orange-500/10 px-2 py-1 text-[10px] font-semibold text-orange-700 dark:text-orange-300">高风险</span>
            </header>

            <div class="space-y-4 p-4">
              <div class="flex items-start gap-2 rounded-md border border-orange-500/20 bg-orange-500/[0.04] px-3 py-2 text-[10px] leading-4 text-orange-800 dark:text-orange-200">
                <span class="i-ep-info-filled mt-0.5 shrink-0 text-[11px]"></span>
                <p>恢复前请先创建一份当前备份。完成后系统会自动刷新，请勿在恢复过程中关闭页面。</p>
              </div>

              <div class="flex gap-2">
                <el-button
                  type="warning"
                  plain
                  class="!mx-0 !h-9 min-w-0 flex-1 !rounded-md border-orange-500/30 font-bold"
                  :disabled="backingUp || cleaningLogs"
                  :loading="restoring"
                  @click="triggerRestore"
                >
                  <span class="i-ep-upload mr-2"></span>
                  {{ restoring ? "正在恢复..." : "选择备份包并恢复" }}
                </el-button>
                <el-button
                  v-if="uploadingRestore"
                  type="danger"
                  plain
                  class="!ml-0 !h-9 !rounded-md font-bold"
                  @click="cancelRestoreUpload"
                >
                  取消上传
                </el-button>
                <input ref="restoreInputRef" type="file" class="hidden" accept=".tar.gz" @change="handleRestore" />
              </div>
            </div>
          </section>
        </div>
      </div>
    </section>

    <section class="module-panel overflow-hidden">
      <header class="flex items-center justify-between gap-3 border-b border-light px-4 py-3">
        <div class="flex min-w-0 items-center gap-3">
          <span class="h-9 w-9 shrink-0 rounded-md bg-rose-500/10 text-rose-600 flex-center dark:text-rose-300">
            <span class="i-ep-delete text-[16px]"></span>
          </span>
          <div class="min-w-0">
            <h2 class="truncate text-[14px] font-bold text-default">存储维护</h2>
            <p class="mt-0.5 truncate text-[11px] text-muted">先预览影响范围，再清理过期日志与历史记录。</p>
          </div>
        </div>
        <span class="shrink-0 rounded-md bg-soft px-2 py-1 text-[10px] font-semibold text-secondary">两步确认</span>
      </header>

      <div class="space-y-4 p-4">
        <div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
          <div class="min-w-0">
            <h3 class="text-[13px] font-bold text-default">清理过期日志</h3>
            <p class="mt-1 text-[11px] leading-5 text-muted">
              清理已结束的任务、系统任务和审计记录；活动任务、最近一次运行、近期日志与当前服务日志会受到保护。
            </p>
          </div>

          <div class="flex flex-col gap-2 sm:flex-row sm:items-end">
            <label class="min-w-0 space-y-1.5">
              <span class="block text-[10px] font-semibold text-secondary">保留最近日志</span>
              <div class="flex h-9 w-full items-center gap-2 rounded-md border border-light bg-card px-1 sm:w-[168px]">
                <el-input-number
                  v-model="cleanupDays"
                  :min="1"
                  :max="365"
                  class="!w-full modern-number"
                  size="small"
                  controls-position="right"
                  aria-label="日志保留天数"
                />
                <span class="shrink-0 px-2 text-[10px] font-bold text-muted">天</span>
              </div>
            </label>

            <div class="flex gap-2">
              <el-button
                type="primary"
                plain
                :loading="previewingLogs"
                :disabled="cleaningLogs || backingUp || restoring"
                class="!mx-0 !h-9 flex-1 !rounded-md px-4 font-bold sm:flex-none"
                @click="previewCleanupLogs"
              >
                <span class="i-ep-view mr-1.5"></span>
                预览影响
              </el-button>
              <el-button
                type="danger"
                :loading="cleaningLogs"
                :disabled="previewingLogs || backingUp || restoring || !logCleanupReport?.dry_run"
                class="!ml-0 !h-9 flex-1 !rounded-md px-4 font-bold sm:flex-none"
                @click="handleCleanupLogs"
              >
                <span class="i-ep-delete mr-1.5"></span>
                确认清理
              </el-button>
            </div>
          </div>
        </div>

        <div
          v-if="logCleanupReport"
          class="overflow-hidden rounded-md border border-light bg-card"
          aria-live="polite"
        >
          <div
            class="flex items-center justify-between gap-3 border-b border-light px-3 py-2.5"
            :class="logCleanupReport.dry_run ? 'bg-amber-500/[0.06]' : 'bg-emerald-500/[0.06]'"
          >
            <div class="flex min-w-0 items-center gap-2">
              <span
                :class="logCleanupReport.dry_run ? 'i-ep-warning text-amber-600' : 'i-ep-circle-check text-emerald-600'"
                class="shrink-0 text-[14px]"
              ></span>
              <div class="min-w-0">
                <div class="text-[11px] font-bold text-default">{{ logCleanupReport.dry_run ? "影响预览" : "清理完成" }}</div>
                <div class="truncate text-[9px] text-muted">截止 {{ formatCleanupTime(logCleanupReport.cutoff_at) }}</div>
              </div>
            </div>
            <span
              class="shrink-0 rounded-md px-2 py-1 text-[9px] font-bold"
              :class="logCleanupReport.dry_run ? 'bg-amber-500/10 text-amber-700 dark:text-amber-300' : 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-300'"
            >
              {{ logCleanupReport.dry_run ? "尚未删除" : "已执行" }}
            </span>
          </div>

          <dl class="grid grid-cols-2 divide-x divide-y divide-light sm:grid-cols-4 sm:divide-y-0">
            <div class="p-3">
              <dt class="text-[9px] font-semibold text-muted">日志文件</dt>
              <dd class="mt-1 font-mono text-[15px] font-bold text-default">{{ logCleanupReport.files }}</dd>
            </div>
            <div class="p-3">
              <dt class="text-[9px] font-semibold text-muted">任务运行</dt>
              <dd class="mt-1 font-mono text-[15px] font-bold text-default">{{ logCleanupReport.task_runs }}</dd>
            </div>
            <div class="p-3">
              <dt class="text-[9px] font-semibold text-muted">系统任务</dt>
              <dd class="mt-1 font-mono text-[15px] font-bold text-default">{{ logCleanupReport.system_jobs }}</dd>
            </div>
            <div class="p-3">
              <dt class="text-[9px] font-semibold text-muted">审计记录</dt>
              <dd class="mt-1 font-mono text-[15px] font-bold text-default">{{ logCleanupReport.audit_logs }}</dd>
            </div>
          </dl>

          <div class="flex flex-wrap items-center gap-x-4 gap-y-1 border-t border-light bg-subtle/40 px-3 py-2 text-[9px] text-muted">
            <span class="flex items-center gap-1">
              <span class="i-ep-coin text-[11px]"></span>
              {{ logCleanupReport.dry_run ? "预计释放" : "已释放" }}
              <strong class="font-mono text-secondary">{{ formatBytes(logCleanupReport.bytes) }}</strong>
            </span>
            <span class="flex items-center gap-1">
              <span class="i-ep-lock text-[11px]"></span>
              已保护 <strong class="font-mono text-secondary">{{ logCleanupReport.protected_files }}</strong> 个近期或最近运行文件
            </span>
            <span v-if="!logCleanupReport.dry_run" class="flex items-center gap-1">
              <span class="i-ep-folder-delete text-[11px]"></span>
              移除 <strong class="font-mono text-secondary">{{ logCleanupReport.empty_directories }}</strong> 个空目录
            </span>
          </div>

          <ul
            v-if="logCleanupReport.warnings.length"
            class="space-y-1 border-t border-amber-300/50 bg-amber-50/60 px-3 py-2 dark:border-amber-700/40 dark:bg-amber-950/15"
          >
            <li
              v-for="warning in logCleanupReport.warnings"
              :key="warning"
              class="flex items-start gap-1.5 text-[9px] leading-4 text-amber-800 dark:text-amber-200"
            >
              <span class="i-ep-warning mt-0.5 shrink-0 text-[10px]"></span>
              <span class="break-all">{{ warning }}</span>
            </li>
          </ul>
        </div>

        <div
          v-else
          class="flex items-start gap-2 rounded-md border border-light bg-subtle/35 px-3 py-2.5 text-[10px] leading-4 text-muted"
        >
          <span class="i-ep-info-filled mt-0.5 shrink-0 text-[11px] text-primary"></span>
          <p>请先生成影响预览，确认文件数量、数据库记录和预计释放空间后，再执行不可逆的清理。</p>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useSystemMaintenance } from "./composables/useSystemMaintenance";

const {
  backingUp,
  backupOptions,
  cancelRestoreUpload,
  cleaningLogs,
  cleanupDays,
  handleBackup,
  handleCleanupLogs,
  handleRestore,
  logCleanupReport,
  maintenanceProgress,
  previewCleanupLogs,
  previewingLogs,
  restoring,
  restoreInputRef,
  triggerRestore,
  uploadingRestore,
} = useSystemMaintenance();

const isMaintaining = computed(
  () => backingUp.value || restoring.value || cleaningLogs.value,
);
const maintenanceActivityMessage = computed(() =>
  cleaningLogs.value ? "正在清理过期日志..." : maintenanceProgress.value.message,
);
const selectedBackupCount = computed(
  () => Object.values(backupOptions.value).filter(Boolean).length,
);

const formatBytes = (bytes: number) => {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const unit = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  const value = bytes / 1024 ** unit;
  return `${value >= 10 || unit === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unit]}`;
};

const formatCleanupTime = (value: string) => {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
};
</script>
