<template>
  <div class="space-y-6">
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-x-12 gap-y-12">
      <div class="space-y-8">
        <section class="space-y-6">
          <div class="flex items-center gap-2 px-1">
            <div class="w-1.5 h-1.5 rounded-full bg-primary"></div>
            <h4 class="text-[13px] font-bold text-default">数据冷备份</h4>
          </div>

          <div class="space-y-5 rounded-md border border-light/40 bg-soft/10 p-5">
            <p class="text-[11px] text-muted leading-relaxed">
              根据选择导出系统核心数据的压缩包 (.tar.gz)。支持<b>免重启即时恢复</b>。
            </p>

            <div class="grid grid-cols-2 gap-x-4 gap-y-3">
              <el-checkbox v-model="backupOptions.tasks" label="任务与脚本" size="small" class="!mr-0" />
              <el-checkbox v-model="backupOptions.variables" label="环境变量" size="small" class="!mr-0" />
              <el-checkbox v-model="backupOptions.settings" label="系统设置" size="small" class="!mr-0" />
              <el-checkbox v-model="backupOptions.environments" label="运行环境元数据" size="small" class="!mr-0" />
              <el-checkbox v-model="backupOptions.telegram" label="TG 机器人配置" size="small" class="!mr-0" />
            </div>

            <div v-if="backingUp" class="space-y-2">
              <div class="flex justify-between items-center px-1">
                <span class="text-[10px] font-bold text-primary">{{ maintenanceProgress.message }}</span>
                <span class="text-[10px] font-mono font-bold">{{ maintenanceProgress.progress }}%</span>
              </div>
              <el-progress :percentage="maintenanceProgress.progress" :show-text="false" :stroke-width="6" striped striped-flow class="!rounded-full" />
            </div>

            <el-button type="primary" class="w-full !h-9 !rounded-md font-bold" @click="handleBackup" :loading="backingUp">
              <div class="i-ep-download mr-2"></div>{{ backingUp ? '正在生成备份...' : '生成备份并下载' }}
            </el-button>
          </div>
        </section>

        <section class="space-y-6">
          <div class="flex items-center gap-2 px-1">
            <div class="w-1.5 h-1.5 rounded-full bg-orange-500"></div>
            <h4 class="text-[13px] font-bold text-default">数据恢复</h4>
          </div>

          <div class="space-y-5 rounded-md border border-orange-500/20 bg-orange-500/[0.03] p-5">
            <p class="text-[11px] text-muted leading-relaxed">
              上传先前导出的备份包以还原系统。注意：这可能<b>覆盖当前数据</b>。
            </p>

            <div v-if="restoring" class="space-y-2">
              <div class="flex justify-between items-center px-1">
                <span class="text-[10px] font-bold text-orange-600">{{ maintenanceProgress.message }}</span>
                <span class="text-[10px] font-mono font-bold">{{ maintenanceProgress.progress }}%</span>
              </div>
              <el-progress :percentage="maintenanceProgress.progress" :show-text="false" :stroke-width="6" color="#f97316" striped striped-flow class="!rounded-full" />
            </div>

            <div class="flex gap-3">
              <el-button type="warning" plain class="w-full !h-9 !rounded-md border-orange-500/30 font-bold" @click="triggerRestore" :loading="restoring">
                <div class="i-ep-upload mr-2"></div>{{ restoring ? '正在恢复...' : '选择备份包并恢复' }}
              </el-button>
              <input type="file" ref="restoreInputRef" class="hidden" accept=".tar.gz" @change="handleRestore" />
            </div>
          </div>
        </section>
      </div>

      <div class="space-y-8">
        <section class="space-y-6">
          <div class="flex items-center gap-2 px-1">
            <div class="w-1.5 h-1.5 rounded-full bg-rose-500"></div>
            <h4 class="text-[13px] font-bold text-default">存储空间清理</h4>
          </div>

          <div class="space-y-5 rounded-md border border-rose-500/20 bg-rose-500/[0.03] p-5">
            <div class="flex items-start gap-3">
              <div class="mt-0.5 h-8 w-8 shrink-0 rounded-md bg-rose-500/10 text-rose-600 flex-center dark:text-rose-300">
                <span class="i-ep-delete text-[15px]"></span>
              </div>
              <div class="min-w-0">
                <h5 class="text-[13px] font-bold text-default">清理过期日志</h5>
                <p class="mt-1 text-[11px] leading-relaxed text-muted">
                  清理已结束的任务、系统任务和审计记录。每个任务最近一次运行、活动任务、近期日志以及当前服务日志都会保留。
                </p>
              </div>
            </div>

            <div class="flex flex-col gap-3 sm:flex-row sm:items-end">
              <label class="min-w-0 flex-1 space-y-1.5">
                <span class="block text-[10px] font-bold text-secondary">保留最近日志</span>
                <div class="flex h-9 items-center gap-2 rounded-md border border-light bg-card px-1">
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

              <div class="flex shrink-0 gap-2">
                <el-button
                  type="primary"
                  plain
                  :loading="previewingLogs"
                  :disabled="cleaningLogs"
                  class="!h-9 flex-1 !rounded-md px-4 font-bold sm:flex-none"
                  @click="previewCleanupLogs"
                >
                  <span class="i-ep-view mr-1.5"></span>
                  预览影响
                </el-button>
                <el-button
                  type="danger"
                  :loading="cleaningLogs"
                  :disabled="previewingLogs || !logCleanupReport?.dry_run"
                  class="!ml-0 !h-9 flex-1 !rounded-md px-4 font-bold sm:flex-none"
                  @click="handleCleanupLogs"
                >
                  <span class="i-ep-delete mr-1.5"></span>
                  确认清理
                </el-button>
              </div>
            </div>

            <div
              v-if="logCleanupReport"
              class="overflow-hidden rounded-lg border border-light bg-card"
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
                    <div class="text-[11px] font-bold text-default">
                      {{ logCleanupReport.dry_run ? "影响预览" : "清理完成" }}
                    </div>
                    <div class="truncate text-[9px] text-muted">
                      截止 {{ formatCleanupTime(logCleanupReport.cutoff_at) }}
                    </div>
                  </div>
                </div>
                <span
                  class="shrink-0 rounded-md px-2 py-1 text-[9px] font-bold"
                  :class="
                    logCleanupReport.dry_run
                      ? 'bg-amber-500/10 text-amber-700 dark:text-amber-300'
                      : 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-300'
                  "
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
                  已保护
                  <strong class="font-mono text-secondary">{{ logCleanupReport.protected_files }}</strong>
                  个近期或最近运行文件
                </span>
                <span v-if="!logCleanupReport.dry_run" class="flex items-center gap-1">
                  <span class="i-ep-folder-delete text-[11px]"></span>
                  移除
                  <strong class="font-mono text-secondary">{{ logCleanupReport.empty_directories }}</strong>
                  个空目录
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
              <p>
                先生成影响预览，确认文件数量、数据库记录和预计释放空间后再执行清理。
              </p>
            </div>
          </div>
        </section>
      </div>
    </div>
  </div>
</template>


<script setup lang="ts">
import { useSystemMaintenance } from "./composables/useSystemMaintenance";

const {
  backingUp,
  backupOptions,
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
} = useSystemMaintenance();

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
