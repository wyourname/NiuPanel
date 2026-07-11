<template>
  <div v-if="items.length > 0" class="results-list custom-scrollbar">
    <div
      v-for="(item, index) in items"
      :key="`${item.type}-${item.title}-${index}`"
      class="result-item"
      :class="{ 'is-active': index === activeIndex }"
      @click="emit('select', index)"
      @mouseenter="emit('hover', index)"
    >
      <div class="item-icon">
        <el-icon v-if="item.type === 'nav' && item.icon">
          <component :is="item.icon" />
        </el-icon>
        <el-icon v-else-if="item.type === 'task'"><List /></el-icon>
        <el-icon v-else-if="item.type === 'variable'"><Key /></el-icon>
        <el-icon v-else-if="item.type === 'env'"><Box /></el-icon>
        <el-icon v-else-if="item.type === 'command'"><Operation /></el-icon>
      </div>
      <div class="item-content">
        <div class="item-title">{{ item.title }}</div>
        <div v-if="item.desc" class="item-desc">{{ item.desc }}</div>
      </div>
      <div v-if="item.actionText" class="item-action">
        {{ item.actionText }}
      </div>
    </div>
  </div>

  <div v-else-if="searchQuery" class="no-results">未找到相关结果</div>
</template>

<script setup lang="ts">
import { Box, Key, List, Operation } from "@element-plus/icons-vue";
import type { PaletteItem } from "./types";

defineProps<{
  activeIndex: number;
  items: PaletteItem[];
  searchQuery: string;
}>();

const emit = defineEmits<{
  (event: "hover", index: number): void;
  (event: "select", index: number): void;
}>();
</script>

<style scoped>
.results-list {
  max-height: 400px;
  overflow-y: auto;
  padding: 8px;
  flex: 1;
}

@media (max-width: 768px) {
  .results-list {
    max-height: none;
  }
}

.result-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.1s;
}

.result-item.is-active {
  background: var(--el-color-primary-light-9);
}

.item-icon {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
  margin-right: 12px;
  background: var(--bg-base);
  border-radius: 6px;
}

.result-item.is-active .item-icon {
  color: var(--el-color-primary);
  background: var(--bg-card);
}

.item-content {
  flex: 1;
  overflow: hidden;
}

.item-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-default);
}

.item-desc {
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.no-results {
  padding: 32px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 14px;
}
</style>
