<template>
  <div class="full overflow-y-auto bg-base/30 p-3 custom-scrollbar">
    <div class="mx-auto max-w-5xl space-y-3">
      <section class="overflow-hidden rounded-md border border-light bg-card">
        <div class="flex min-h-11 items-center gap-2 border-b border-light/70 px-4 py-2.5">
          <div class="i-ep-document-checked text-primary"></div>
          <span class="text-[13px] font-bold text-default">配置概览</span>
        </div>
        <div class="grid grid-cols-1 divide-y divide-light/70 md:grid-cols-2 md:divide-x md:divide-y-0 xl:grid-cols-3">
        <div class="flex flex-col gap-3 p-4">
          <div class="flex items-center gap-2 pb-2">
            <div class="i-ep-info-filled text-primary text-lg"></div>
            <span class="label-sm">基础信息</span>
          </div>
          <div class="space-y-2">
            <div class="flex justify-between items-center">
              <span class="text-[10px] font-semibold text-muted">任务 ID</span>
              <span class="text-xs font-mono font-bold bg-base px-1.5 py-0.5 rounded">
                #{{ task.id }}
              </span>
            </div>
            <div class="flex justify-between items-center">
              <span class="text-[10px] font-semibold text-muted">任务名称</span>
              <span class="text-xs font-bold truncate max-w-[150px]">
                {{ task.name }}
              </span>
            </div>
            <div class="flex justify-between items-center">
              <span class="text-[10px] font-semibold text-muted">脚本路径</span>
              <span class="text-xs font-mono truncate max-w-[150px] opacity-80">
                {{ task.path || "内联脚本" }}
              </span>
            </div>
            <div class="flex flex-col gap-1 mt-1">
              <span class="text-[10px] font-semibold text-muted">任务描述</span>
              <span class="text-[11px] text-secondary leading-relaxed line-clamp-2">
                {{ task.description || "暂无描述" }}
              </span>
            </div>
          </div>
        </div>

        <div class="flex flex-col gap-3 p-4">
          <div class="flex items-center gap-2 pb-2">
            <div class="i-ep-setting text-warning text-lg"></div>
            <span class="label-sm">执行配置</span>
          </div>
          <div class="space-y-2">
            <div class="flex justify-between items-center">
              <span class="text-[10px] font-semibold text-muted">定时计划</span>
              <span class="text-xs font-mono font-bold">
                {{ task.cron_schedule || "手动触发" }}
              </span>
            </div>
            <div v-if="task.next_run_at" class="flex justify-between items-center">
              <span class="text-[10px] font-semibold text-muted">下次运行</span>
              <span class="text-xs font-mono font-bold">
                {{ formatDate(task.next_run_at) }}
              </span>
            </div>
            <div class="flex justify-between items-center">
              <span class="text-[10px] font-semibold text-muted">运行环境</span>
              <span class="text-xs font-bold">
                {{ task.env_type }}
                <span class="opacity-60 text-[10px]">
                  ({{ (task.env_version || "默认").replace(/^venv_/, "") }})
                </span>
              </span>
            </div>
            <div class="flex justify-between items-center">
              <span class="text-[10px] font-semibold text-muted">状态</span>
              <div class="flex items-center gap-1.5">
                <div class="w-1.5 h-1.5 rounded-full" :class="getStatusDotClass(task)"></div>
                <span class="text-xs font-semibold">
                  {{ getStatusLabel(task.status) }}
                </span>
              </div>
            </div>
            <div class="flex justify-between items-center">
              <span class="text-[10px] font-semibold text-muted">上次执行</span>
              <span
                class="text-xs font-mono opacity-80"
                :title="task.last_finished_at || undefined"
              >
                {{ task.last_finished_at ? formatDate(task.last_finished_at) : "从未执行" }}
              </span>
            </div>
          </div>
        </div>

        <div class="flex flex-col gap-3 p-4 md:col-span-2 xl:col-span-1">
          <div class="flex items-center gap-2 pb-2">
            <div class="i-ep-cpu text-emerald-500 text-lg"></div>
            <span class="label-sm">限制与标记</span>
          </div>
          <div class="space-y-2">
            <div class="flex justify-between items-center">
              <span class="text-[10px] font-semibold text-muted">CPU 限制</span>
              <span class="text-xs font-mono font-bold">
                {{ formatLimit(task.cpu_limit, "%") }}
              </span>
            </div>
            <div class="flex justify-between items-center">
              <span class="text-[10px] font-semibold text-muted">超时限制</span>
              <span class="text-xs font-mono font-bold">
                {{ formatLimit(task.timeout_sec, " 秒") }}
              </span>
            </div>
            <div class="flex justify-between items-center">
              <span class="text-[10px] font-semibold text-muted">内存限制</span>
              <span class="text-xs font-mono font-bold">
                {{ formatLimit(task.memory_limit, " MB") }}
              </span>
            </div>
            <div class="flex justify-between items-center">
              <span class="text-[10px] font-semibold text-muted">消息通知</span>
              <div
                class="text-[16px] leading-none"
                :class="task.notify ? 'i-ep-select text-emerald-500' : 'i-ep-close text-rose-500'"
              ></div>
            </div>
            <div class="flex justify-between items-center">
              <span class="text-[10px] font-semibold text-muted">创建时间</span>
              <span class="text-[10px] font-mono opacity-60">
                {{ formatDate(task.created_at) }}
              </span>
            </div>
          </div>
        </div>
        </div>
      </section>

      <section class="overflow-hidden rounded-md border border-light bg-card">
        <div class="min-h-11 border-b border-light px-4 py-3 flex-between">
          <div class="text-[13px] font-bold text-default">
            任务历史记录
          </div>
        </div>
        <div class="h-[350px]">
          <ShareImportHistory
            :task-id="task.id"
            @view-log="(logPath, runId) => emit('view-log', logPath, runId)"
          />
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Task } from "@/types";
import {
  getStatusDotClass,
  getStatusLabel,
} from "../../composables/useTaskPresentation";
import { formatDate } from "../../utils/format";
import ShareImportHistory from "../../views/modules/share/components/ShareImportHistory.vue";

defineProps<{
  task: Task;
}>();

const emit = defineEmits<{
  (event: "view-log", logPath: string, runId: number): void;
}>();

const formatLimit = (value: number | null | undefined, unit: string) =>
  value && value > 0 ? `${value}${unit}` : "无限制";
</script>
