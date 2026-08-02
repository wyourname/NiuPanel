<template>
  <section class="flex h-full min-h-0 flex-col bg-card">
    <!-- 工具栏:作用域切换 + 搜索 + 操作,收成一行 -->
    <header class="flex h-[52px] shrink-0 items-center gap-3 border-b border-light px-4">
      <div class="segmented-tabs shrink-0">
        <button
          type="button"
          class="segmented-tabs__item"
          :class="activeTab === 'Global' ? 'is-active' : ''"
          @click="emit('select-scope', 'Global')"
        >
          <span class="i-ep-coin text-[13px]"></span>
          <span class="segmented-tabs__label">全局变量</span>
        </button>
        <button
          type="button"
          class="segmented-tabs__item"
          :class="activeTab === 'Script' ? 'is-active' : ''"
          @click="emit('select-scope', 'Script')"
        >
          <span class="i-ep-document text-[13px]"></span>
          <span class="segmented-tabs__label">脚本变量</span>
        </button>
      </div>

      <div class="group relative w-full max-w-[320px]">
        <span
          class="i-ep-search pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[14px] text-muted transition-colors duration-200 group-focus-within:text-primary"
        ></span>
        <input
          :value="searchQuery"
          type="text"
          inputmode="search"
          autocomplete="off"
          aria-label="搜索变量名或备注"
          class="variable-search-input h-9 w-full rounded-md pl-9 pr-9 text-[13px] text-default outline-none"
          placeholder="搜索变量名或备注"
          @input="emit('search-input', ($event.target as HTMLInputElement).value)"
        />
        <button
          v-if="searchQuery"
          type="button"
          class="absolute right-2 top-1/2 h-6 w-6 -translate-y-1/2 cursor-pointer rounded-md text-muted flex-center transition-colors duration-200 hover:bg-active hover:text-default focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/30"
          title="清空"
          aria-label="清空搜索内容"
          @click="emit('search-input', '')"
        >
          <span class="i-ep-close text-[12px]"></span>
        </button>
      </div>

      <div class="ml-auto flex shrink-0 items-center gap-2">
        <button
          type="button"
          class="h-9 rounded-md border border-light bg-card px-3 text-[12px] font-semibold text-secondary transition-colors hover:bg-soft hover:text-default"
          @click="emit('import')"
        >
          <span class="i-ep-upload mr-1.5"></span>导入
        </button>
        <button
          type="button"
          class="h-9 rounded-md border border-light bg-card px-3 text-[12px] font-semibold text-secondary transition-colors hover:bg-soft hover:text-default"
          @click="emit('export')"
        >
          <span class="i-ep-download mr-1.5"></span>导出
        </button>
        <button
          type="button"
          class="h-9 rounded-md accent-subtle px-3.5 text-[12px] font-bold transition-colors hover:brightness-95"
          @click="emit('create')"
        >
          <span class="i-ep-plus mr-1.5"></span>新增变量
        </button>
      </div>
    </header>

    <!-- 表头 -->
    <div
      class="grid shrink-0 items-center gap-3 border-b border-light bg-subtle px-4 py-2 text-[11px] font-semibold text-muted"
      :style="gridStyle"
    >
      <span></span>
      <span>变量名</span>
      <span>值</span>
      <span>作用域</span>
      <span>备注</span>
      <span class="text-center">启用</span>
      <span class="text-right">操作</span>
    </div>

    <!-- 表体 -->
    <div
      class="min-h-0 flex-1 overflow-y-auto custom-scrollbar"
      @scroll.passive="handleScroll"
    >
      <div v-if="loading && !variables.length" class="flex flex-col">
        <div v-for="index in 8" :key="index" class="grid items-center gap-3 border-b border-light px-4 py-3" :style="gridStyle">
          <span></span>
          <div class="h-3 w-2/3 rounded bg-subtle"></div>
          <div class="h-3 w-4/5 rounded bg-subtle"></div>
          <div class="h-3 w-10 rounded bg-subtle"></div>
          <div class="h-3 w-1/2 rounded bg-subtle"></div>
          <div class="mx-auto h-3 w-8 rounded bg-subtle"></div>
          <div class="ml-auto h-3 w-12 rounded bg-subtle"></div>
        </div>
      </div>

      <div v-else-if="!variables.length" class="flex h-full min-h-[300px] flex-col items-center justify-center text-center">
        <span class="i-ep-document-copy text-[34px] text-muted opacity-50"></span>
        <p class="mt-3 text-[14px] font-semibold text-default">{{ searchQuery ? "没有匹配的变量" : "还没有变量" }}</p>
        <p class="mt-1 text-[12px] text-muted">{{ searchQuery ? "换个关键词试试" : "点击右上角新增第一个变量" }}</p>
        <button
          v-if="!searchQuery"
          type="button"
          class="mt-4 h-9 rounded-md accent-subtle px-4 text-[12px] font-bold"
          @click="emit('create')"
        >
          <span class="i-ep-plus mr-1.5"></span>新增变量
        </button>
      </div>

      <template v-else>
        <div
          v-for="row in variables"
          :key="row.id"
          class="group grid items-center gap-3 border-b border-light px-4 py-2.5 transition-colors hover:bg-soft"
          :style="gridStyle"
        >
          <el-checkbox
            :model-value="selectedIds.includes(row.id)"
            class="!mr-0"
            @change="emit('selection-change', row, $event)"
            @click.stop
          />

          <div class="min-w-0">
            <div class="truncate font-mono text-[13px] font-semibold text-default" :title="row.key">{{ row.key }}</div>
            <div class="font-mono text-[10px] text-muted opacity-70">#{{ row.id }}</div>
          </div>

          <div class="flex min-w-0 items-center gap-1.5">
            <code
              class="min-w-0 flex-1 truncate rounded bg-subtle px-2 py-1 font-mono text-[12px] text-secondary"
              :class="isValueVisible(row.id) ? '' : 'select-none tracking-[0.2em]'"
              :title="isValueVisible(row.id) ? row.value ?? '' : ''"
            >{{ isValueVisible(row.id) ? row.value ?? "" : maskValue(row.value) }}</code>
            <button
              type="button"
              class="h-7 w-7 shrink-0 rounded text-muted flex-center transition-colors hover:bg-active hover:text-default"
              :title="isValueVisible(row.id) ? '隐藏' : '显示'"
              :disabled="isValueLoading(row.id)"
              @click="emit('toggle-value', row.id)"
            >
              <span
                :class="isValueLoading(row.id) ? 'i-ep-loading animate-spin' : isValueVisible(row.id) ? 'i-ep-hide' : 'i-ep-view'"
                class="text-[13px]"
              ></span>
            </button>
            <button
              type="button"
              class="h-7 w-7 shrink-0 rounded text-muted flex-center opacity-0 transition-colors hover:bg-active hover:text-primary group-hover:opacity-100"
              title="复制值"
              aria-label="复制变量值"
              :disabled="isValueLoading(row.id)"
              @click="emit('copy-value', row.id)"
            >
              <span class="i-ep-copy-document text-[13px]"></span>
            </button>
          </div>

          <span class="rounded bg-subtle px-1.5 py-0.5 text-[10px] font-semibold text-secondary">{{ row.scope === "Global" ? "全局" : "脚本" }}</span>

          <span class="truncate text-[12px] text-muted" :title="row.remarks || ''">{{ row.remarks || "—" }}</span>

          <div class="flex justify-center">
            <el-switch
              :model-value="row.enabled"
              size="small"
              @change="emit('status-change', row, $event)"
            />
          </div>

          <div class="flex justify-end gap-0.5">
            <button
              type="button"
              class="h-7 w-7 rounded text-muted flex-center transition-colors hover:bg-active hover:text-default"
              title="编辑"
              @click="emit('edit', row)"
            >
              <span class="i-ep-edit text-[13px]"></span>
            </button>
            <button
              type="button"
              class="h-7 w-7 rounded text-muted flex-center transition-colors hover:bg-active hover:text-rose-500"
              title="删除"
              @click="emit('delete', row)"
            >
              <span class="i-ep-delete text-[13px]"></span>
            </button>
          </div>
        </div>

        <div v-if="loading && variables.length" class="py-3 text-center">
          <span class="i-ep-loading inline-block animate-spin text-primary"></span>
        </div>
        <div v-if="!hasMore && variables.length" class="py-3 text-center text-[11px] text-muted">
          共 {{ variables.length }} 个变量
        </div>
      </template>
    </div>
  </section>
