<template>
  <section class="desktop-widget glass-card flex min-h-[172px] flex-col p-4" :data-tone="tone">
    <header class="mb-2 flex h-6 shrink-0 items-center justify-between">
      <h2 class="text-[13px] font-semibold text-default">{{ title }}</h2>
      <span
        v-if="count !== undefined"
        class="min-w-[20px] rounded-full px-1.5 py-0.5 text-center text-[10px] font-bold"
        :class="badgeClass"
      >{{ count }}</span>
    </header>
    <div class="min-h-0 flex-1" :class="contentClass || 'overflow-y-auto no-scrollbar'">
      <slot />
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  title: string;
  count?: number;
  tone?: "default" | "warning";
  contentClass?: string;
}>();

const badgeClass = computed(() =>
  props.tone === "warning" && (props.count ?? 0) > 0
    ? "warning-subtle"
    : "bg-subtle text-muted",
);
</script>

<style scoped>
.desktop-widget { position: relative; border-radius: var(--radius-lg); }
.desktop-widget > header { padding-bottom: 12px; margin-bottom: 12px; height: auto; border-bottom: 1px solid var(--border-light); }
.desktop-widget > header h2 { display: flex; align-items: center; gap: 8px; }
.desktop-widget > header h2::before { content: ''; width: 3px; height: 12px; border-radius: 2px; background: var(--el-color-primary); }
.desktop-widget[data-tone="warning"] > header h2::before { background: var(--warning-subtle-text); }
</style>
