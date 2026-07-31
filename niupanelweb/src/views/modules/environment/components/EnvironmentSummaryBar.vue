<template>
  <div
    class="flex items-center justify-between gap-4 border-b border-light/50 bg-base/30 px-4 py-2.5 md:px-5"
  >
    <div class="flex items-center gap-3 min-w-0">
      <div class="h-8 w-8 shrink-0 rounded-md bg-soft text-primary flex-center">
        <div class="i-ep-box text-base"></div>
      </div>
      <div class="min-w-0">
        <div class="text-[13px] font-bold text-default truncate">
          {{ title }}
        </div>
        <div class="text-[10px] font-medium text-muted">
          共 {{ count }} 个条目
        </div>
      </div>
    </div>
    <div v-if="!isMobile" class="text-[11px] text-muted italic">
      安装、卸载和镜像操作均保留日志
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { EnvType } from "@/types";

const props = defineProps<{
  count: number;
  filterType: EnvType;
  isMobile: boolean;
}>();

const title = computed(() => {
  if (props.filterType === "node") return "Node.js 运行环境";
  if (props.filterType === "python") return "Python 运行环境";
  return "系统环境";
});
</script>
