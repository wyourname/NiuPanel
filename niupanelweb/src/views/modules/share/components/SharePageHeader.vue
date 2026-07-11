<template>
  <div class="flex items-center justify-between shrink-0 z-20 w-full transition-all">
    <div
      class="flex gap-1 rounded-md border border-light/50 bg-soft p-1"
      :class="isMobile ? 'min-w-0 flex-1' : 'w-[390px]'"
    >
      <button
        v-for="tab in tabs"
        :key="tab.value"
        type="button"
        class="flex min-w-0 flex-1 cursor-pointer select-none items-center justify-center gap-1.5 rounded py-1.5 text-[11px] font-bold transition-colors duration-200"
        :class="
          activeTab === tab.value
            ? 'bg-white dark:bg-card text-primary shadow-sm ring-1 ring-black/5'
            : 'text-muted hover:text-default hover:bg-black/5 dark:hover:bg-white/5'
        "
        @click="emit('update:activeTab', tab.value)"
      >
        <span :class="tab.icon" class="shrink-0 text-[13px]"></span>
        <span class="truncate">{{ tab.label }}</span>
      </button>
    </div>

    <div class="ml-2 flex shrink-0 items-center gap-2">
      <button
        v-if="activeTab === 'market'"
        type="button"
        class="h-8 rounded-md border border-light bg-base px-3 text-xs font-bold text-default flex-center gap-1.5 transition-colors hover:bg-soft"
        title="订阅源管理"
        aria-label="订阅源管理"
        @click="emit('open-sources')"
      >
        <div class="i-ep-connection text-primary"></div>
        <span v-if="!isMobile">订阅源管理</span>
      </button>
      <button
        v-if="activeTab === 'import'"
        type="button"
        class="h-8 rounded-md bg-primary px-3 text-xs font-bold text-white flex-center gap-1.5 transition-colors hover:bg-primary/90"
        title="导入资源"
        aria-label="导入资源"
        @click="emit('open-import')"
      >
        <div class="i-ep-plus"></div>
        <span v-if="!isMobile">导入资源</span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
type ShareTab = "market" | "import" | "manage";

defineProps<{
  activeTab: ShareTab;
  isMobile: boolean;
}>();

const emit = defineEmits<{
  (event: "open-import"): void;
  (event: "open-sources"): void;
  (event: "update:activeTab", tab: ShareTab): void;
}>();

const tabs: Array<{ label: string; value: ShareTab; icon: string }> = [
  { label: "脚本市场", value: "market", icon: "i-carbon-store" },
  { label: "导入记录", value: "import", icon: "i-ep-download" },
  { label: "云端资源", value: "manage", icon: "i-carbon-cloud-service-management" },
];
</script>
