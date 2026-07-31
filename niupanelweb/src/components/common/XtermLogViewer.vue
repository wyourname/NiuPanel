<template>
  <div
    class="h-full w-full bg-[#1e293b] overflow-hidden relative group rounded-lg custom-xterm-container"
  >
    <div ref="terminalRef" class="h-full w-full"></div>

    <!-- Copy Button Overlay -->
    <div
      class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10"
    >
      <button
        class="p-1.5 rounded-md bg-white/10 hover:bg-white/20 text-gray-300 transition-colors"
        @click="copyAll"
        title="复制全部"
      >
        <div class="i-ep-document-copy"></div>
      </button>
    </div>

    <!-- Empty State Overlay -->
    <div
      v-if="isEmpty"
      class="absolute inset-0 flex items-center justify-center pointer-events-none select-none"
    >
      <div class="flex flex-col items-center gap-2 opacity-40">
        <div class="i-ep-monitor text-4xl text-gray-400"></div>
        <span class="text-sm font-mono text-gray-400"
          >Waiting for task output...</span
        >
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import "@xterm/xterm/css/xterm.css";
import { useXtermLogViewer } from "@/composables/useXtermLogViewer";

type XtermLogViewerProps = {
  isMobile?: boolean;
  fontSize?: number;
};

const props = withDefaults(defineProps<XtermLogViewerProps>(), {
  isMobile: false,
  fontSize: 13,
});

const terminalRef = ref<HTMLElement | null>(null);
const {
  chunks,
  clear,
  copyAll,
  fit,
  init,
  isEmpty,
  reset,
  scrollToBottom,
  setSearch,
  write,
  writeln,
} = useXtermLogViewer({
  fontSize: () => props.fontSize,
  isMobile: () => props.isMobile,
  terminalRef,
});

// Expose API compatible with LogViewer
defineExpose({
  write,
  writeln,
  clear,
  reset,
  scrollToBottom,
  setSearch,
  fit,
  init,
  chunks,
});
</script>

<style scoped>
/* Custom container to handle padding and border radius */
.custom-xterm-container {
  box-shadow: inset 0 0 20px rgba(0, 0, 0, 0.2);
}

/* XTerm Scrollbar Customization */
:deep(.xterm-viewport::-webkit-scrollbar) {
  width: 14px; /* Larger */
}
:deep(.xterm-viewport::-webkit-scrollbar-track) {
  background: transparent;
}
:deep(.xterm-viewport::-webkit-scrollbar-thumb) {
  background-color: rgba(255, 255, 255, 0.15); /* Dark grey/transparent */
  border-radius: 4px;
}
:deep(.xterm-viewport::-webkit-scrollbar-thumb:hover) {
  background-color: rgba(255, 255, 255, 0.25);
}
</style>
