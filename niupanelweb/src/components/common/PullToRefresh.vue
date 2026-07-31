<template>
  <div
    ref="containerRef"
    class="pull-to-refresh-container"
    :style="containerStyle"
    @touchstart="handleTouchStart"
    @touchmove="handleTouchMove"
    @touchend="handleTouchEnd"
  >
    <!-- Pull Indicator -->
    <div class="pull-to-refresh-indicator" :style="indicatorStyle">
      <div v-if="state === 'pulling'" class="flex flex-col items-center gap-1">
        <div
          class="i-ep-bottom transition-transform duration-200"
          :style="{ transform: `rotate(${pullProgress * 180}deg)` }"
        ></div>
        <span class="text-[10px] font-semibold opacity-60"
          >下拉刷新</span
        >
      </div>
      <div
        v-else-if="state === 'ready'"
        class="flex flex-col items-center gap-1"
      >
        <div
          class="i-ep-refresh transition-transform duration-200 rotate-180"
        ></div>
        <span
          class="text-[10px] font-semibold text-primary"
          >释放刷新</span
        >
      </div>
      <div
        v-else-if="state === 'refreshing'"
        class="flex flex-col items-center gap-1"
      >
        <div class="i-ep-loading animate-spin text-primary"></div>
        <span
          class="text-[10px] font-semibold text-primary"
          >正在刷新</span
        >
      </div>
    </div>

    <!-- Content -->
    <slot />
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";

type PullToRefreshProps = {
  onRefresh: () => unknown;
  threshold?: number;
  disabled?: boolean;
};

const props = withDefaults(defineProps<PullToRefreshProps>(), {
  threshold: 60,
  disabled: false,
});

const containerRef = ref<HTMLElement | null>(null);
const state = ref<"idle" | "pulling" | "ready" | "refreshing">("idle");
const startY = ref(0);
const currentY = ref(0);
const pullDistance = ref(0);

const pullProgress = computed(() => {
  return Math.min(pullDistance.value / props.threshold, 1);
});

const containerStyle = computed(() => {
  if (state.value === "refreshing") {
    return {
      transform: `translateY(${props.threshold}px)`,
      transition: "transform 0.3s cubic-bezier(0.2, 0.8, 0.2, 1)",
    };
  }
  if (state.value === "pulling" || state.value === "ready") {
    return {
      transform: `translateY(${pullDistance.value}px)`,
      transition: "none",
    };
  }
  return {
    transform: "translateY(0)",
    transition: "transform 0.3s cubic-bezier(0.2, 0.8, 0.2, 1)",
  };
});

const indicatorStyle = computed(() => {
  return {
    opacity: pullProgress.value,
    transform: `translateY(-100%) scale(${0.5 + pullProgress.value * 0.5})`,
    height: `${props.threshold}px`,
  };
});

const handleTouchStart = (e: TouchEvent) => {
  if (props.disabled || state.value === "refreshing") return;

  // Only trigger pull to refresh when at the top of the scrollable area
  // Use e.target to find the actual scrollable container (which might be a child)
  const scrollParent = getScrollParent(
    e.target instanceof HTMLElement ? e.target : null,
  );
  if (scrollParent && scrollParent.scrollTop > 0) return;

  startY.value = e.touches[0].pageY;
  state.value = "idle";
};

const handleTouchMove = (e: TouchEvent) => {
  if (props.disabled || state.value === "refreshing") return;
  if (startY.value === 0) return;

  currentY.value = e.touches[0].pageY;
  const diff = currentY.value - startY.value;

  if (diff > 0) {
    // Apply resistance
    pullDistance.value = Math.pow(diff, 0.8);

    if (pullDistance.value > 0) {
      if (e.cancelable) e.preventDefault();
      state.value = pullDistance.value > props.threshold ? "ready" : "pulling";
    }
  }
};

const handleTouchEnd = async () => {
  if (props.disabled || state.value === "refreshing") return;

  if (state.value === "ready") {
    state.value = "refreshing";
    try {
      await props.onRefresh();
    } finally {
      state.value = "idle";
      pullDistance.value = 0;
      startY.value = 0;
    }
  } else {
    state.value = "idle";
    pullDistance.value = 0;
    startY.value = 0;
  }
};

const getScrollParent = (node: HTMLElement | null): HTMLElement | null => {
  if (!node) return null;
  if (node.scrollHeight > node.clientHeight) {
    const overflowY = window.getComputedStyle(node).overflowY;
    if (overflowY === "auto" || overflowY === "scroll") return node;
  }
  return getScrollParent(node.parentElement) || document.documentElement;
};
</script>

<style scoped>
.pull-to-refresh-container {
  position: relative;
  height: 100%;
  width: 100%;
  will-change: transform;
}

.pull-to-refresh-indicator {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  z-index: 1;
}
</style>
