<template>
  <header
    class="panel-header sticky top-0 z-30 flex shrink-0 items-center justify-between px-3 transition-colors duration-200 pt-safe md:px-4"
    :class="[appStore.isMobile ? 'safe-top-header' : 'h-16']"
  >
    <div class="flex min-w-0 flex-1 items-center gap-2.5 sm:gap-3">
      <span class="panel-header-mark" aria-hidden="true">N</span>
      <span
        class="min-w-0 max-w-[min(52vw,240px)] truncate select-none font-bold md:max-w-none"
        :class="
          appStore.isMobile ? 'text-[16px] text-default' : 'text-[15px] text-default'
        "
      >
        {{ pageTitle }}
      </span>
    </div>

    <!-- Right: Actions -->
    <div class="flex shrink-0 items-center gap-1 md:gap-3">
      <NetworkRateDisplay
        v-if="appStore.isMobile"
        compact
        class="mr-1 shrink-0"
        :upload="metrics?.network_upload_speed"
        :download="metrics?.network_download_speed"
      />
      <!-- Desktop Search (Cmd+K) -->
      <button
        type="button"
        aria-label="打开全局搜索"
        v-if="!appStore.isMobile && route.name !== 'tasks'"
        class="hidden md:flex items-center gap-2 px-3 py-1.5 bg-base/50 border border-light rounded-sm cursor-pointer hover:border-primary/50 transition-all group"
        @click="emit('open-search')"
      >
        <div
          class="i-ep-search text-muted group-hover:text-primary transition-colors text-sm"
        ></div>
        <span class="text-xs text-secondary font-medium">搜索面板</span>
        <span
          class="text-[9px] px-1.5 py-0.5 rounded bg-card border border-light text-muted font-mono"
          >Ctrl+K</span
        >
      </button>

      <button
        v-if="appStore.isMobile"
        type="button"
        class="btn-icon"
        title="搜索"
        aria-label="打开全局搜索"
        @click="emit('open-search')"
      >
        <div class="i-ep-search h-5 w-5"></div>
      </button>

      <div class="mx-0.5 h-5 w-px bg-light"></div>

      <!-- Theme Toggle -->
      <button
        type="button"
        class="btn-icon"
        :title="appStore.isDark ? '切换到浅色模式' : '切换到深色模式'"
        :aria-label="appStore.isDark ? '切换到浅色模式' : '切换到深色模式'"
        @click="appStore.toggleDark()"
      >
        <div v-if="appStore.isDark" class="i-ep-sunny h-5 w-5 text-yellow-400"></div>
        <div v-else class="i-ep-moon h-5 w-5 text-slate-500"></div>
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed } from "vue";
import NetworkRateDisplay from "@/components/common/NetworkRateDisplay.vue";
import { useSystemMetrics } from "@/composables/useSystemMetrics";
import { useRoute } from "vue-router";
import { useAppStore } from "../../stores/app";
import { usePluginAppsStore, primaryPluginRoute } from "../../stores/pluginApps";

const emit = defineEmits(["open-search"]);
const route = useRoute();
const appStore = useAppStore();
const { metrics } = useSystemMetrics();
const pluginApps = usePluginAppsStore();

const pageTitle = computed(() => {
  if (route.name === "plugin-app") {
    const rawPluginId = route.params.pluginId;
    const pluginId = Array.isArray(rawPluginId) ? rawPluginId[0] : rawPluginId;
    const app = pluginId ? pluginApps.getApp(pluginId) : null;
    return app ? primaryPluginRoute(app)?.title || app.name : "插件应用";
  }

  return typeof route.meta.title === "string" ? route.meta.title : "NiuPanel";
});
</script>

<style scoped>
.panel-header { background: var(--bg-card); border-bottom: 1px solid var(--border-light); box-shadow: 0 1px 3px rgb(20 38 66 / 3%); }
.panel-header-mark { display: grid; place-items: center; width: 30px; height: 30px; flex-shrink: 0; border-radius: 8px; background: var(--button-primary-bg); color: var(--button-primary-text); font-size: 15px; font-weight: 800; box-shadow: 0 1px 0 rgb(255 255 255 / 20%) inset; }
</style>
