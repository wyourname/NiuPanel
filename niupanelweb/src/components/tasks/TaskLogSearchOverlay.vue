<template>
  <transition name="el-zoom-in-top">
    <div
      v-if="visible"
      class="absolute top-4 left-1/2 -translate-x-1/2 w-full max-w-lg z-40 px-4"
    >
      <div
        class="flex items-center gap-3 rounded-md border border-primary/20 bg-card p-2 shadow-sm"
      >
        <div class="i-ep-search text-primary ml-2"></div>
        <input
          ref="inputRef"
          v-model="queryValue"
          placeholder="在输出中搜索..."
          class="flex-1 bg-transparent border-none outline-none text-xs font-bold text-default h-8"
          @keyup.esc="visibleValue = false"
        />
        <button
          type="button"
          class="h-7 w-7 rounded-md text-muted flex-center transition-colors hover:bg-soft hover:text-default"
          aria-label="关闭日志搜索"
          @click="visibleValue = false"
        >
          <div class="i-ep-close"></div>
        </button>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";

const props = defineProps<{
  query: string;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "update:query", query: string): void;
  (event: "update:visible", visible: boolean): void;
}>();

const inputRef = ref<HTMLInputElement | null>(null);

const queryValue = computed({
  get: () => props.query,
  set: (value: string) => emit("update:query", value),
});

const visibleValue = computed({
  get: () => props.visible,
  set: (value: boolean) => emit("update:visible", value),
});

defineExpose({
  focus: () => inputRef.value?.focus(),
});
</script>
