<template>
  <div class="flex h-full min-h-[196px] flex-col">
    <div
      ref="viewport"
      class="min-h-0 flex-1 snap-x snap-mandatory overflow-x-auto overflow-y-hidden no-scrollbar focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/35"
      role="region"
      :tabindex="pages.length > 1 ? 0 : -1"
      :aria-label="label"
      @keydown.left.prevent="goToPage(currentPage - 1)"
      @keydown.right.prevent="goToPage(currentPage + 1)"
      @scroll.passive="syncPageFromScroll"
    >
      <div class="flex h-full">
        <section
          v-for="(page, pageIndex) in pages"
          :key="pageIndex"
          class="grid h-full min-w-full snap-start snap-always grid-cols-2 grid-rows-5 content-start gap-1 pr-px"
          :aria-hidden="pageIndex !== currentPage"
          :aria-label="`第 ${pageIndex + 1} 页，共 ${pages.length} 页`"
          :inert="pageIndex !== currentPage"
        >
          <template v-for="task in page" :key="task.id">
            <slot name="item" :task="task" />
          </template>
        </section>
      </div>
    </div>

    <nav
      v-if="pages.length > 1"
      class="mt-2 flex shrink-0 items-center justify-between border-t border-light/70 pt-2"
      :aria-label="`${label}分页`"
    >
      <button
        type="button"
        class="h-7 w-7 cursor-pointer rounded-md text-muted flex-center transition-colors hover:bg-soft hover:text-default focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 disabled:cursor-not-allowed disabled:opacity-35"
        :disabled="currentPage === 0"
        aria-label="上一页"
        @click="goToPage(currentPage - 1)"
      >
        <span class="i-ep-arrow-left text-[13px]"></span>
      </button>

      <div class="flex items-center gap-0.5">
        <button
          v-for="(_, pageIndex) in pages"
          :key="pageIndex"
          type="button"
          class="group h-6 w-6 cursor-pointer rounded-md flex-center focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
          :aria-current="pageIndex === currentPage ? 'page' : undefined"
          :aria-label="`转到第 ${pageIndex + 1} 页`"
          @click="goToPage(pageIndex)"
        >
          <span
            class="h-1.5 rounded-full transition-[width,background-color] duration-200 motion-reduce:transition-none"
            :class="pageIndex === currentPage ? 'w-5 bg-primary' : 'w-1.5 bg-muted/35 group-hover:bg-muted/60'"
          ></span>
        </button>
      </div>

      <div class="flex items-center gap-2">
        <span class="font-mono text-[9px] tabular-nums text-muted" aria-live="polite">
          {{ currentPage + 1 }}/{{ pages.length }}
        </span>
        <button
          type="button"
          class="h-7 w-7 cursor-pointer rounded-md text-muted flex-center transition-colors hover:bg-soft hover:text-default focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 disabled:cursor-not-allowed disabled:opacity-35"
          :disabled="currentPage === pages.length - 1"
          aria-label="下一页"
          @click="goToPage(currentPage + 1)"
        >
          <span class="i-ep-arrow-right text-[13px]"></span>
        </button>
      </div>
    </nav>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import type { Task } from "@/types";

const PAGE_SIZE = 10;

const props = defineProps<{
  items: Task[];
  label: string;
}>();

defineSlots<{
  item(props: { task: Task }): unknown;
}>();

const viewport = ref<HTMLElement | null>(null);
const currentPage = ref(0);
const pages = computed(() => {
  const result: Task[][] = [];
  for (let index = 0; index < props.items.length; index += PAGE_SIZE) {
    result.push(props.items.slice(index, index + PAGE_SIZE));
  }
  return result;
});

const goToPage = (page: number) => {
  const target = Math.max(0, Math.min(page, pages.value.length - 1));
  const element = viewport.value;
  currentPage.value = target;
  if (!element) return;
  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  element.scrollTo({
    left: target * element.clientWidth,
    behavior: reduceMotion ? "auto" : "smooth",
  });
};

const syncPageFromScroll = () => {
  const element = viewport.value;
  if (!element?.clientWidth) return;
  currentPage.value = Math.max(
    0,
    Math.min(Math.round(element.scrollLeft / element.clientWidth), pages.value.length - 1),
  );
};

watch(
  () => props.items.length,
  () => {
    if (currentPage.value >= pages.value.length) {
      currentPage.value = Math.max(0, pages.value.length - 1);
    }
    nextTick(() => goToPage(currentPage.value));
  },
);
</script>
