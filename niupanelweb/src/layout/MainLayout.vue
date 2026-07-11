<template>
  <div class="h-screen w-screen flex overflow-hidden bg-base font-sans antialiased">
    <div class="flex-1 flex flex-col min-w-0 overflow-hidden relative bg-card">
      <TheHeader
        v-if="appStore.isMobile"
        class="glass-header"
        @open-search="searchVisible = true"
      />
      <DesktopStatusBar v-else @open-search="searchVisible = true" />

      <main
        class="flex-1 overflow-hidden relative"
        :class="appStore.isMobile ? 'bg-base' : 'bg-card'"
      >
        <DesktopWorkspaceHome v-if="!appStore.isMobile" />
        <router-view v-else v-slot="{ Component }">
          <transition :name="appStore.isMobile ? transitionName : 'fade-slide'" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </main>

      <WorkspaceLayer />
      <TheSidebar />
    </div>

    <!-- Global Command Palette -->
    <CommandPalette v-model:visible="searchVisible" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { useRouter, useRoute } from "vue-router";
import { App } from "@capacitor/app";
import { StatusBar, Style } from "@capacitor/status-bar";
import { Keyboard } from "@capacitor/keyboard";
import { useAppStore } from "../stores/app";
import { useWorkspaceStore } from "../stores/workspace";
import { usePluginThemesStore } from "../stores/pluginThemes";
import TheSidebar from "./components/TheSidebar.vue";
import TheHeader from "./components/TheHeader.vue";
import DesktopStatusBar from "./components/DesktopStatusBar.vue";
import CommandPalette from "../components/common/CommandPalette.vue";
import DesktopWorkspaceHome from "../components/workspace/DesktopWorkspaceHome.vue";
import WorkspaceLayer from "../components/workspace/WorkspaceLayer.vue";
import { workspaceApps } from "../workspace/apps";

const appStore = useAppStore();
const workspaceStore = useWorkspaceStore();
const pluginThemes = usePluginThemesStore();
const router = useRouter();
const route = useRoute();
const searchVisible = ref(false);
const transitionName = ref("fade-slide");

type AppListenerHandle = {
  remove: () => Promise<void>;
};

let backButtonListener: AppListenerHandle | null = null;
let stopStatusBarThemeSync: (() => void) | null = null;

const setupCapacitor = async () => {
  if (!appStore.isMobile) return;

  try {
    backButtonListener = await App.addListener(
      "backButton",
      async ({ canGoBack }) => {
        // 1. 优先处理 Store 中的返回栈（如关闭日志、弹窗等）
        if (await appStore.handleBack()) {
          return;
        }

        // 2. 如果路由可以后退
        if (canGoBack) {
          router.back();
        } else {
          // 3. 退出 App
          App.exitApp();
        }
      },
    );

    // 禁用沉浸式遮盖，让 WebView 从状态栏下方开始，解决“大额头”和刘海屏兼容性
    await StatusBar.setOverlaysWebView({ overlay: false });

    stopStatusBarThemeSync = watch(
      () => appStore.isDark,
      (isDark) => {
        StatusBar.setStyle({ style: isDark ? Style.Dark : Style.Light });
        // 动态同步状态栏颜色与主题色一致
        StatusBar.setBackgroundColor({ color: isDark ? "#17212b" : "#ffffff" });
      },
      { immediate: true }
    );

    Keyboard.setAccessoryBarVisible({ isVisible: true });
  } catch (e) {}
};

const isEditableTarget = (target: EventTarget | null) => {
  if (!(target instanceof HTMLElement)) return false;

  return !!target.closest(
    "input, textarea, [contenteditable='true'], .monaco-editor, .xterm",
  );
};

const handleKeydown = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key === "k") {
    e.preventDefault();
    searchVisible.value = true;
    return;
  }

  if (appStore.isMobile || isEditableTarget(e.target)) return;

  const hasMod = e.metaKey || e.ctrlKey;
  const activeWindowId = workspaceStore.activeWindowId;

  if (hasMod && e.key === "Tab") {
    e.preventDefault();
    workspaceStore.focusNextWindow();
  } else if (hasMod && e.key.toLowerCase() === "w" && activeWindowId) {
    e.preventDefault();
    workspaceStore.closeActiveWindow();
  } else if (hasMod && e.shiftKey && e.key.toLowerCase() === "m") {
    e.preventDefault();
    workspaceStore.minimizeActiveWindow();
  } else if (hasMod && e.altKey && e.key === "ArrowLeft" && activeWindowId) {
    e.preventDefault();
    workspaceStore.placeWindow(activeWindowId, "left");
  } else if (hasMod && e.altKey && e.key === "ArrowRight" && activeWindowId) {
    e.preventDefault();
    workspaceStore.placeWindow(activeWindowId, "right");
  }
};

const openRouteAsDesktopWindow = () => {
  if (appStore.isMobile) return;

  const routeName = route.name as string | undefined;
  if (!routeName || routeName === "tasks") return;

  const app = workspaceApps.find((item) => item.routeName === routeName);
  if (!app || app.id === "task-log") return;

  workspaceStore.openAppWindow(app.id);
};

onMounted(() => {
  window.addEventListener("keydown", handleKeydown);
  setupCapacitor();
  openRouteAsDesktopWindow();
  void pluginThemes.loadThemes().catch(() => undefined);
});

watch(
  () => [route.name, appStore.isMobile] as const,
  openRouteAsDesktopWindow,
);

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeydown);
  backButtonListener?.remove();
  backButtonListener = null;
  stopStatusBarThemeSync?.();
  stopStatusBarThemeSync = null;
});
</script>
