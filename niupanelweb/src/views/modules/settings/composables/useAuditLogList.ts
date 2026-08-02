import { nextTick, onMounted, reactive, ref } from "vue";
import { useRouter } from "vue-router";
import { useInfiniteScroll } from "@vueuse/core";
import * as auditApi from "@/api/audit";
import type { AuditLog } from "@/api/audit";
import { useMobile } from "@/composables/useMobile";

const ACTION_LABELS: Record<string, string> = {
  LOGIN: "登录系统",
  LOGOUT: "退出登录",
  TASK_CREATE: "创建任务",
  TASK_UPDATE: "更新任务",
  TASK_DELETE: "删除任务",
  TASK_RUN: "执行任务",
  TASK_STOP: "停止任务",
  VAR_CREATE: "创建变量",
  VAR_UPDATE: "更新变量",
  VAR_DELETE: "删除变量",
  KEY_CREATE: "创建密钥",
  KEY_DELETE: "删除密钥",
  SETTINGS_UPDATE: "更新设置",
};

const RESOURCE_LABELS: Record<string, string> = {
  api_key: "API 密钥",
  environment: "运行环境",
  file: "文件",
  plugin: "插件",
  session: "会话",
  settings: "系统设置",
  task: "任务",
  telegram: "Telegram",
  user: "用户",
  variable: "变量",
  webhook: "Webhook",
};

const DEFAULT_ACTION_STYLE =
  "bg-slate-100 text-slate-500 dark:bg-slate-800/40 dark:text-slate-400";

const normalizeAction = (action: string) =>
  action.trim().replace(/[.\s-]+/g, "_").toUpperCase();

const formatFallbackAction = (action: string) =>
  action
    .split(/[.\s_-]+/)
    .filter(Boolean)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
    .join(" ");

const formatTime = (dateStr: string) => {
  if (!dateStr) return "";
  const date = new Date(dateStr);
  return date.toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
};

const formatDateOnly = (dateStr: string) => {
  if (!dateStr) return "";
  const date = new Date(dateStr);
  return `${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
};

const formatActionText = (action: string) => {
  if (!action) return "UNKNOWN";
  return ACTION_LABELS[normalizeAction(action)] || formatFallbackAction(action);
};

const getActionStyle = (action: string) => {
  if (!action) return DEFAULT_ACTION_STYLE;

  const normalizedAction = normalizeAction(action);

  if (
    normalizedAction.includes("DELETE") ||
    normalizedAction.includes("STOP") ||
    normalizedAction.includes("DISABLE") ||
    normalizedAction.includes("REVOKE")
  ) {
    return "bg-rose-50 text-rose-600 dark:bg-rose-950/25 dark:text-rose-400";
  }
  if (normalizedAction.includes("CREATE") || normalizedAction.includes("ADD")) {
    return "bg-blue-50 text-blue-600 dark:bg-blue-950/25 dark:text-blue-400";
  }
  if (normalizedAction.includes("UPDATE") || normalizedAction.includes("EDIT")) {
    return "bg-purple-50 text-purple-600 dark:bg-purple-950/25 dark:text-purple-400";
  }
  if (
    normalizedAction === "LOGIN" ||
    normalizedAction.includes("RUN") ||
    normalizedAction.includes("START")
  ) {
    return "bg-emerald-50 text-emerald-600 dark:bg-emerald-950/25 dark:text-emerald-400";
  }
  if (normalizedAction === "LOGOUT") {
    return "bg-gray-100 text-gray-500 dark:bg-gray-800/30 dark:text-gray-400";
  }

  return DEFAULT_ACTION_STYLE;
};

const formatActorText = (row: AuditLog) => {
  if (row.actor_type === "User") {
    return row.user_id !== undefined && row.user_id !== null
      ? `用户 ${row.user_id}`
      : "系统";
  }

  return row.user_id !== undefined && row.user_id !== null
    ? `密钥 ${row.user_id}`
    : "API 密钥";
};

const formatResourceText = (resource: string) => {
  const normalized = resource.trim().toLowerCase().replace(/[.\s-]+/g, "_");
  return RESOURCE_LABELS[normalized] || resource || "未知资源";
};

const canOpenResource = (row: AuditLog) =>
  row.resource.trim().toLowerCase() === "task" && Boolean(row.resource_id);

export function useAuditLogList() {
  const { isMobile } = useMobile();
  const router = useRouter();

  const logs = ref<AuditLog[]>([]);
  const loading = ref(false);
  const loadingMore = ref(false);
  const noMore = ref(false);
  const scrollContainerRef = ref<HTMLElement>();
  const scrollContainerRefMobile = ref<HTMLElement>();

  const params = reactive({
    page: 1,
    page_size: 20,
  });

  const loadLogs = async (isReset = false) => {
    if (loading.value || loadingMore.value) return;
    if (!isReset && noMore.value) return;

    if (isReset) {
      loading.value = true;
      params.page = 1;
      noMore.value = false;
    } else {
      loadingMore.value = true;
    }

    try {
      const res = await auditApi.listAuditLogs(params);
      const newItems = res.data.items || [];
      const total = res.data.total;

      if (isReset) {
        logs.value = newItems;
      } else {
        logs.value.push(...newItems);
      }

      if (logs.value.length >= total || newItems.length === 0) {
        noMore.value = true;
      } else {
        params.page += 1;
      }
    } finally {
      loading.value = false;
      loadingMore.value = false;
    }
  };

  const resetAndLoad = () => {
    void loadLogs(true);
  };

  const openResource = (row: AuditLog) => {
    if (canOpenResource(row)) {
      void router.push("/tasks");
    }
  };

  onMounted(async () => {
    await loadLogs(true);
    await nextTick();

    const el = isMobile.value
      ? scrollContainerRefMobile.value
      : scrollContainerRef.value;
    if (!el) return;

    useInfiniteScroll(
      el,
      () => {
        if (!loading.value && !loadingMore.value && !noMore.value) {
          void loadLogs(false);
        }
      },
      { distance: 80 },
    );
  });

  return {
    canOpenResource,
    formatActorText,
    formatActionText,
    formatDateOnly,
    formatResourceText,
    formatTime,
    getActionStyle,
    isMobile,
    loading,
    loadingMore,
    logs,
    noMore,
    openResource,
    resetAndLoad,
    scrollContainerRef,
    scrollContainerRefMobile,
  };
}
