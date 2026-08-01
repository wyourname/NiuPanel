<template>
  <transition name="el-zoom-in-top">
    <section
      v-if="visible"
      class="shrink-0 border-b border-primary/15 bg-primary/[0.04] px-3 py-2 dark:bg-primary/[0.08]"
      role="status"
      aria-live="polite"
    >
      <div class="flex items-center gap-3">
        <div class="h-8 w-8 shrink-0 rounded-md bg-primary/10 text-primary flex-center">
          <span class="i-ep-upload-filled text-[16px]"></span>
        </div>
        <div class="min-w-0 flex-1">
          <div class="flex items-center justify-between gap-3 text-[11px]">
            <span class="truncate font-semibold text-default">正在上传 {{ label }}</span>
            <span class="shrink-0 font-mono text-secondary">{{ percentage }}%</span>
          </div>
          <div class="mt-1.5 h-1.5 overflow-hidden rounded-full bg-primary/10">
            <div
              class="h-full rounded-full bg-primary transition-[width] duration-200 motion-reduce:transition-none"
              :style="{ width: `${percentage}%` }"
            ></div>
          </div>
          <p class="mt-1 text-[10px] text-muted">
            {{ sizeLabel }} · 大文件会持续流式写入，不受页面请求超时限制
          </p>
        </div>
        <button
          type="button"
          class="h-8 shrink-0 cursor-pointer rounded-md border border-light px-2.5 text-[11px] font-semibold text-secondary transition-colors hover:border-primary/30 hover:bg-primary/5 hover:text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
          aria-label="取消文件上传"
          @click="emit('cancel')"
        >
          取消
        </button>
      </div>
    </section>
  </transition>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { formatFileSize } from "@/utils/format";

const props = defineProps<{
  label: string;
  loadedBytes: number;
  percentage: number;
  totalBytes: number;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "cancel"): void;
}>();

const sizeLabel = computed(() => {
  const loaded = formatFileSize(props.loadedBytes);
  return props.totalBytes > 0
    ? `${loaded} / ${formatFileSize(props.totalBytes)}`
    : loaded;
});
</script>
