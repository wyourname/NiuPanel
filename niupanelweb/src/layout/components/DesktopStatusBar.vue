<template>
  <header class="flex h-12 shrink-0 items-center border-b border-light bg-card px-4 text-[12px] select-none">
    <div class="flex min-w-0 items-center gap-2.5">
      <div class="h-7 w-7 shrink-0 rounded-md bg-primary text-[12px] font-extrabold text-white flex-center">N</div>
      <span class="max-w-[200px] truncate text-[13px] font-bold text-default">{{ systemName }}</span>
      <span class="h-2 w-2 shrink-0 rounded-full bg-emerald-500" title="服务已连接"></span>
    </div>

    <div class="ml-5 hidden items-center gap-1.5 lg:flex">
      <span class="rounded-md px-2.5 py-1.5 text-secondary hover:bg-subtle">
        CPU <strong class="ml-1 font-mono text-default">{{ cpuLabel }}</strong>
      </span>
      <span class="rounded-md px-2.5 py-1.5 text-secondary hover:bg-subtle">
        内存 <strong class="ml-1 font-mono text-default">{{ memoryLabel }}</strong>
      </span>
      <span
        class="rounded-md px-2.5 py-1.5"
        :class="runningJobs > 0 ? 'warning-subtle' : 'text-secondary hover:bg-subtle'"
      >
        作业 <strong class="ml-1 font-mono">{{ runningJobs }}</strong>
      </span>
    </div>

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
        class="flex h-8 min-w-[190px] items-center gap-2 rounded-md border border-light bg-base px-3 text-muted transition-colors hover:border-base hover:text-default"
        @click="emit('open-search')"
      >
        <span class="i-ep-search"></span>
        <span class="flex-1 text-left">搜索</span>
        <kbd class="rounded border border-light bg-card px-1 text-[9px] font-mono">Ctrl K</kbd>
      </button>
      <el-dropdown trigger="click" @command="handleAccountCommand">
        <button
          type="button"
          class="flex h-8 max-w-[170px] items-center gap-2 rounded-md px-2 text-secondary transition-colors hover:bg-subtle hover:text-default"
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
import { getSystemOverview } from "@/api/overview";
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
const cpuUsage = ref<number | null>(null);
const memoryUsed = ref(0);
const memoryTotal = ref(0);
const runningJobs = ref(0);
let refreshTimer: number | undefined;

const cpuLabel = computed(() => cpuUsage.value === null ? "--" : `${cpuUsage.value.toFixed(0)}%`);
const accountInitial = computed(() => (userStore.userInfo.username || "U").slice(0, 1).toUpperCase());
const memoryLabel = computed(() => {
  if (!memoryTotal.value) return "--";
  return `${Math.round((memoryUsed.value / memoryTotal.value) * 100)}%`;
});

const refresh = async () => {
  const [overview, jobs] = await Promise.allSettled([getSystemOverview(), getJobs()]);
  if (overview.status === "fulfilled") {
    cpuUsage.value = overview.value.data.cpu_usage;
    memoryUsed.value = overview.value.data.memory_used;
    memoryTotal.value = overview.value.data.memory_total;
  }
  if (jobs.status === "fulfilled") {
    runningJobs.value = jobs.value.data.filter((job) => job.status === "Running" || job.status === "Pending").length;
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