</template>

<script setup lang="ts">
import type { VariablePageRow } from "../../composables/useVariablePageData";

const props = defineProps<{
  activeTab: string;
  searchQuery: string;
  variables: VariablePageRow[];
  selectedIds: number[];
  loading: boolean;
  hasMore: boolean;
  isValueLoading: (id: number) => boolean;
  isValueVisible: (id: number) => boolean;
}>();

const emit = defineEmits<{
  (e: "select-scope", scope: "Global" | "Script"): void;
  (e: "search-input", value: string): void;
  (e: "create"): void;
  (e: "import"): void;
  (e: "export"): void;
  (e: "load-more"): void;
  (e: "selection-change", row: VariablePageRow, selected: unknown): void;
  (e: "toggle-value", id: number): void;
  (e: "copy-value", id: number): void;
  (e: "status-change", row: VariablePageRow, value: unknown): void;
  (e: "edit", row: VariablePageRow): void;
  (e: "delete", row: VariablePageRow): void;
}>();

const handleScroll = (event: Event) => {
  if (props.loading || !props.hasMore) return;

  const target = event.currentTarget as HTMLElement;
  const remaining = target.scrollHeight - target.scrollTop - target.clientHeight;
  if (remaining <= 40) emit("load-more");
};

const gridStyle =
  "grid-template-columns: 28px minmax(140px, 1.4fr) minmax(160px, 2fr) 60px minmax(100px, 1.2fr) 56px 72px;";

const maskValue = (value?: string) =>
  "•".repeat(Math.min(Math.max(value?.length ?? 12, 6), 24));
</script>

<style scoped>
.variable-search-input {
  -webkit-appearance: none;
  appearance: none;
  border: 1px solid var(--border-base);
  background: color-mix(in srgb, var(--bg-base) 72%, var(--bg-card));
  box-shadow: none;
  transition:
    border-color 0.2s ease,
    background-color 0.2s ease,
    box-shadow 0.2s ease;
}

.variable-search-input:hover {
  border-color: color-mix(in srgb, var(--border-base) 72%, var(--text-secondary));
  background: var(--bg-base);
}

.variable-search-input:focus {
  border-color: rgb(var(--brand-primary-rgb) / 0.68);
  background: var(--bg-card);
  box-shadow: 0 0 0 3px rgb(var(--brand-primary-rgb) / 0.12);
}

.variable-search-input::placeholder {
  color: var(--text-muted);
}
</style>
