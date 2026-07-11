import { computed, nextTick, onMounted, onScopeDispose, ref } from "vue";
import { getSystemOverview } from "@/api/overview";
import type {
  OverviewActivityItem,
  OverviewChartData,
  OverviewSystemInfo,
  OverviewTaskStats,
} from "@/types";

type OverviewChartHooks = {
  initCharts: () => void;
  updateCharts: () => void;
};

export function useOverviewData() {
  const loading = ref(true);
  const stats = ref<OverviewTaskStats>({
    total: 0,
    running: 0,
    failed_today: 0,
    next_run: null,
  });
  const sysInfo = ref<OverviewSystemInfo>({
    cpu_usage: 0,
    memory_total: 0,
    memory_used: 0,
    disk_total: 0,
    disk_used: 0,
    uptime: 0,
    os_info: "-",
    public_ip: null,
  });
  const recentActivity = ref<OverviewActivityItem[]>([]);
  const chartData = ref<OverviewChartData>({
    hours: [],
    success: [],
    failed: [],
  });

  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let chartHooks: OverviewChartHooks | null = null;

  const setChartHooks = (hooks: OverviewChartHooks) => {
    chartHooks = hooks;
  };

  const nextRunTime = computed(() => {
    if (stats.value.next_run === null) return "暂无计划";

    const diff = stats.value.next_run - Date.now() / 1000;
    if (diff < 0) return "即将执行";
    if (diff < 60) return `${Math.max(0, Math.floor(diff))} 秒后`;
    return new Date(stats.value.next_run * 1000).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
  });

  const diskPercentage = computed(() =>
    sysInfo.value.disk_total === 0
      ? 0
      : Math.round((sysInfo.value.disk_used / sysInfo.value.disk_total) * 100)
  );

  const loadData = async (isPolling = false) => {
    if (!isPolling) loading.value = true;
    try {
      const res = await getSystemOverview();
      const data = res.data;
      stats.value = data.task_stats;
      sysInfo.value = {
        cpu_usage: parseFloat(data.cpu_usage.toFixed(1)),
        memory_total: data.memory_total,
        memory_used: data.memory_used,
        disk_total: data.disk_total,
        disk_used: data.disk_used,
        uptime: data.uptime,
        os_info: data.os_info,
        public_ip: data.public_ip,
      };
      recentActivity.value = data.recent_activity;
      chartData.value = data.chart_data;
      if (!loading.value || isPolling) chartHooks?.updateCharts();
    } finally {
      if (!isPolling) {
        loading.value = false;
        void nextTick(() => {
          chartHooks?.initCharts();
          chartHooks?.updateCharts();
        });
      }
    }
  };

  onMounted(() => {
    void loadData();
    pollTimer = setInterval(() => {
      void loadData(true);
    }, 10000);
  });

  onScopeDispose(() => {
    if (pollTimer) clearInterval(pollTimer);
  });

  return {
    chartData,
    diskPercentage,
    loadData,
    loading,
    nextRunTime,
    recentActivity,
    setChartHooks,
    stats,
    sysInfo,
  };
}
