<template>
  <header class="desktop-status-bar flex h-12 shrink-0 items-center px-4 text-[12px] select-none">
    <div class="flex min-w-0 items-center gap-2.5">
      <div class="status-brand-mark h-7 w-7 shrink-0 rounded-md text-[12px] font-extrabold flex-center">N</div>
      <span class="max-w-[200px] truncate text-[13px] font-bold text-default">{{ systemName }}</span>
      <span class="status-connection" :class="{ 'is-connected': metrics }" :title="metrics ? '监控指标已更新' : '监控指标暂不可用'" :aria-label="metrics ? '监控指标已更新' : '监控指标暂不可用'"></span>
    </div>

    <div class="status-metrics ml-4 hidden shrink-0 items-center gap-1 xl:flex">
      <span class="status-metric">
        CPU <strong class="ml-1 font-mono text-default">{{ cpuLabel }}</strong>
      </span>
      <span class="status-metric">
        内存 <strong class="ml-1 font-mono text-default">{{ memoryLabel }}</strong>
      </span>
      <span
        class="status-metric"
        :class="runningJobs > 0 ? 'status-metric--active' : ''"
      >
        作业 <strong class="ml-1 font-mono">{{ runningJobs }}</strong>
      </span>
    </div>

    <NetworkRateDisplay
      class="ml-3 shrink-0"
      :upload="metrics?.network_upload_speed"
      :download="metrics?.network_download_speed"
    />

    <div class="ml-auto flex items-center gap-1.5">
      <button
        v-if="workspace.windows.length"
        type="button"
        class="h-8 rounded-md px-2.5 text-secondary transition-colors hover:bg-subtle hover:text-default"
        @click="workspace.focusNextWindow()"
      >
        窗口 <strong class="ml-1 font-mono">{{ workspace.windows.length }}</strong>
      </button>
      <button
        type="button"
        class="status-search flex h-8 items-center gap-2 rounded-md px-3 transition-colors xl:min-w-[160px]"
        aria-label="搜索"
        @click="emit('open-search')"
      >
        <span class="i-ep-search"></span>
        <span class="hidden flex-1 text-left xl:inline">搜索</span>
        <kbd class="hidden rounded border border-light px-1 text-[9px] font-mono xl:inline">Ctrl K</kbd>
      </button>
      <el-dropdown trigger="click" @command="handleAccountCommand">
        <button
          type="button"
          aria-label="账户菜单"
          class="status-account flex h-8 max-w-[170px] items-center gap-2 rounded-md px-2 transition-colors"
        >
          <span class="h-6 w-6 shrink-0 rounded-full accent-subtle flex-center text-[10px] font-bold">
            {{ accountInitial }}
          </span>
          <span class="truncate font-semibold">{{ userStore.userInfo.username || '用户' }}</span>
          <span class="i-ep-arrow-down shrink-0 text-[10px] text-muted"></span>
        </button>
        <template #dropdown>
          <el-dropdown-menu class="modern-dropdown w-48">
            <el-dropdown-item command="profile">
              <span class="i-ep-user mr-2"></span>个人资料
            </el-dropdown-item>
            <el-dropdown-item command="keys">
              <span class="i-ep-key mr-2"></span>API 访问
            </el-dropdown-item>
            <el-dropdown-item command="theme" divided>
              <span :class="appStore.isDark ? 'i-ep-sunny' : 'i-ep-moon'" class="mr-2"></span>
              {{ appStore.isDark ? "浅色模式" : "深色模式" }}
            </el-dropdown-item>
            <el-dropdown-item command="settings">
              <span class="i-ep-setting mr-2"></span>系统设置
            </el-dropdown-item>
            <el-dropdown-item command="logout" divided>
              <span class="i-ep-switch-button mr-2 text-rose-500"></span>
              <span class="text-rose-600 dark:text-rose-400">退出登录</span>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { ElMessageBox } from "element-plus";
import { useRouter } from "vue-router";
import { getJobs } from "@/api/jobs";
import { useSystemMetrics } from "@/composables/useSystemMetrics";
import NetworkRateDisplay from "@/components/common/NetworkRateDisplay.vue";
import { useSystemSettings } from "@/composables/useSystemSettings";
import { useAppStore } from "@/stores/app";
import { useUserStore } from "@/stores/user";
import { useWorkspaceStore } from "@/stores/workspace";

const emit = defineEmits<{ (event: "open-search"): void }>();
const appStore = useAppStore();
const userStore = useUserStore();
const workspace = useWorkspaceStore();
const router = useRouter();
const { systemName } = useSystemSettings();
const { metrics } = useSystemMetrics();
const runningJobs = ref(0);
let refreshTimer: number | undefined;

const cpuLabel = computed(() => metrics.value === null ? "--" : `${metrics.value.cpu_usage.toFixed(0)}%`);
const accountInitial = computed(() => (userStore.userInfo.username || "U").slice(0, 1).toUpperCase());
const memoryLabel = computed(() => {
  if (!metrics.value?.memory_total) return "--";
  return `${Math.round((metrics.value.memory_used / metrics.value.memory_total) * 100)}%`;
});

const refresh = async () => {
  try {
    const jobs = await getJobs();
    runningJobs.value = jobs.data.filter((job) => job.status === "Running" || job.status === "Pending").length;
  } catch {
    // The request interceptor reports job refresh failures.
  }
};

const openSettingsSection = async (section: string) => {
  await router.push({ name: "settings", query: { section } });
  workspace.openAppWindow("settings");
};

const handleAccountCommand = async (command: string) => {
  if (command === "profile") await openSettingsSection("security");
  else if (command === "keys") await openSettingsSection("keys");
  else if (command === "settings") await openSettingsSection("basic");
  else if (command === "theme") appStore.toggleDark();
  else if (command === "logout") {
    await ElMessageBox.confirm("确认退出当前账户？", "退出登录", {
      type: "warning",
      confirmButtonText: "退出",
      cancelButtonText: "取消",
    });
    await userStore.logout();
    await router.replace({ name: "login" });
  }
};

onMounted(() => {
  void refresh();
  refreshTimer = window.setInterval(refresh, 30_000);
});

onUnmounted(() => {
  if (refreshTimer) window.clearInterval(refreshTimer);
});
</script>

<style scoped>
.desktop-status-bar { position: relative; z-index: 31; background: var(--bg-card); color: var(--text-default); border-bottom: 1px solid var(--border-light); }
.status-brand-mark { color: var(--button-primary-text); background: var(--button-primary-bg); box-shadow: var(--shadow-sm); }
.status-connection { width: 6px; height: 6px; border-radius: 50%; background: var(--text-muted); }
.status-connection.is-connected { background: var(--success-subtle-text); }
.status-metrics { padding-left: 12px; border-left: 1px solid var(--border-light); }
.status-metric { padding: 5px 10px; border-radius: 5px; color: var(--text-muted); font-size: 11px; }
.status-metric--active { color: var(--warning-subtle-text); background: var(--warning-subtle-bg); }
.status-search { color: var(--text-muted); border: 1px solid var(--border-light); background: var(--bg-subtle); }
.status-search:hover { color: var(--text-default); border-color: var(--border-base); background: var(--bg-soft); }
.status-account { color: var(--text-default); }
.status-account:hover { background: var(--bg-subtle); }
</style>
