import { onMounted, onScopeDispose, ref, type Ref } from "vue";
import * as echarts from "echarts";
import { useAppStore } from "@/stores/app";
import type { OverviewChartData } from "@/types";

export function useOverviewChart(chartData: Ref<OverviewChartData>) {
  const appStore = useAppStore();
  const trendChartRef = ref<HTMLElement | null>(null);
  let trendChart: echarts.ECharts | null = null;
  let initRetryTimer: ReturnType<typeof setTimeout> | null = null;

  const clearInitRetry = () => {
    if (initRetryTimer) clearTimeout(initRetryTimer);
    initRetryTimer = null;
  };

  const initCharts = () => {
    if (!trendChartRef.value) return;

    const dom = trendChartRef.value;

    if (dom.clientWidth === 0 || dom.clientHeight === 0) {
      clearInitRetry();
      initRetryTimer = setTimeout(() => {
        initCharts();
      }, 100);
      return;
    }

    if (!trendChart) {
      trendChart = echarts.init(dom);
    }
  };

  const updateCharts = () => {
    if (!trendChart) return;

    const isDark = appStore.isDark;
    const isMobile = appStore.isMobile;
    const textColor = isDark ? "#7f91a4" : "#707579";
    const gridColor = isDark ? "#101921" : "#f1f5f9";

    trendChart.setOption({
      tooltip: {
        trigger: "axis",
        axisPointer: { type: "shadow" },
        backgroundColor: isDark ? "#17212b" : "#fff",
        borderColor: gridColor,
        textStyle: { color: textColor, fontSize: 10 },
      },
      grid: {
        left: "0%",
        right: "0%",
        bottom: "5%",
        top: "10%",
        containLabel: true,
      },
      xAxis: {
        type: "category",
        data: chartData.value.hours,
        axisLabel: { interval: isMobile ? 5 : 2, color: textColor, fontSize: 9 },
        axisLine: { show: false },
        axisTick: { show: false },
      },
      yAxis: {
        type: "value",
        splitLine: { lineStyle: { color: gridColor, type: "dashed" } },
        axisLabel: { color: textColor, fontSize: 9 },
      },
      series: [
        {
          name: "成功",
          type: "bar",
          stack: "total",
          itemStyle: { color: "#10b981", borderRadius: [0, 0, 0, 0] },
          barWidth: "40%",
          data: chartData.value.success,
        },
        {
          name: "失败",
          type: "bar",
          stack: "total",
          itemStyle: { color: "#ef4444" },
          data: chartData.value.failed,
        },
      ],
    });
  };

  const resizeTrendChart = () => {
    trendChart?.resize();
  };

  onMounted(() => {
    window.addEventListener("resize", resizeTrendChart);
  });

  onScopeDispose(() => {
    clearInitRetry();
    window.removeEventListener("resize", resizeTrendChart);
    trendChart?.dispose();
    trendChart = null;
  });

  return {
    initCharts,
    trendChartRef,
    updateCharts,
  };
}
