<template>
  <el-drawer
    v-model="visibleValue"
    direction="btt"
    size="auto"
    :with-header="false"
    class="action-sheet-drawer"
    append-to-body
  >
    <div class="rounded-t-md bg-card px-4 pb-8 pt-3 text-center">
      <div class="mx-auto mb-4 h-1 w-9 rounded-full bg-muted/30"></div>
      <div class="mb-4 flex items-center gap-3 border-b border-light pb-4 text-left">
        <div
          class="accent-subtle h-10 w-10 rounded-md flex-center"
        >
          <div class="i-ep-plus"></div>
        </div>
        <div class="flex flex-col min-w-0">
          <span class="text-base font-bold text-default truncate">创建新任务</span>
          <span class="text-[10px] font-medium text-muted">
            选择任务创建方式
          </span>
        </div>
      </div>
      <div class="grid gap-2">
        <button
          type="button"
          class="flex min-h-14 items-center gap-3 rounded-md border border-light bg-base px-4 text-left transition-colors hover:bg-soft"
          @click="selectCreateMode('create')"
        >
          <div class="accent-subtle h-8 w-8 rounded-md flex-center">
            <div class="i-ep-plus"></div>
          </div>
          <span class="min-w-0 flex-1">
            <span class="block text-[12px] font-bold text-default">新建任务</span>
            <span class="mt-0.5 block text-[10px] text-muted">配置脚本、定时规则和运行环境</span>
          </span>
          <span class="i-ep-arrow-right text-muted"></span>
        </button>
        <button
          type="button"
          class="flex min-h-14 items-center gap-3 rounded-md border border-light bg-base px-4 text-left transition-colors hover:bg-soft"
          @click="selectCreateMode('quick-create')"
        >
          <div class="h-8 w-8 rounded-md bg-amber-500/10 text-amber-500 flex-center">
            <div class="i-ep-lightning"></div>
          </div>
          <span class="min-w-0 flex-1">
            <span class="block text-[12px] font-bold text-default">从 URL 导入</span>
            <span class="mt-0.5 block text-[10px] text-muted">自动下载并分析公开脚本地址</span>
          </span>
          <span class="i-ep-arrow-right text-muted"></span>
        </button>
      </div>
    </div>
  </el-drawer>
</template>

<script setup lang="ts">
import { computed } from "vue";

type CreateMode = "create" | "quick-create";

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "create"): void;
  (event: "quick-create"): void;
  (event: "update:visible", visible: boolean): void;
}>();

const visibleValue = computed({
  get: () => props.visible,
  set: (value: boolean) => emit("update:visible", value),
});

const selectCreateMode = (mode: CreateMode) => {
  emit("update:visible", false);
  if (mode === "create") emit("create");
  else emit("quick-create");
};
</script>
