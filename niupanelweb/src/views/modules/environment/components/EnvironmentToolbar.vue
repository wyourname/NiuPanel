<template>
  <div
    class="flex justify-between gap-3"
    :class="isMobile ? 'flex-col border-b border-light bg-soft/10 px-4 py-3 sm:flex-row sm:items-center' : 'min-h-8 items-center'"
  >
    <div class="flex items-center gap-4">
      <SegmentedTabs
        v-model="filterValue"
        :items="filterItems"
        :full-width="isMobile"
        class="flex-1 sm:flex-none"
      />
      <div
        v-if="loading"
        class="i-ep-loading animate-spin text-primary text-xs"
      ></div>
    </div>

    <div class="flex items-center justify-end gap-2">
      <ToolbarButton size="small" @click="emit('open-jobs')">
        <template #icon>
          <div class="i-ep-list"></div>
        </template>
        <span v-if="!isMobile">运行记录</span>
      </ToolbarButton>

      <ToolbarButton size="small" @click="emit('open-mirror')">
        <template #icon>
          <div class="i-ep-setting"></div>
        </template>
        <span v-if="!isMobile">镜像配置</span>
      </ToolbarButton>

      <ToolbarButton
        variant="primary"
        size="small"
        :disabled="filterType === 'sh'"
        class="flex-1 sm:flex-none"
        @click="emit('create')"
      >
        <template #icon>
          <div class="i-ep-plus"></div>
        </template>
        {{ createButtonText }}
      </ToolbarButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import SegmentedTabs from "../../../../components/common/SegmentedTabs.vue";
import ToolbarButton from "../../../../components/common/ToolbarButton.vue";
import type { EnvType } from "@/types";

const filterItems: Array<{ label: string; value: EnvType }> = [
  { label: "Python", value: "python" },
  { label: "Node.js", value: "node" },
  { label: "Linux", value: "sh" },
];

const props = defineProps<{
  filterType: EnvType;
  isMobile: boolean;
  loading: boolean;
}>();

const emit = defineEmits<{
  (event: "create"): void;
  (event: "open-jobs"): void;
  (event: "open-mirror"): void;
  (event: "update:filterType", value: EnvType): void;
}>();

const filterValue = computed({
  get: () => props.filterType,
  set: (value) => emit("update:filterType", value),
});

const createButtonText = computed(() => {
  if (props.filterType === "node") {
    return props.isMobile ? "安装" : "安装版本";
  }
  return props.isMobile ? "添加" : "添加环境";
});
</script>
