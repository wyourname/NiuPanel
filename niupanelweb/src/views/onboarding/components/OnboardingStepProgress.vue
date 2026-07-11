<template>
  <div class="mb-5 flex items-center justify-center gap-2 sm:mb-6">
    <div v-for="(item, index) in steps" :key="item" class="flex items-center gap-2">
      <div
        class="h-8 w-8 rounded-full border-2 text-xs font-bold flex-center transition-colors duration-200"
        :class="[
          step > index
            ? 'bg-primary border-primary text-white'
            : step === index
              ? 'border-primary text-primary bg-white dark:bg-dark-900'
              : 'border-gray-200 dark:border-gray-700 text-muted bg-white dark:bg-dark-900',
        ]"
      >
        <div v-if="step > index" class="i-ep-check text-sm"></div>
        <span v-else>{{ index + 1 }}</span>
      </div>
      <div
        v-if="index < steps.length - 1"
        class="h-0.5 w-8 transition-colors duration-200 sm:w-12"
        :class="step > index ? 'bg-primary' : 'bg-gray-200 dark:bg-gray-700'"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  step: number;
  steps: readonly number[];
}>();
</script>
