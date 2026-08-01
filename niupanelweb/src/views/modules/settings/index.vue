<template>
  <WorkspaceAppFrame
    v-if="!appStore.isMobile"
    sidebar-class="w-[246px]"
    content-class="overflow-hidden"
  >
    <template #sidebar>
      <nav class="h-full overflow-y-auto p-2 custom-scrollbar">
        <button
          v-for="item in menuItems"
          :key="item.id"
          type="button"
          class="group relative flex h-10 w-full cursor-pointer items-center gap-2.5 rounded-md px-2.5 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/35"
          :class="
            activeSection === item.id
              ? 'bg-card text-default shadow-sm ring-1 ring-black/5 dark:bg-white/[0.06] dark:ring-white/10'
              : 'text-secondary hover:bg-card/70 hover:text-default dark:hover:bg-white/[0.045]'
          "
          @click="activeSection = item.id"
        >
          <span
            v-if="activeSection === item.id"
            class="absolute bottom-2 left-0 top-2 w-0.5 rounded-full bg-primary"
          ></span>
          <span
            class="h-7 w-7 shrink-0 rounded-md flex-center text-[14px]"
            :class="
              activeSection === item.id
                ? 'bg-primary text-white'
                : item.colorClass
            "
          >
            <span :class="item.icon"></span>
          </span>
          <span class="min-w-0 flex-1 truncate text-[13px] font-bold">
            {{ item.label }}
          </span>
        </button>
      </nav>
    </template>

    <div
      :key="activeSection"
      class="h-full overflow-y-auto px-5 py-4 custom-scrollbar"
    >
      <component :is="currentComponent" />
    </div>
  </WorkspaceAppFrame>

  <PageShell v-else :padded="false">
    <div class="flex h-full min-h-0 flex-col overflow-hidden">
      <div class="mobile-dock-safe flex-1 overflow-y-auto p-4 custom-scrollbar">
        <div class="mb-4">
          <h2 class="text-[20px] font-bold text-default">设置</h2>
          <p class="mt-1 text-[12px] font-medium text-muted">{{ versionLabel }}</p>
        </div>

        <div class="overflow-hidden rounded-lg border border-light bg-card divide-y divide-light">
          <button
            v-for="item in menuItems"
            :key="item.id"
            type="button"
            class="flex w-full items-center gap-3 px-4 py-3 text-left transition-colors active:bg-soft"
            @click="openSection(item.id)"
          >
            <div
              class="h-9 w-9 shrink-0 rounded-md flex-center"
              :class="item.colorClass"
            >
              <div :class="item.icon" class="text-base"></div>
            </div>
            <div class="min-w-0 flex-1">
              <div class="truncate text-[13px] font-bold text-default">{{ item.label }}</div>
              <div class="truncate text-[11px] font-medium text-muted">{{ item.desc }}</div>
            </div>
            <div class="i-ep-arrow-right text-sm text-muted/50"></div>
          </button>
        </div>
      </div>
    </div>

    <section
      v-if="appStore.isMobile && drawerVisible"
      class="mobile-dock-safe fixed inset-0 z-[45] flex flex-col bg-base"
    >
      <div class="h-14 shrink-0 border-b border-light bg-card px-4 flex items-center gap-3">
        <button
          class="h-9 w-9 rounded-md bg-soft text-primary flex-center transition-colors hover:bg-primary/15"
          @click="drawerVisible = false"
        >
          <div class="i-ep-back text-lg"></div>
        </button>
        <div class="min-w-0">
          <h3 class="truncate text-[15px] font-bold text-default">{{ currentMenuItem?.label }}</h3>
          <p class="truncate text-[10px] font-medium text-muted">{{ currentMenuItem?.desc }}</p>
        </div>
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto p-5 custom-scrollbar">
        <component :is="currentComponent" />
      </div>
    </section>
  </PageShell>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useRoute } from "vue-router";
