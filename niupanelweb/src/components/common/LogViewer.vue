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

    <!-- Log Scroller -->
    <div class="flex-1 overflow-hidden relative">
      <DynamicScroller
        ref="scrollerRef"
        :items="filteredChunks"
        :min-item-size="compact ? 20 : 24"
        :buffer="800"
        class="h-full custom-scrollbar"
        :class="compact ? '!px-1 !py-1' : '!px-1 py-2 md:!px-4'"
        key-field="id"
        @scroll.passive="handleScroll"
      >
        <template #before>
          <div
            ref="topSentinelRef"
            class="w-full"
            :class="compact ? 'h-2' : 'h-4'"
          ></div>
        </template>
        <template #default="{ item, index, active }">
          <DynamicScrollerItem
            :item="item"
            :active="active"
            :size-dependencies="[item.content, isWrap]"
            :data-index="index"
          >
            <div
              class="flex w-full"
              :class="
                isLogSystemMessage(item.content)
                  ? 'justify-center my-3'
                  : 'justify-start'
              "
            >
              <!-- System Hint Style (Service Messages) -->
              <div
                v-if="isLogSystemMessage(item.content)"
                class="rounded-md border border-light bg-subtle px-2 py-1 text-[10px] font-semibold text-muted"
              >
                {{ stripLogUiPrefix(item.content) }}
              </div>

              <!-- Chat Bubble Style for Output -->
              <div
                v-else
                class="relative w-full overflow-hidden border-l-2 border-transparent font-mono text-[12px] leading-5 break-words transition-colors hover:bg-soft/45"
                :class="[
                  compact ? 'px-1.5 py-0.5' : 'px-2.5 py-1',
                  isWrap
                    ? 'whitespace-pre-wrap'
                    : 'whitespace-pre overflow-x-auto',
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
          </DynamicScrollerItem>
        </template>
      </DynamicScroller>

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
          class="absolute inset-0 z-10 flex flex-col items-center justify-center text-muted pointer-events-none"
        >
          <div class="flex flex-col items-center">
            <div class="i-ep-document text-[28px] opacity-35"></div>
            <span class="mt-2 text-[11px] font-semibold opacity-60">{{
              searchText ? "没有匹配日志" : "等待任务输出"
            }}</span>
          </div>
        </div>
      </transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { DynamicScroller, DynamicScrollerItem } from "vue-virtual-scroller";
import "vue-virtual-scroller/dist/vue-virtual-scroller.css";
import { useLogViewerCore } from "@/composables/useLogViewerCore";
import type { LogUiEvent } from "@/types/logViewer";
import { isLogSystemMessage, stripLogUiPrefix } from "@/utils/logViewer";

type LogViewerProps = {
  maxChunks?: number;
  isMobile?: boolean;
  disableUi?: boolean;
  compact?: boolean;
};

type DynamicScrollerRef = {
  $el?: HTMLElement;
  forceUpdate?: () => void;
  scrollToBottom?: () => void;
};

const props = withDefaults(defineProps<LogViewerProps>(), {
  maxChunks: 10000,
  isMobile: false,
  disableUi: false,
  compact: false,
});

const emit = defineEmits<{
  (event: "ui-event", payload: LogUiEvent): void;
}>();

const scrollerRef = ref<DynamicScrollerRef | null>(null);
const topSentinelRef = ref<HTMLElement | null>(null);
let observer: IntersectionObserver | null = null;

const getScrollElement = () => scrollerRef.value?.$el ?? null;

const scrollToBottom = () => {
  nextTick(() => {
    scrollerRef.value?.scrollToBottom?.();
  });
};

const preserveScrollOnPrepend = (insert: () => void) => {
  const el = getScrollElement();
  if (!el) {
    insert();
    return;
  }

  const oldScrollHeight = el.scrollHeight;
  const oldScrollTop = el.scrollTop;

  insert();

  nextTick(() => {
    const currentEl = getScrollElement();
    if (!currentEl) return;

    const newScrollHeight = currentEl.scrollHeight;
    currentEl.scrollTop = oldScrollTop + (newScrollHeight - oldScrollHeight);
  });
};

const {
  chunks,
  currentStart,
  filteredChunks,
  handleScroll,
  hasNewLogs,
  init,
  isAtBottom,
  isHighlighted,
  isPullMode,
  isWrap,
  loadOlderLogs,
  loadingTop,
  renderLine,
  reset,
  clear,
  searchText,
  setSearch,
  toggleWrap,
  write,
  writeln,
} = useLogViewerCore({
  disableUi: () => props.disableUi,
  getScrollElement,
  maxChunks: () => props.maxChunks,
  onUiEvent: (event) => emit("ui-event", event),
  onWrapChange: () => {
    nextTick(() => {
      scrollerRef.value?.forceUpdate?.();
    });
  },
  preserveScrollOnPrepend,
  scrollToBottom,
});

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
    { root: null, threshold: 0.1 },
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
