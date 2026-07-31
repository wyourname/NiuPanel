<template>
  <section class="glass-card flex min-h-[172px] flex-col p-4">
    <header class="mb-2 flex h-6 shrink-0 items-center justify-between">
      <h2 class="text-[12px] font-semibold text-secondary">{{ title }}</h2>
      <span
        v-if="count !== undefined"
        class="min-w-[20px] rounded-full px-1.5 py-0.5 text-center text-[10px] font-bold"
        :class="badgeClass"
      >{{ count }}</span>
    </header>
    <div class="min-h-0 flex-1 overflow-y-auto no-scrollbar">
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
}>();

const badgeClass = computed(() =>
  props.tone === "warning" && (props.count ?? 0) > 0
    ? "warning-subtle"
    : "bg-subtle text-muted",
);
</script>
