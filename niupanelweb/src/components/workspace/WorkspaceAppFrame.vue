<template>
  <section
    class="flex h-full min-h-0 overflow-hidden bg-card text-default"
    :class="rootClass"
  >
    <aside
      v-if="$slots.sidebar"
      class="shrink-0 border-r border-light bg-subtle dark:bg-white/[0.025]"
      :class="sidebarClass"
    >
      <slot name="sidebar" />
    </aside>

    <div class="flex min-w-0 flex-1 flex-col overflow-hidden">
      <header
        v-if="$slots.toolbar"
        class="shrink-0 border-b border-light bg-card px-3 py-2 dark:bg-white/[0.035]"
        :class="toolbarClass"
      >
        <slot name="toolbar" />
      </header>

      <main
        class="flex min-h-0 flex-1 flex-col overflow-hidden bg-card"
        :class="contentClass"
      >
        <slot />
      </main>

      <footer
        v-if="$slots.status"
        class="shrink-0 border-t border-light bg-subtle px-3 py-1.5 text-[11px] font-medium text-muted dark:bg-white/[0.025]"
        :class="statusClass"
      >
        <slot name="status" />
      </footer>
    </div>
  </section>
</template>

<script setup lang="ts">
import type { VNode } from "vue";

withDefaults(
  defineProps<{
    contentClass?: string;
    rootClass?: string;
    sidebarClass?: string;
    statusClass?: string;
    toolbarClass?: string;
  }>(),
  {
    contentClass: "",
    rootClass: "",
    sidebarClass: "",
    statusClass: "",
    toolbarClass: "",
  },
);

defineSlots<{
  default?: () => VNode[];
  sidebar?: () => VNode[];
  status?: () => VNode[];
  toolbar?: () => VNode[];
}>();
</script>
