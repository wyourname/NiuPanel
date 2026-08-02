<template>
  <div
    class="variable-card flex items-center gap-4 px-4 py-3 bg-card active:bg-soft transition-colors border-b border-light/50"
    :class="[
      selected ? 'bg-soft ring-1 ring-primary/25' : '',
      dragOver ? 'ring-2 ring-primary/20 ring-inset' : '',
      row.enabled ? '' : 'opacity-70',
      touchDragging ? 'scale-[0.99] bg-soft' : '',
    ]"
    :draggable="!searchQuery"
    :data-index="index"
    @click="emit('card-click', row)"
    @dragstart="emit('drag-start', index)"
    @dragover.prevent="emit('drag-over', index)"
    @drop.prevent="emit('drop', index)"
    @dragend="emit('drag-end')"
    @touchstart="emit('touch-start', $event, index)"
    @touchmove="emit('touch-move', $event)"
    @touchend="emit('touch-end')"
  >
    <button
      v-if="!searchQuery"
      type="button"
      class="hidden h-7 w-7 shrink-0 cursor-grab items-center justify-center rounded-md text-muted transition-colors hover:bg-base hover:text-default active:cursor-grabbing md:flex"
      title="拖拽排序"
      @click.stop
    >
      <div class="i-ep-rank text-xs"></div>
    </button>
    <span v-else class="hidden h-7 w-7 md:block"></span>

    <el-checkbox
      :model-value="selected"
      class="!mr-0"
      size="small"
      @change="emit('selection-change', row)"
      @click.stop
    />

    <div class="w-1.5 h-1.5 rounded-full shrink-0 md:hidden" :class="row.enabled ? 'bg-emerald-500' : 'bg-muted'"></div>

    <div class="flex-1 min-w-0 flex flex-col justify-center">
      <div class="flex items-center gap-2">
        <span class="font-mono text-[14px] font-bold text-default truncate">{{ row.key }}</span>
        <span
          v-if="row.scope === 'Script'"
          class="hidden max-w-[180px] shrink-0 items-center gap-1 rounded-md bg-soft px-1.5 py-0.5 text-[10px] font-semibold text-primary md:inline-flex"
        >
          <div class="i-ep-link text-[10px]"></div>
          <span class="truncate">{{ taskNames }}</span>
        </span>
      </div>
      <div class="mt-0.5 flex items-center gap-1.5 truncate text-[11px] text-muted">
        <span class="font-mono">#{{ row.id }}</span>
        <span>·</span>
        <span>{{ row.remarks || (valueVisible ? (row.value ?? "").substring(0, 20) : "••••••••••••••••") }}</span>
      </div>
    </div>

    <div class="hidden min-w-0 items-center gap-1.5 md:flex shrink-0 w-1/3">
      <button
        type="button"
        class="flex min-w-0 flex-1 items-center gap-1.5 rounded-md border border-light bg-base/60 px-2.5 py-1.5 text-left transition-colors hover:bg-base"
        title="显示或隐藏变量值"
        :disabled="valueLoading"
        @click.stop="emit('toggle-value', row.id)"
      >
        <div class="i-ep-lock shrink-0 text-[11px] text-primary/45"></div>
        <span class="min-w-0 truncate font-mono text-[11px] text-default opacity-80">
          {{ valueVisible ? row.value ?? "" : "••••••••••••••••" }}
        </span>
      </button>
      <button
        type="button"
        class="h-7 w-7 shrink-0 rounded-md text-muted flex-center transition-colors hover:bg-base hover:text-default"
        :title="valueVisible ? '隐藏变量值' : '显示变量值'"
        :aria-label="valueVisible ? '隐藏变量值' : '显示变量值'"
        :disabled="valueLoading"
        @click.stop="emit('toggle-value', row.id)"
      >
        <div
          :class="valueLoading ? 'i-ep-loading animate-spin' : valueVisible ? 'i-ep-view' : 'i-ep-hide'"
          class="text-xs"
        ></div>
      </button>
      <button
        type="button"
        class="h-7 w-7 shrink-0 rounded-md text-muted flex-center transition-colors hover:bg-base hover:text-default"
        title="复制变量值"
        aria-label="复制变量值"
        :disabled="valueLoading"
        @click.stop="emit('copy-value', row.id)"
      >
        <div class="i-ep-copy-document text-xs"></div>
      </button>
    </div>

    <div class="flex items-center gap-2 shrink-0 md:hidden">
      <span class="text-[11px] font-bold text-primary">{{ row.scope === 'Global' ? '全局' : '脚本' }}</span>
      <div class="i-ep-arrow-right text-muted opacity-30 text-xs"></div>
    </div>

    <div class="hidden shrink-0 items-center justify-end gap-1 md:flex">
      <el-switch
        :model-value="row.enabled"
        size="small"
        class="scale-90 shrink-0"
        @change="emit('status-change', row, $event)"
        @click.stop
      />
      <button
        type="button"
        class="h-7 w-7 rounded-md text-primary flex-center transition-colors hover:bg-soft"
        title="编辑变量"
        aria-label="编辑变量"
        @click.stop="emit('edit', row)"
      >
        <div class="i-ep-edit text-xs"></div>
      </button>
      <button
        type="button"
        class="h-7 w-7 rounded-md text-rose-500 flex-center transition-colors hover:bg-rose-500/10"
        title="删除变量"
        aria-label="删除变量"
        @click.stop="emit('delete', row)"
      >
        <div class="i-ep-delete text-xs"></div>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { VariablePageRow } from "../../composables/useVariablePageData";

defineProps<{
  row: VariablePageRow;
  index: number;
  selected: boolean;
  dragOver: boolean;
  searchQuery: string;
  taskNames: string;
  touchDragging: boolean;
  valueLoading: boolean;
  valueVisible: boolean;
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
  (event: "selection-change", row: VariablePageRow): void;
  (event: "status-change", row: VariablePageRow, value: unknown): void;
  (event: "toggle-value", id: number): void;
  (event: "touch-end"): void;
  (event: "touch-move", touchEvent: TouchEvent): void;
  (event: "touch-start", touchEvent: TouchEvent, index: number): void;
}>();
</script>
