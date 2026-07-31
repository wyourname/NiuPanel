<template>
  <component
    :is="isMobile ? 'el-drawer' : 'el-dialog'"
    v-model="visible"
    :title="title"
    :size="isMobile ? '100%' : '800px'"
    :width="isMobile ? '100%' : '800px'"
    :align-center="!isMobile"
    :with-header="!isMobile"
    direction="btt"
    destroy-on-close
    append-to-body
    class="log-modal"
  >
    <div class="flex-1 flex flex-col overflow-hidden bg-[var(--editor-bg)]">
      <div
        v-if="isMobile"
        class="flex items-center justify-between p-3 border-b border-base shrink-0 bg-base"
      >
        <div class="flex items-center gap-2 overflow-hidden">
          <div
            class="i-ep-back text-xl text-muted cursor-pointer"
            @click="visible = false"
          ></div>
          <span class="font-bold truncate text-default">{{ title }}</span>
        </div>
        <div
          class="i-ep-delete text-xl text-red-500 cursor-pointer"
          @click="clear"
        ></div>
      </div>

      <div
        v-else
        class="flex items-center justify-end px-4 border-b border-[var(--editor-border)] shrink-0 h-12"
      >
        <el-button size="small" @click="clear">清空</el-button>
      </div>

      <div class="flex-1 overflow-hidden relative">
        <LogViewer ref="logViewerRef" :is-mobile="isMobile" />
      </div>
    </div>
  </component>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import LogViewer from "../../../../components/common/LogViewer.vue";
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
