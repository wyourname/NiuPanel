<template>
  <OverlayDrawer
    v-model:visible="visibleValue"
    title="创建新任务"
    variant="sheet"
    content-preset="list"
    append-to-body
  >
    <div class="grid gap-2">
        <button
          type="button"
          class="flex min-h-14 cursor-pointer items-center gap-3 rounded-md border border-light bg-base px-3 text-left transition-colors hover:bg-soft focus-visible:outline-2 focus-visible:outline-primary"
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
          class="flex min-h-14 cursor-pointer items-center gap-3 rounded-md border border-light bg-base px-3 text-left transition-colors hover:bg-soft focus-visible:outline-2 focus-visible:outline-primary"
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
  </OverlayDrawer>
</template>

<script setup lang="ts">
import { computed } from "vue";
import OverlayDrawer from "../common/OverlayDrawer.vue";

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
