<template>
  <ResponsiveDialog
    v-model:visible="visible"
    :title="title"
    desktop-size="xl"
    content-preset="workspace"
    size="100%"
    mobile-mode="fullscreen"
    destroy-on-close
    append-to-body
  >
    <template #header-actions>
      <button
        type="button"
        class="mobile-touch-target cursor-pointer rounded-md px-2 text-[10px] font-semibold text-secondary flex-center gap-1.5 transition-colors hover:bg-soft hover:text-default"
        title="清空当前日志视图"
        aria-label="清空当前日志视图"
        @click="clear"
      >
        <span class="i-ep-delete" aria-hidden="true"></span>
        <span class="hidden sm:inline">清空</span>
      </button>
    </template>

    <div
      class="environment-log-shell flex h-full min-h-[420px] flex-col overflow-hidden bg-[var(--editor-bg)] md:h-[min(640px,72vh)]"
    >
      <div class="relative min-h-0 flex-1 overflow-hidden">
        <LogViewer
          ref="logViewerRef"
          :is-mobile="isMobile"
          compact
          class="min-h-0 flex-1"
        />
      </div>
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import LogViewer from "../../../../components/common/LogViewer.vue";
import ResponsiveDialog from "../../../../components/common/ResponsiveDialog.vue";
import type { LogFetcher, LogViewerRef, LogViewerWriteInput } from "@/types";

const props = defineProps<{
  isMobile: boolean;
  modelValue: boolean;
  title: string;
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
}>();

const logViewerRef = ref<LogViewerRef | null>(null);

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit("update:modelValue", value),
});

const clear = () => {
  logViewerRef.value?.clear?.();
};

const reset = () => {
  logViewerRef.value?.reset?.();
};

const write = (data: LogViewerWriteInput) => {
  logViewerRef.value?.write?.(data);
};

const writeln = (data: string) => {
  logViewerRef.value?.writeln?.(data);
};

const init = (loader: LogFetcher) => {
  return logViewerRef.value?.init?.(loader);
};

const scrollToBottom = () => {
  logViewerRef.value?.scrollToBottom?.();
};

const setSearch = (query: string, onlyShowMatches?: boolean) => {
  logViewerRef.value?.setSearch?.(query, onlyShowMatches);
};

const toggleWrap = () => {
  logViewerRef.value?.toggleWrap?.();
};

defineExpose({
  clear,
  init,
  reset,
  scrollToBottom,
  setSearch,
  toggleWrap,
  write,
  writeln,
});
</script>
