<template>
  <div class="segmented-tabs" :class="{ 'segmented-tabs--full': fullWidth }">
    <button
      v-for="item in items"
      :key="item.value"
      type="button"
      class="segmented-tabs__item"
      :class="{ 'is-active': modelValue === item.value }"
      @click="emit('update:modelValue', item.value)"
    >
      <span class="segmented-tabs__label">{{ item.label }}</span>
      <span v-if="item.meta" class="segmented-tabs__meta">{{ item.meta }}</span>
    </button>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  modelValue: string;
  items: Array<{ label: string; value: string; meta?: string }>;
  fullWidth?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();
</script>
