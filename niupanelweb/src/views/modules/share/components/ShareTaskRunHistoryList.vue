<template>
  <el-table
    v-if="!isMobile"
    :data="runs"
    class="w-full h-full"
    header-cell-class-name="!bg-base/30 !text-muted text-[11px] font-bold"
  >
    <el-table-column label="状态" width="100">
      <template #default="{ row }">
        <el-tag
          :type="getStatusType(row.status)"
          size="small"
          effect="plain"
          class="font-bold border-0"
        >
          {{ row.status }}
        </el-tag>
      </template>
    </el-table-column>

    <el-table-column label="开始时间" min-width="160">
      <template #default="{ row }">
        <span class="text-xs font-mono text-default">
          {{ formatDate(row.started_at) }}
        </span>
      </template>
    </el-table-column>

    <el-table-column label="耗时" width="120">
      <template #default="{ row }">
        <span class="text-xs font-mono text-secondary">
          {{ formatRunDuration(row) }}
        </span>
      </template>
    </el-table-column>

    <el-table-column label="操作" width="100" align="right">
      <template #default="{ row }">
        <el-button
          link
          type="primary"
          size="small"
          @click="emit('view-log', row.id)"
        >
          查看日志
        </el-button>
      </template>
    </el-table-column>
  </el-table>

  <div
    v-else
    class="h-full overflow-y-auto bg-white dark:bg-[#1c2431] pt-1 custom-scrollbar"
  >
    <div
      v-for="row in runs"
      :key="row.id"
      class="px-4 py-3 flex items-center justify-between border-b border-light/30 last:border-b-0 active:bg-black/5 dark:active:bg-white/5 transition-colors"
    >
      <div class="flex flex-col gap-1 flex-1 pl-1">
        <div class="flex items-center gap-2 mb-0.5">
          <span
            class="w-[6px] h-[6px] rounded-full"
            :class="getStatusDotClass(row.status)"
          ></span>
          <span
            class="text-[13px] font-bold"
            :class="getStatusTextClass(row.status)"
          >
            {{ row.status }}
          </span>
        </div>
        <div
          class="flex items-center gap-3 text-[12px] text-muted opacity-80 font-mono"
        >
          <span>{{ formatDateCompact(new Date(row.started_at).getTime()) }}</span>
          <span
            v-if="row.ended_at"
            class="text-secondary bg-secondary/10 px-1.5 py-0.5 rounded-md"
          >
            {{ formatRunDuration(row) }}
          </span>
        </div>
      </div>

      <button
        class="accent-subtle mr-1 h-[30px] rounded-md px-3.5 text-[12px] font-bold outline-none transition-[filter,box-shadow] duration-200 hover:brightness-95 focus-visible:ring-2 focus-visible:ring-primary/25"
        @click="emit('view-log', row.id)"
      >
        查看日志
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { formatDate } from "../../../../utils/format";
import type { TaskRunHistoryItem } from "@/types";

type TagType = "success" | "danger" | "primary" | "warning" | "info";

defineProps<{
  isMobile: boolean;
  runs: TaskRunHistoryItem[];
}>();

const emit = defineEmits<{
  (event: "view-log", runId: number): void;
}>();

const getStatusType = (status: string): TagType => {
  if (status === "Finished") return "success";
  if (status === "Failed") return "danger";
  if (status === "Running") return "primary";
  if (status === "Stopped") return "warning";
  if (status === "Paused") return "warning";
  return "info";
};

const formatDateCompact = (timestamp: number) => {
  const date = new Date(timestamp);
  const pad = (value: number) => value.toString().padStart(2, "0");
  return `${date.getFullYear()}/${pad(date.getMonth() + 1)}/${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
};

const getStatusDotClass = (status: string) => {
  if (status === "Finished") {
    return "bg-green-500 shadow-[0_0_8px_rgba(34,197,94,0.6)]";
  }
  if (status === "Failed") {
    return "bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.6)]";
  }
  if (status === "Running") {
    return "bg-blue-500 shadow-[0_0_8px_rgba(59,130,246,0.6)]";
  }
  if (status === "Stopped" || status === "Paused") {
    return "bg-orange-500 shadow-[0_0_8px_rgba(249,115,22,0.6)]";
  }
  return "bg-gray-400";
};

const getStatusTextClass = (status: string) => {
  if (status === "Finished") return "text-green-600 dark:text-green-500";
  if (status === "Failed") return "text-red-600 dark:text-red-500";
  if (status === "Running") return "text-blue-600 dark:text-blue-500";
  if (status === "Stopped" || status === "Paused") {
    return "text-orange-600 dark:text-orange-500";
  }
  return "text-gray-500";
};

const formatDuration = (seconds: number) => {
  if (!seconds) return "-";
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return `${minutes}m ${remainingSeconds}s`;
};

const formatRunDuration = (run: TaskRunHistoryItem) => {
  if (!run.started_at || !run.ended_at) return "-";
  return formatDuration(
    Math.round(
      (new Date(run.ended_at).getTime() - new Date(run.started_at).getTime()) /
        1000,
    ),
  );
};
</script>
