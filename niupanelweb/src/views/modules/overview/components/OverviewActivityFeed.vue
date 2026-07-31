<template>
  <div class="grid w-full shrink-0 grid-cols-1">
    <div
      class="flex min-h-[260px] flex-col overflow-hidden rounded-md border border-light bg-card"
    >
      <div
        class="flex min-h-11 items-center justify-between border-b border-light/70 px-4 py-2.5"
      >
        <div class="flex items-center gap-2">
          <div class="i-ep-timer text-primary text-sm"></div>
          <span class="text-[13px] font-bold text-default">最近活动</span>
        </div>
        <el-button
          link
          type="primary"
          size="small"
          class="!text-[11px] font-medium"
          @click="emit('open-audit')"
        >
          查看审计日志
        </el-button>
      </div>

      <div class="flex-1">
        <el-skeleton :loading="loading" animated :count="5" class="p-3 md:p-5">
          <template #default>
            <div v-if="isMobile" class="divide-y divide-light/70">
              <div
                v-for="row in activity"
                :key="`${row.task_name}-${row.time}`"
                class="flex items-start gap-3 px-4 py-3"
              >
                <span
                  class="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full"
                  :class="getStatusDotClass(row.status)"
                ></span>
                <div class="min-w-0 flex-1">
                  <div class="flex items-start justify-between gap-3">
                    <span class="truncate text-[12px] font-semibold text-default">
                      {{ row.task_name }}
                    </span>
                    <span class="shrink-0 font-mono text-[10px] text-muted">
                      {{ row.duration }}
                    </span>
                  </div>
                  <div class="mt-1 flex items-center justify-between gap-3">
                    <span class="text-[10px] font-medium" :class="getStatusTextClass(row.status)">
                      {{ getStatusLabel(row.status) }}
                    </span>
                    <span class="font-mono text-[10px] text-muted">
                      {{ formatDate(row.time) }}
                    </span>
                  </div>
                </div>
              </div>
              <div v-if="activity.length === 0" class="py-12 text-center text-[11px] text-muted">
                暂无运行活动
              </div>
            </div>

            <el-table
              v-else
              :data="activity"
              class="w-full h-full modern-overview-table"
              header-cell-class-name="!bg-transparent !text-muted text-[10px] font-semibold border-light/50"
            >
              <el-table-column prop="status" label="状态" width="100">
                <template #default="{ row }">
                  <div class="flex items-center gap-1.5">
                    <div
                      class="w-1.5 h-1.5 rounded-full"
                      :class="getStatusDotClass(row.status)"
                    ></div>
                    <span
                      class="text-[11px] font-medium"
                      :class="getStatusTextClass(row.status)"
                    >
                      {{ getStatusLabel(row.status) }}
                    </span>
                  </div>
                </template>
              </el-table-column>
              <el-table-column prop="task_name" label="任务" min-width="200">
                <template #default="{ row }">
                  <span class="text-xs font-medium text-default">
                    {{ row.task_name }}
                  </span>
                </template>
              </el-table-column>
              <el-table-column label="时间" width="150">
                <template #default="{ row }">
                  <span class="text-[10px] text-muted font-mono">
                    {{ formatDate(row.time) }}
                  </span>
                </template>
              </el-table-column>
              <el-table-column
                prop="duration"
                label="耗时"
                width="90"
                align="right"
              >
                <template #default="{ row }">
                  <span class="text-[10px] font-mono text-secondary">
                    {{ row.duration }}
                  </span>
                </template>
              </el-table-column>
            </el-table>
          </template>
        </el-skeleton>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { formatDate } from "../../../../utils/format";
import type { OverviewActivityItem } from "@/types";
import { useOverviewStatus } from "../composables/useOverviewStatus";

defineProps<{
  activity: OverviewActivityItem[];
  isMobile: boolean;
  loading: boolean;
}>();

const emit = defineEmits<{
  (event: "open-audit"): void;
}>();

const {
  getStatusDotClass,
  getStatusLabel,
  getStatusTextClass,
} = useOverviewStatus();
</script>
