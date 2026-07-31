<template>
  <button
    type="button"
    class="toolbar-button"
    :class="[
      variant ? `toolbar-button--${variant}` : '',
      `toolbar-button--${size}`,
      block ? 'toolbar-button--block' : '',
    ]"
    :disabled="disabled"
  >
    <span v-if="$slots.icon" class="toolbar-button__icon">
      <slot name="icon" />
    </span>
    <span class="toolbar-button__label">
      <slot />
    </span>
  </button>
</template>

<script setup lang="ts">
import type { VNode } from "vue";

type ToolbarButtonVariant = "default" | "primary" | "soft" | "danger";
type ToolbarButtonSize = "small" | "default" | "large";

type ToolbarButtonProps = {
  variant?: ToolbarButtonVariant;
  size?: ToolbarButtonSize;
  block?: boolean;
  disabled?: boolean;
};

withDefaults(defineProps<ToolbarButtonProps>(), {
  variant: "default",
  size: "default",
  block: false,
  disabled: false,
});

defineSlots<{
  default?: () => VNode[];
  icon?: () => VNode[];
}>();
</script>
