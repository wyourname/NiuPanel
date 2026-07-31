<template>
  <header
    class="flex items-center justify-between sticky top-0 z-30 transition-all duration-300 px-4"
    :class="[appStore.isMobile ? 'h-14' : 'h-16']"
  >
    <div class="flex items-center gap-1 sm:gap-3">
      <span
        class="max-w-[150px] truncate select-none font-bold md:max-w-none"
        :class="
          appStore.isMobile ? 'text-[22px] ml-1 text-primary' : 'text-[15px] text-default'
        "
      >
        {{ appStore.isMobile ? "NiuPanel" : pageTitle }}
      </span>
    </div>

    <!-- Right: Actions -->
    <div class="flex items-center gap-1.5 md:gap-3">
      <!-- Desktop Search (Cmd+K) -->
      <div
        v-if="!appStore.isMobile && route.name !== 'tasks'"
        class="hidden md:flex items-center gap-2 px-3 py-1.5 bg-base/50 border border-light rounded-sm cursor-pointer hover:border-primary/50 transition-all group"
        @click="emit('open-search')"
      >
        <div
          class="i-ep-search text-muted group-hover:text-primary transition-colors text-sm"
        ></div>
        <span class="text-xs text-secondary font-medium">Quick Search...</span>
        <span
          class="text-[9px] px-1.5 py-0.5 rounded bg-card border border-light text-muted font-mono"
          >Ctrl+K</span
        >
      </div>

      <!-- Mobile Search Icon -->
      <div v-if="appStore.isDark" class="w-px h-4 bg-dark mx-1"></div>
      <div v-else class="w-px h-4 bg-light mx-1"></div>

      <!-- Theme Toggle -->
      <button class="btn-icon" @click="appStore.toggleDark()">
        <div v-if="appStore.isDark" class="i-ep-sunny h-5 w-5 text-yellow-400"></div>
        <div v-else class="i-ep-moon h-5 w-5 text-slate-500"></div>
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import { useAppStore } from "../../stores/app";

const emit = defineEmits(["open-search"]);
const route = useRoute();
const appStore = useAppStore();

const pageTitle = computed(() => {
  const map: Record<string, string> = {
    overview: "系统概览",
    tasks: "任务列表",
    variables: "环境变量",
    files: "文件管理",
    environments: "环境管理",
    compiler: "代码加密",
    share: "分享中心",
    settings: "系统设置",
    git: "Git 管理",
    telegram: "电报机器人",
    terminal: "系统终端",
    more: "更多",
  };
  return map[route.name as string] || "Dashboard";
});
</script>
