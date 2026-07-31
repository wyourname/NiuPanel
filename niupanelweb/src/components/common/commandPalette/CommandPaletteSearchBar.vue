<template>
  <div class="search-bar">
    <el-icon class="search-icon"><Search /></el-icon>
    <input
      ref="searchInput"
      :value="modelValue"
      type="text"
      placeholder="搜索任务、页面或命令..."
      class="search-input"
      @input="emit('update:modelValue', inputValue($event))"
      @keydown.down.prevent="emit('navigate-down')"
      @keydown.up.prevent="emit('navigate-up')"
      @keydown.enter.prevent="emit('select-active')"
      @keydown.esc.prevent="emit('close')"
    />
    <span class="esc-hint">ESC</span>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { Search } from "@element-plus/icons-vue";

defineProps<{
  modelValue: string;
}>();

const emit = defineEmits<{
  (event: "close"): void;
  (event: "navigate-down"): void;
  (event: "navigate-up"): void;
  (event: "select-active"): void;
  (event: "update:modelValue", value: string): void;
}>();

const searchInput = ref<HTMLInputElement | null>(null);

const inputValue = (event: Event) =>
  event.target instanceof HTMLInputElement ? event.target.value : "";

defineExpose({
  focus: () => searchInput.value?.focus(),
});
</script>

<style scoped>
.search-bar {
  display: flex;
  align-items: center;
  padding: 16px;
  border-bottom: 1px solid var(--border-base);
}

@media (max-width: 768px) {
  .search-bar {
    padding: 12px 16px;
    padding-top: calc(12px + env(safe-area-inset-top));
  }
}

.search-icon {
  font-size: 20px;
  color: var(--text-secondary);
  margin-right: 12px;
}

.search-input {
  flex: 1;
  border: none;
  outline: none;
  font-size: 16px;
  color: var(--text-default);
  background: transparent;
}

.search-input::placeholder {
  color: var(--text-muted);
}

.esc-hint {
  font-size: 12px;
  color: var(--text-secondary);
  background: var(--bg-base);
  padding: 2px 6px;
  border-radius: 4px;
  border: 1px solid var(--border-base);
}

@media (max-width: 768px) {
  .esc-hint {
    display: none;
  }
}
</style>
