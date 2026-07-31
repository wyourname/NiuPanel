<template>
  <div
    class="flex-1 overflow-y-auto custom-scrollbar transition-all duration-300"
    :class="isMobile ? 'px-0 py-1' : 'px-2 py-2'"
    v-infinite-scroll="loadMore"
    :infinite-scroll-disabled="loading || !hasMore"
    :infinite-scroll-distance="40"
  >
    <div
      v-if="loading && variables.length === 0"
      class="overflow-hidden rounded-sm border border-light bg-card"
    >
      <el-skeleton v-for="i in 6" :key="i" animated :loading="true">
        <template #template>
          <div class="flex items-center gap-3 border-b border-light px-3 py-3">
            <el-skeleton-item variant="circle" style="width: 18px; height: 18px" />
            <div class="min-w-0 flex-1">
              <el-skeleton-item variant="text" style="width: 28%" />
              <el-skeleton-item variant="text" style="width: 70%; margin-top: 8px" />
            </div>
            <el-skeleton-item
              variant="text"
              style="width: 24%; height: 24px"
            />
          </div>
        </template>
      </el-skeleton>
    </div>

    <div
      v-else-if="variables.length === 0"
      class="h-full min-h-[320px] flex-col-center opacity-70 text-center"
    >
      <div class="mb-4 h-12 w-12 rounded-md bg-soft text-primary flex-center">
        <div class="i-ep-key text-2xl"></div>
      </div>
      <h3 class="text-base font-semibold text-default">暂无变量</h3>
      <p class="text-sm text-secondary mt-2">
        {{ searchQuery ? "当前搜索没有命中结果" : "可以从右上角创建、导入，或切换变量作用域" }}
      </p>
    </div>

    <TransitionGroup
      v-else
      tag="div"
      name="list"
      class="overflow-hidden rounded-sm border border-light bg-card"
    >
      <VariableCard
        v-for="(row, index) in variables"
        :key="row.id"
        :drag-over="dragOverIndex === index"
        :index="index"
        :row="row"
        :search-query="searchQuery"
        :selected="selectedIds.includes(row.id)"
        :task-names="getTaskNames(row.task_ids)"
        :touch-dragging="touchDragIndex === index"
        :value-loading="isValueLoading(row.id)"
        :value-visible="isValueVisible(row.id)"
        @card-click="emit('card-click', $event)"
        @copy-value="emit('copy-value', $event)"
        @delete="emit('delete', $event)"
        @drag-end="emit('drag-end')"
        @drag-over="emit('drag-over', $event)"
        @drag-start="emit('drag-start', $event)"
        @drop="emit('drop', $event)"
        @edit="emit('edit', $event)"
        @selection-change="emit('selection-change', $event)"
        @status-change="forwardStatusChange"
        @toggle-value="emit('toggle-value', $event)"
        @touch-end="emit('touch-end')"
        @touch-move="emit('touch-move', $event)"
        @touch-start="forwardTouchStart"
      />
    </TransitionGroup>

    <div
      v-if="loading && variables.length > 0"
      class="flex justify-center py-5 text-primary"
    >
      <div class="i-ep-loading animate-spin text-2xl opacity-50"></div>
    </div>

    <div
      v-if="!hasMore && variables.length > 0"
      class="text-center py-4 text-[11px] text-muted"
    >
      已加载全部记录
    </div>
  </div>
</template>

<script setup lang="ts">
import type { VariablePageRow } from "../../composables/useVariablePageData";
import VariableCard from "./VariableCard.vue";

defineProps<{
  dragOverIndex: number | null;
  getTaskNames: (taskIds?: number[]) => string;
  hasMore: boolean;
  isMobile: boolean;
  isValueLoading: (id: number) => boolean;
  isValueVisible: (id: number) => boolean;
  loading: boolean;
  searchQuery: string;
  selectedIds: number[];
  touchDragIndex: number | null;
  variables: VariablePageRow[];
}>();

const emit = defineEmits<{
  (event: "card-click", row: VariablePageRow): void;
  (event: "copy-value", id: number): void;
  (event: "delete", row: VariablePageRow): void;
  (event: "drag-end"): void;
  (event: "drag-over", index: number): void;
  (event: "drag-start", index: number): void;
  (event: "drop", index: number): void;
  (event: "edit", row: VariablePageRow): void;
  (event: "load-more"): void;
  (event: "selection-change", row: VariablePageRow): void;
  (event: "status-change", row: VariablePageRow, value: unknown): void;
  (event: "toggle-value", id: number): void;
  (event: "touch-end"): void;
  (event: "touch-move", touchEvent: TouchEvent): void;
  (event: "touch-start", touchEvent: TouchEvent, index: number): void;
}>();

const loadMore = () => {
  emit("load-more");
};

const forwardStatusChange = (row: VariablePageRow, value: unknown) => {
  emit("status-change", row, value);
};

const forwardTouchStart = (touchEvent: TouchEvent, index: number) => {
  emit("touch-start", touchEvent, index);
};
</script>
