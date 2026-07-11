<template>
  <section
    class="module-shell"
    :class="[
      padded ? 'module-shell--padded' : '',
      centered ? 'module-shell--centered' : '',
      compact ? 'module-shell--compact' : '',
    ]"
  >
    <div v-if="title || eyebrow || $slots.header" class="module-shell__header">
      <div class="module-shell__heading">
        <span v-if="eyebrow" class="module-shell__eyebrow">{{ eyebrow }}</span>
        <h1 v-if="title" class="module-shell__title">{{ title }}</h1>
        <p v-if="description" class="module-shell__description">{{ description }}</p>
        <slot name="header" />
      </div>
      <div v-if="$slots.actions" class="module-shell__actions">
        <slot name="actions" />
      </div>
    </div>
    <div class="module-shell__body">
      <slot />
    </div>
  </section>
</template>

<script setup lang="ts">
import type { VNode } from "vue";

type PageShellProps = {
  padded?: boolean;
  centered?: boolean;
  compact?: boolean;
  title?: string;
  eyebrow?: string;
  description?: string;
};

withDefaults(defineProps<PageShellProps>(), {
  padded: true,
  centered: true,
  compact: false,
  title: "",
  eyebrow: "",
  description: "",
});

defineSlots<{
  default?: () => VNode[];
  header?: () => VNode[];
  actions?: () => VNode[];
}>();
</script>
