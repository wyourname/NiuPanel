<template>
  <PullToRefresh :on-refresh="() => loadData(true)" :disabled="!appStore.isMobile">
    <div
      class="relative flex h-full flex-col overflow-x-hidden overflow-y-auto custom-scrollbar"
      :class="appStore.isMobile ? 'gap-3 p-3' : 'gap-3 p-3'"
    >
      <OverviewStatsGrid
        :loading="loading"
        :next-run-time="nextRunTime"
        :stats="stats"
        :sys-info="sysInfo"
        @open-tasks="openTasks"
      />

      <div class="grid w-full shrink-0 grid-cols-1 gap-3 xl:grid-cols-[minmax(0,1fr)_320px]">
        <div
          class="flex min-h-[250px] min-w-0 flex-col overflow-hidden rounded-md border border-light bg-card md:min-h-[320px]"
        >
          <div class="flex min-h-11 items-center justify-between border-b border-light/70 px-4 py-2.5">
            <div class="flex items-center gap-2">
              <div class="i-ep-data-analysis text-primary text-sm"></div>
              <span class="text-[13px] font-bold text-default">24 小时执行趋势</span>
            </div>
            <div class="flex gap-3">
              <div class="flex items-center gap-1.5">
                <div class="w-2 h-2 rounded-full bg-emerald-500"></div>
                <span class="text-[10px] text-muted">成功</span>
              </div>
              <div class="flex items-center gap-1.5">
                <div class="w-2 h-2 rounded-full bg-rose-500"></div>
                <span class="text-[10px] text-muted">失败</span>
              </div>
            </div>
          </div>
          <div class="relative h-full w-full flex-1 box-border p-3 md:p-4">
            <div ref="trendChartRef" class="full"></div>
          </div>
        </div>

        <OverviewSystemCard
          :disk-percentage="diskPercentage"
          :loading="loading"
          :sys-info="sysInfo"
          @open-share="openShare"
          @open-tasks="openTasks"
        />
      </div>

      <OverviewActivityFeed
        :activity="recentActivity"
        :is-mobile="appStore.isMobile"
        :loading="loading"
        @open-audit="openAudit"
      />
    </div>
  </PullToRefresh>
</template>

<script setup lang="ts">
import { useRouter } from "vue-router";
import { useAppStore } from "../../../stores/app";
import PullToRefresh from "../../../components/common/PullToRefresh.vue";
import OverviewActivityFeed from "./components/OverviewActivityFeed.vue";
import OverviewStatsGrid from "./components/OverviewStatsGrid.vue";
import OverviewSystemCard from "./components/OverviewSystemCard.vue";
import { useOverviewChart } from "./composables/useOverviewChart";
import { useOverviewData } from "./composables/useOverviewData";

const appStore = useAppStore();
const router = useRouter();

const {
  chartData,
  diskPercentage,
  loadData,
  loading,
  nextRunTime,
  recentActivity,
  setChartHooks,
  stats,
  sysInfo,
} = useOverviewData();

const {
  initCharts,
  trendChartRef,
  updateCharts,
} = useOverviewChart(chartData);

setChartHooks({
  initCharts,
  updateCharts,
});

const openTasks = () => {
  void router.push("/tasks");
};

const openShare = () => {
  void router.push("/share");
};

const openAudit = () => {
  void router.push({ name: "settings", query: { section: "audit" } });
};
</script>
