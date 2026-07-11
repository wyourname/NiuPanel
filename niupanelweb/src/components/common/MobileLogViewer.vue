<template>
  <div
    class="relative h-full overflow-hidden flex flex-col font-sans text-sm bg-transparent select-text"
  >
    <!-- Floating Loader (Glass) -->
    <transition name="el-fade-in-linear">
      <div
        v-if="loadingTop"
        class="absolute left-1/2 top-3 z-20 flex -translate-x-1/2 select-none items-center gap-1.5 rounded-md border border-light bg-card px-2.5 py-1.5 text-[10px] font-semibold text-secondary shadow-sm"
      >
        <div class="i-ep-loading animate-spin"></div>
        <span>正在加载更早日志</span>
      </div>
    </transition>

    <!-- Native Scroll Container for Mobile -->
    <div
      ref="scrollContainerRef"
      class="flex-1 w-full h-full overflow-y-auto overflow-x-hidden custom-scrollbar !px-1 md:!px-4 py-2 relative touch-pan-y"
      style="overscroll-behavior-y: contain; -webkit-overflow-scrolling: touch"
      @scroll.passive="handleScroll"
    >
      <!-- Top Sentinel for Infinite Load -->
      <div ref="topSentinelRef" class="w-full h-1"></div>

      <div
        v-for="item in chunks"
        :key="item.id"
        class="flex w-full"
        :class="
          isLogSystemMessage(item.content)
            ? 'justify-center my-3'
            : 'justify-start'
        "
      >
        <!-- System Hint Style -->
        <div
          v-if="isLogSystemMessage(item.content)"
          class="rounded-md border border-light bg-subtle px-2 py-1 text-[10px] font-semibold text-muted"
        >
          {{ stripLogUiPrefix(item.content) }}
        </div>

        <!-- Chat Bubble Style for Output -->
        <div
          v-else
          class="relative w-full overflow-hidden border-l-2 border-transparent px-2.5 py-1 font-mono text-[12px] leading-5 break-words transition-colors"
          :class="[
            isWrap ? 'whitespace-pre-wrap' : 'whitespace-pre overflow-x-auto',
            isHighlighted(item.content)
              ? 'border-primary bg-primary/10'
              : '',
          ]"
        >
          <div
            v-html="renderLine(item.content)"
            class="opacity-90"
          ></div>
        </div>
      </div>

      <!-- Scroll anchor -->
      <div ref="bottomAnchorRef" class="w-full h-[1px]"></div>
    </div>

    <!-- Scroll to Bottom Floating Button -->
    <transition name="el-zoom-in-bottom">
      <button
        v-if="!isAtBottom"
        class="absolute bottom-3 right-3 z-30 h-9 w-9 touch-none rounded-lg border border-light bg-card text-secondary shadow-sm flex-center transition-colors hover:bg-soft hover:text-primary"
        title="滚动到底部"
        aria-label="滚动到底部"
        @click="scrollToBottom"
      >
        <div class="i-ep-arrow-down text-[16px]"></div>
        <div
          v-if="hasNewLogs"
          class="absolute -right-0.5 -top-0.5 h-2.5 w-2.5 rounded-full border-2 border-card bg-primary"
        ></div>
      </button>
    </transition>

    <!-- Empty State Overlay -->
    <transition name="fade">
      <div
        v-if="chunks.length === 0 && !loadingTop"
        class="absolute inset-0 flex flex-col items-center justify-center text-muted pointer-events-none z-10"
        :class="loadError ? 'opacity-80' : 'opacity-60'"
      >
        <div class="flex flex-col items-center">
          <div :class="emptyStateIconClass" class="text-[28px] opacity-60"></div>
          <span
            class="mt-2 px-10 text-center text-[11px] font-semibold leading-5"
          >
            {{
              loadError
                ? loadError
                : searchText
                  ? "没有匹配日志"
                  : "等待任务输出"
            }}
          </span>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useLogViewerCore } from "@/composables/useLogViewerCore";
import type { LogUiEvent } from "@/types/logViewer";
import { isLogSystemMessage, stripLogUiPrefix } from "@/utils/logViewer";

type MobileLogViewerProps = {
  maxChunks?: number;
  disableUi?: boolean;
  searchText?: string;
};

const props = withDefaults(defineProps<MobileLogViewerProps>(), {
  maxChunks: 2000,
  disableUi: false,
  searchText: "",
});

const emit = defineEmits<{
  (event: "ui-event", payload: LogUiEvent): void;
}>();

const scrollContainerRef = ref<HTMLElement | null>(null);
const topSentinelRef = ref<HTMLElement | null>(null);
const bottomAnchorRef = ref<HTMLElement | null>(null);

let observer: IntersectionObserver | null = null;

const getScrollElement = () => scrollContainerRef.value;

const scrollToBottom = () => {
  nextTick(() => {
    if (scrollContainerRef.value) {
      scrollContainerRef.value.scrollTop =
        scrollContainerRef.value.scrollHeight;
    }
  });
};

const preserveScrollOnPrepend = (insert: () => void) => {
  const el = scrollContainerRef.value;
  if (!el) {
    insert();
    return;
  }

  const oldScrollHeight = el.scrollHeight;
  const oldScrollTop = el.scrollTop;

  insert();

  nextTick(() => {
    const newScrollHeight = el.scrollHeight;
    el.scrollTop = oldScrollTop + (newScrollHeight - oldScrollHeight);
  });
};

const {
  chunks,
  currentStart,
  handleScroll,
  hasNewLogs,
  init,
  isAtBottom,
  isHighlighted,
  isPullMode,
  isWrap,
  loadError,
  loadOlderLogs,
  loadingTop,
  renderLine,
  reset,
  clear,
  setSearch,
  toggleWrap,
  write,
  writeln,
} = useLogViewerCore({
  disableUi: () => props.disableUi,
  getScrollElement,
  loadErrorMessages: {
    empty: "当前任务没有运行日志",
    failure: "无法加载日志数据",
  },
  maxChunks: () => props.maxChunks,
  onUiEvent: (event) => emit("ui-event", event),
  preserveScrollOnPrepend,
  scrollToBottom,
  searchText: () => props.searchText,
});

const emptyStateIconClass = computed(() =>
  loadError.value ? "i-ep-document" : "i-ep-chat-line-round",
);

onMounted(() => {
  observer = new IntersectionObserver(
    (entries) => {
      if (
        entries[0].isIntersecting &&
        !loadingTop.value &&
        isPullMode.value &&
        currentStart.value > 0
      ) {
        loadOlderLogs();
      }
    },
    { root: scrollContainerRef.value || null, threshold: 0.1 },
  );
});

watch(topSentinelRef, (el) => {
  if (observer) {
    observer.disconnect();
    if (el) observer.observe(el);
  }
});

onUnmounted(() => {
  if (observer) observer.disconnect();
});

defineExpose({
  chunks,
  clear,
  write,
  writeln,
  reset,
  init,
  toggleWrap,
  setSearch,
  scrollToBottom,
});
</script>

<style scoped>
:deep(.log-mark) {
  background-color: #fde047;
  color: #000;
  border-radius: 2px;
}
html.dark :deep(.log-mark) {
  background-color: #a16207;
  color: #fff;
}
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
