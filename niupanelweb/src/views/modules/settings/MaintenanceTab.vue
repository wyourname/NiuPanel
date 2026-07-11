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
            <div>
              <h5 class="text-[13px] font-bold text-default mb-1">清理运行日志</h5>
              <p class="text-[11px] text-muted leading-relaxed">
                物理删除指定天数前的所有运行和审计日志记录。
              </p>
            </div>

            <div class="flex items-center gap-3">
              <div class="flex flex-1 items-center gap-2 rounded-md border border-light bg-white p-1 dark:bg-white/5">
                <el-input-number v-model="cleanupDays" :min="1" :max="365" class="!w-full modern-number" size="small" controls-position="right" />
                <span class="shrink-0 px-2 text-[10px] font-bold text-muted">天</span>
              </div>
              <el-button type="danger" plain @click="handleCleanupLogs" :loading="cleaningLogs" class="!h-9 !rounded-md border-rose-500/30 px-6 font-bold">
                立即执行
              </el-button>
            </div>
          </div>
        </section>

        <section class="space-y-6 opacity-60">
           <div class="flex items-center gap-2 px-1">
            <div class="w-1.5 h-1.5 rounded-full bg-muted"></div>
            <h4 class="text-[13px] font-bold text-muted">状态摘要</h4>
          </div>
          <div class="grid grid-cols-2 gap-4">
             <div class="space-y-1 rounded-md border border-light/50 p-4">
                <span class="text-[10px] font-bold text-muted">健康状态</span>
                <p class="text-sm font-bold text-emerald-500">良好</p>
             </div>
             <div class="space-y-1 rounded-md border border-light/50 p-4">
                <span class="text-[10px] font-bold text-muted">可用率</span>
                <p class="text-sm font-bold text-default font-mono">99.9%</p>
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
  maintenanceProgress,
  restoring,
  restoreInputRef,
  triggerRestore,
} = useSystemMaintenance();
</script>