import PageShell from "../../../components/common/PageShell.vue";
import WorkspaceAppFrame from "../../../components/workspace/WorkspaceAppFrame.vue";
import GeneralTab from "@/views/modules/settings/GeneralTab.vue";
import NotificationTab from "@/views/modules/settings/NotificationTab.vue";
import SecurityTab from "@/views/modules/settings/SecurityTab.vue";
import MaintenanceTab from "@/views/modules/settings/MaintenanceTab.vue";
import AboutTab from "@/views/modules/settings/AboutTab.vue";
import ApiDocumentsTab from "@/views/modules/settings/ApiDocumentsTab.vue";
import ApiKeyTab from "@/views/modules/settings/ApiKeyTab.vue";
import AuditLogTab from "@/views/modules/settings/AuditLogTab.vue";
import { useAppStore } from "../../../stores/app";
import { useHaptics } from "../../../composables/useHaptics";
import { getVersion } from "@/api/settings";
import { formatVersion } from "@/version";

const systemVersion = ref("");
onMounted(async () => {
  try {
    const res = await getVersion();
    if (res.data) {
      systemVersion.value = res.data;
    }
  } catch (error) {
    console.error("Failed to fetch system version", error);
  }
});

const appStore = useAppStore();
const route = useRoute();
const haptics = useHaptics();
const activeSection = ref("basic");
const drawerVisible = ref(false);
const versionLabel = computed(
  () => `Panel ${formatVersion(systemVersion.value)}`,
);

const menuItems = [
  {
    id: "basic",
    label: "常规设置",
    desc: "系统品牌、时区和任务并发限制等基础配置",
    icon: "i-ep-setting",
    colorClass: "bg-blue-50 dark:bg-blue-950/25 text-blue-500",
    component: GeneralTab,
  },
  {
    id: "notification",
    label: "通知配置",
    desc: "Webhook 和 SMTP 邮件服务器配置",
    icon: "i-ep-bell",
    colorClass: "bg-orange-50 dark:bg-orange-950/25 text-orange-500",
    component: NotificationTab,
  },
  {
    id: "security",
    label: "隐私与安全",
    desc: "账户凭据管理、密码修改及会话控制",
    icon: "i-ep-lock",
    colorClass: "bg-rose-50 dark:bg-rose-950/25 text-rose-500",
    component: SecurityTab,
  },
  {
    id: "keys",
    label: "API 访问",
    desc: "SDK 调用和自动化脚本的安全访问令牌",
    icon: "i-ep-key",
    colorClass: "bg-amber-50 dark:bg-amber-950/25 text-amber-500",
    component: ApiKeyTab,
  },
  {
    id: "docs",
    label: "API 文档",
    desc: "系统开放接口定义和开发者文档",
    icon: "i-ep-document",
    colorClass: "bg-cyan-50 dark:bg-cyan-950/25 text-cyan-500",
    component: ApiDocumentsTab,
  },
  {
    id: "audit",
    label: "审计日志",
    desc: "系统关键操作的可追溯历史记录",
    icon: "i-ep-list",
    colorClass: "bg-emerald-50 dark:bg-emerald-950/25 text-emerald-500",
    component: AuditLogTab,
  },
  {
    id: "maintenance",
    label: "系统维护",
    desc: "数据库备份、数据恢复及运行日志清理",
    icon: "i-ep-tools",
    colorClass: "bg-indigo-50 dark:bg-indigo-950/25 text-indigo-500",
    component: MaintenanceTab,
  },
  {
    id: "about",
    label: "版本与更新",
    desc: "管理完整 Panel Release 的更新通道、版本与回退",
    icon: "i-ep-info-filled",
    colorClass: "bg-slate-100 dark:bg-slate-800/35 text-slate-500",
    component: AboutTab,
  },
];

const syncSectionFromRoute = () => {
  const section = typeof route.query.section === "string" ? route.query.section : "";
  if (menuItems.some((item) => item.id === section)) activeSection.value = section;
};

watch(() => route.query.section, syncSectionFromRoute, { immediate: true });

const currentMenuItem = computed(() =>
  menuItems.find((i) => i.id === activeSection.value),
);
const currentComponent = computed(() => currentMenuItem.value?.component);

const openSection = (id: string) => {
  haptics.impact();
  activeSection.value = id;
  drawerVisible.value = true;
};
</script>
