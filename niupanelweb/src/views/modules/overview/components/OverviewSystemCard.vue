<template>
  <div
    class="flex min-h-[250px] flex-col overflow-hidden rounded-md border border-light bg-card md:min-h-[320px]"
  >
    <div
      class="flex min-h-11 items-center justify-between border-b border-light/70 px-4 py-2.5"
    >
      <div class="flex items-center gap-2">
        <div class="i-ep-monitor text-primary text-sm"></div>
        <span class="text-[13px] font-bold text-default">节点状态</span>
      </div>
    </div>
    <div class="p-4 md:p-5 flex-1 flex flex-col">
      <el-skeleton :loading="loading" animated :count="3">
        <template #default>
          <div class="flex flex-col gap-4">
            <div class="flex items-center gap-3">
              <div
                class="h-8 w-8 shrink-0 rounded-md border border-light/50 bg-base flex-center"
              >
                <div class="i-ep-platform text-sm opacity-50"></div>
              </div>
              <div class="flex flex-col min-w-0">
                <span class="text-[10px] font-medium text-muted">操作系统</span>
                <span class="text-xs font-medium text-default truncate">
                  {{ sysInfo.os_info }}
                </span>
              </div>
            </div>

            <div class="flex items-center gap-3">
              <div
                class="h-8 w-8 shrink-0 rounded-md border border-light/50 bg-base flex-center"
              >
                <div class="i-ep-connection text-sm opacity-50"></div>
              </div>
              <div class="flex flex-col min-w-0">
                <span class="text-[10px] font-medium text-muted">节点地址</span>
                <span class="text-xs font-mono font-medium text-default">
                  {{ sysInfo.public_ip || "本地节点" }}
                </span>
              </div>
            </div>

            <div class="flex flex-col gap-1.5">
              <div class="flex justify-between items-center">
                <span class="text-[10px] text-muted font-medium">磁盘</span>
                <span class="text-[10px] font-mono text-muted">
                  {{ diskPercentage }}%
                </span>
              </div>
              <div class="h-1.5 w-full bg-base rounded-full overflow-hidden">
                <div
                  class="h-full rounded-full transition-all duration-1000 ease-out"
                  :class="diskBarClass"
                  :style="{ width: diskPercentage + '%' }"
                ></div>
              </div>
            </div>
          </div>

          <div class="mt-auto flex gap-2 border-t border-light/70 pt-4">
            <el-button
              type="primary"
              class="flex-1 !h-9 !rounded-md !text-xs font-semibold"
              @click="emit('open-tasks')"
            >
              <div class="i-ep-plus mr-1"></div>
              任务管理
            </el-button>
            <el-button
              class="flex-1 !h-9 !rounded-md !border-light !text-xs font-semibold"
              @click="emit('open-share')"
            >
              <div class="i-ep-download mr-1"></div>
              资源中心
            </el-button>
          </div>
        </template>
      </el-skeleton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { OverviewSystemInfo } from "@/types";

const props = defineProps<{
  diskPercentage: number;
  loading: boolean;
  sysInfo: OverviewSystemInfo;
}>();

const emit = defineEmits<{
  (event: "open-share"): void;
  (event: "open-tasks"): void;
}>();

const diskBarClass = computed(() => {
  if (props.diskPercentage > 90) return "bg-rose-500";
  if (props.diskPercentage > 70) return "bg-amber-500";
  return "bg-primary";
});
</script>
