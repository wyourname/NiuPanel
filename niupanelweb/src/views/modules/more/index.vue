<template>
  <div class="flex flex-col h-full bg-base">
    <div class="shrink-0 border-b border-light bg-card px-5 py-4">
      <div class="flex flex-col items-center">
        <div
          class="mb-2.5 h-12 w-12 rounded-lg bg-primary text-lg font-bold text-white flex-center"
        >
          {{ avatarText }}
        </div>
        <h2 class="mb-1 text-[16px] font-bold text-default">{{ username }}</h2>
        <div
          class="rounded-md border border-light bg-subtle px-2 py-0.5 text-[11px] font-medium text-muted"
        >
          {{ userRoleLabel }}
        </div>
      </div>
    </div>

    <div class="mobile-dock-safe flex-1 space-y-3 overflow-y-auto p-4 custom-scrollbar">
      <div
        class="overflow-hidden rounded-lg border border-light bg-card divide-y divide-light"
      >
        <div
          v-for="item in mainMenuItems"
          :key="item.path"
          class="group flex cursor-pointer items-center justify-between px-3 py-3 transition-colors hover:bg-subtle active:bg-soft"
          @click="navigate(item)"
        >
          <div class="flex items-center gap-3">
            <div
              class="h-9 w-9 shrink-0 rounded-md text-[16px] flex-center"
              :class="item.colorClass"
            >
              <div :class="item.icon"></div>
            </div>
            <div class="flex flex-col">
              <span class="text-[13px] font-semibold text-default">{{
                item.label
              }}</span>
              <span class="mt-0.5 text-[11px] text-muted">{{ item.desc }}</span>
            </div>
          </div>
          <div
            class="i-ep-arrow-right text-muted opacity-30 group-hover:opacity-60 transition-opacity text-sm"
          ></div>
        </div>
      </div>

      <div
        v-if="subMenuItems.length > 0"
        class="overflow-hidden rounded-lg border border-light bg-card divide-y divide-light"
      >
        <div
          v-for="item in subMenuItems"
          :key="item.path"
          class="group flex cursor-pointer items-center justify-between px-3 py-3 transition-colors hover:bg-subtle active:bg-soft"
          @click="navigate(item)"
        >
          <div class="flex items-center gap-3">
            <div
              class="h-9 w-9 shrink-0 rounded-md text-[16px] flex-center"
              :class="item.colorClass"
            >
              <div :class="item.icon"></div>
            </div>
            <div class="flex flex-col">
              <span class="text-[13px] font-semibold text-default">{{ item.label }}</span>
              <span class="mt-0.5 text-[11px] text-muted">{{ item.desc }}</span>
            </div>
          </div>
          <div
            class="i-ep-arrow-right text-muted opacity-30 group-hover:opacity-60 transition-opacity text-sm"
          ></div>
        </div>
      </div>

      <div class="py-2 text-center text-[11px] text-muted">
        {{ versionLabel }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter, type LocationQueryRaw } from "vue-router";
import { useUserStore } from "../../../stores/user";
import { hasPermission } from "@/utils/permission";
import { getVersion } from "@/api/settings";
import { FRONTEND_VERSION, formatVersion } from "@/version";

const router = useRouter();
const userStore = useUserStore();

const username = computed(() => userStore.userInfo.username || "Admin");
const systemVersion = ref("");
const userRoleLabel = computed(() => {
  const role = userStore.userInfo.role || "user";
  return role === "admin" ? "管理员" : "用户";
});
const avatarText = computed(() => username.value.charAt(0).toUpperCase());

type MoreMenuItem = {
  label: string;
  desc: string;
  path: string;
  name: string;
  perm: string;
  icon: string;
  colorClass: string;
  query?: LocationQueryRaw;
};

const allMenuItems: MoreMenuItem[] = [
  {
    label: "环境管理",
    desc: "Python / Node / Shell",
    path: "/environments",
    name: "environments",
    perm: "env:read",
    icon: "i-carbon-container-services",
    colorClass: "bg-blue-500/10 text-blue-500",
  },
  {
    label: "扩展中心",
    desc: "插件安装与应用管理",
    path: "/extensions",
    name: "extensions",
    perm: "setting:read",
    icon: "i-carbon-plug",
    colorClass: "bg-emerald-500/10 text-emerald-500",
  },
  {
    label: "Git 仓库",
    desc: "自动化同步脚本库",
    path: "/git",
    name: "git",
    perm: "git:read",
    icon: "i-ep-refresh",
    colorClass: "bg-indigo-500/10 text-indigo-500",
  },
  {
    label: "电报机器人",
    desc: "TG 消息推送配置",
    path: "/telegram",
    name: "telegram",
    perm: "setting:read",
    icon: "i-ep-promotion",
    colorClass: "bg-sky-500/10 text-sky-500",
  },
  {
    label: "系统终端",
    desc: "Shell 访问",
    path: "/terminal",
    name: "terminal",
    perm: "terminal:access",
    icon: "i-carbon-terminal",
    colorClass: "bg-emerald-500/10 text-emerald-500",
  },
  {
    label: "系统设置",
    desc: "通知、安全与维护",
    path: "/settings",
    name: "settings",
    perm: "setting:read",
    icon: "i-ep-setting",
    colorClass: "bg-slate-500/10 text-slate-500",
  },
];

const visibleMenuItems = computed(() =>
  allMenuItems.filter((item) => hasPermission(item.perm)),
);

const mainMenuItems = computed(() => visibleMenuItems.value.slice(0, 4));
const subMenuItems = computed(() => visibleMenuItems.value.slice(4));
const versionLabel = computed(() => {
  const version = systemVersion.value.trim();
  return `服务端 ${formatVersion(version)} · Web ${formatVersion(FRONTEND_VERSION)}`;
});

onMounted(async () => {
  try {
    const response = await getVersion();
    systemVersion.value = response.data?.trim() ?? "";
  } catch {
    systemVersion.value = "";
  }
});

const navigate = (item: MoreMenuItem) => {
  router.push({ name: item.name, query: item.query });
};
</script>
