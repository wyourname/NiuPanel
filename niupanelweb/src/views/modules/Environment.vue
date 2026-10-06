<template>
  <div class="h-full min-h-0">
    <WorkspaceAppFrame
      v-if="!appStore.isMobile"
      content-class="overflow-hidden"
    >
      <template #toolbar>
        <EnvironmentToolbar
          v-model:filter-type="filterType"
          :is-mobile="appStore.isMobile"
          :loading="loading"
          @create="createDialogVisible = true"
          @open-jobs="jobListVisible = true"
          @open-mirror="mirrorDialogVisible = true"
        />
      </template>

      <PullToRefresh
        :on-refresh="loadEnvironments"
        disabled
        class="flex min-h-0 flex-1 flex-col"
      >
        <EnvTable
          :data="filteredEnvironments"
          :loading="loading"
          @view-logs="handleViewLogs"
          @manage-packages="showPackages"
          @delete="deleteEnvironment"
          @create="handleRestoreEnvironment"
          @set-default="handleSetNodeDefault"
        />
      </PullToRefresh>

      <template #status>
        <div class="flex items-center justify-between gap-3">
          <span>{{ environmentFilterLabel }} · {{ filteredEnvironments.length }} 个条目</span>
          <span v-if="loading" class="text-primary">加载中</span>
        </div>
      </template>
    </WorkspaceAppFrame>

    <PageShell v-else compact>
      <div class="module-panel flex min-h-0 flex-1 flex-col overflow-hidden">
        <EnvironmentToolbar
          v-model:filter-type="filterType"
          :is-mobile="appStore.isMobile"
          :loading="loading"
          @create="createDialogVisible = true"
          @open-jobs="jobListVisible = true"
          @open-mirror="mirrorDialogVisible = true"
        />

        <EnvironmentSummaryBar
          :count="filteredEnvironments.length"
          :filter-type="filterType"
          :is-mobile="appStore.isMobile"
        />

        <PullToRefresh
          :on-refresh="loadEnvironments"
          :disabled="!appStore.isMobile"
          class="flex min-h-0 flex-1 flex-col"
        >
          <EnvTable
            :data="filteredEnvironments"
            :loading="loading"
            @view-logs="handleViewLogs"
            @manage-packages="showPackages"
            @delete="deleteEnvironment"
            @create="handleRestoreEnvironment"
            @set-default="handleSetNodeDefault"
          />
        </PullToRefresh>
      </div>
    </PageShell>

    <EnvJobListDialog
      v-model="jobListVisible"
      @show-log="showInstallLogDialog"
    />

    <EnvCreateDialog
      v-model="createDialogVisible"
      :default-env-type="filterType === 'node' ? 'node' : 'python'"
      @show-log="handleEnvironmentCreated"
    />

    <EnvMirrorDialog v-model="mirrorDialogVisible" :filter-type="filterType" />

    <EnvPackageManagerDialog
      v-model="packageDialogVisible"
      :env="currentEnv"
      @show-log="showInstallLogDialog"
    />

    <EnvLogDialog
      ref="logDialogRef"
      v-model="logDialogVisible"
      :title="currentLogTitle"
      :connection-state="logConnectionState"
      @retry-connection="retryLogConnection"
      :is-mobile="appStore.isMobile"
    />
  </div>
</template>

<script setup lang="ts">
import { getJobLogContent } from "@/api/jobs";
import { createLogConnection, type LogConnectionState } from "@/utils/logConnection";
import { ref, onMounted, onUnmounted, nextTick, computed, watch } from "vue";
import { useRoute } from "vue-router";
import { ElMessage, ElMessageBox } from "element-plus";
import * as envApi from "../../api/environment";
import { getJob } from "../../api/jobs";
import request from "../../utils/request";
import { useAppStore } from "../../stores/app";
import PullToRefresh from "../../components/common/PullToRefresh.vue";
import PageShell from "../../components/common/PageShell.vue";
import WorkspaceAppFrame from "../../components/workspace/WorkspaceAppFrame.vue";
import type { Env, EnvType, InstallableEnvType, LogViewerRef } from "@/types";

// Sub-components
import EnvTable from "./environment/components/EnvTable.vue";
import EnvJobListDialog from "./environment/components/EnvJobListDialog.vue";
import EnvCreateDialog from "./environment/components/EnvCreateDialog.vue";
import EnvLogDialog from "./environment/components/EnvLogDialog.vue";
import EnvMirrorDialog from "./environment/components/EnvMirrorDialog.vue";
import EnvPackageManagerDialog from "./environment/components/EnvPackageManagerDialog.vue";
import EnvironmentSummaryBar from "./environment/components/EnvironmentSummaryBar.vue";
import EnvironmentToolbar from "./environment/components/EnvironmentToolbar.vue";

const appStore = useAppStore();
const route = useRoute();

const loading = ref(false);
const environments = ref<Env[]>([]);
const filterType = ref<EnvType>("python");
const searchQuery = ref("");

const filteredEnvironments = computed(() => {
  let list = environments.value.filter(
    (env) => env.env_type === filterType.value,
  );
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase();
    list = list.filter(
      (env) =>
        env.name.toLowerCase().includes(q) ||
        (env.path && env.path.toLowerCase().includes(q)),
    );
  }
  return list;
});

const environmentFilterLabel = computed(() => {
  if (filterType.value === "node") return "Node.js";
  if (filterType.value === "python") return "Python";
  return "Linux";
});

// Dialog visibilities
const jobListVisible = ref(false);
const createDialogVisible = ref(false);
const mirrorDialogVisible = ref(false);
const packageDialogVisible = ref(false);
const currentEnv = ref<Env | null>(null);

// Logs logic
const logDialogVisible = ref(false);
const currentLogTitle = ref("");
const logDialogRef = ref<LogViewerRef | null>(null);
let eventSource: ReturnType<typeof createLogConnection> | null = null;
const logConnectionState = ref<LogConnectionState>('idle');
const retryLogConnection = () => eventSource?.retry();
const pendingEnvironmentJobs = new Set<number>();
let environmentJobTimer: ReturnType<typeof setTimeout> | undefined;
let checkingEnvironmentJobs = false;
let disposed = false;
const restoringEnvironments = new Map<string, number | null>();

const pollEnvironmentJobs = async () => {
  if (disposed || checkingEnvironmentJobs) return;
  checkingEnvironmentJobs = true;
  clearTimeout(environmentJobTimer);
  let changed = false;
  await Promise.allSettled([...pendingEnvironmentJobs].map(async id => {
    const response = await getJob(id);
    if (!["Pending", "Running"].includes(response.data.status)) {
      pendingEnvironmentJobs.delete(id);
      for (const [key, jobId] of restoringEnvironments) {
        if (jobId === id) restoringEnvironments.delete(key);
      }
      changed = true;
    }
  }));
  checkingEnvironmentJobs = false;
  if (disposed) return;
  if (changed) void loadEnvironments();
  if (pendingEnvironmentJobs.size) environmentJobTimer = setTimeout(pollEnvironmentJobs, 2000);
};

const handleEnvironmentCreated = (id: number | string, name: string) => {
  pendingEnvironmentJobs.add(Number(id));
  void pollEnvironmentJobs();
  void loadEnvironments();
  showInstallLogDialog(id, name);
};

const clearLogViewer = () => {
  logDialogRef.value?.clear?.();
};

const loadEnvironments = async () => {
  loading.value = true;
  try {
    const res = await envApi.getEnvironments();
    environments.value = res.data;
  } catch (error) {
    ElMessage.error("加载环境列表失败");
  } finally {
    loading.value = false;
  }
};

const deleteEnvironment = async (env: Env) => {
  try {
    await ElMessageBox.confirm(`确定要删除环境 ${env.name} 吗？`, "警告", {
      confirmButtonText: "确定",
      cancelButtonText: "取消",
      type: "warning",
    });
    await envApi.deleteEnvironment(env);
    ElMessage.success("删除成功");
    loadEnvironments();
  } catch (error) {}
};

const showPackages = (env: Env) => {
  currentEnv.value = env;
  packageDialogVisible.value = true;
};

const handleRestoreEnvironment = async (env: Env) => {
  const key = `${env.env_type}:${env.name}`;
  if (!env.version || restoringEnvironments.has(key)) return;
  restoringEnvironments.set(key, null);
  try {
    const envType: InstallableEnvType =
      env.env_type === "node" ? "node" : "python";
    const response = await envApi.createEnvironment({ version: env.version }, envType);
    restoringEnvironments.set(key, response.data);
    handleEnvironmentCreated(response.data, `恢复环境 · ${env.name}`);
    ElMessage.success("恢复任务已提交");
  } catch (e) { restoringEnvironments.delete(key); }
};

const handleSetNodeDefault = async (env: Env) => {
  try {
    await envApi.setNodeDefault(env.name);
    ElMessage.success(`已将 Node.js ${env.name} 设为系统默认版本`);
    loadEnvironments();
  } catch (e) {
    ElMessage.error("切换失败，请确认该版本已安装");
  }
};

const showInstallLogDialog = (id: number | string, name: string) => {
  currentLogTitle.value = name;
  logDialogVisible.value = true;
  nextTick(() => {
    if (!logDialogVisible.value || disposed) return;
    clearLogViewer();
    if (eventSource) eventSource.close();

    eventSource = createLogConnection({
      open: () => new EventSource(`${request.defaults.baseURL}/jobs/${encodeURIComponent(id)}/logs`, { withCredentials: true }),
      readMissing: async (offset, limit) => (await getJobLogContent(id, offset, limit)).data,
      write: (content, reset) => { if (reset) logDialogRef.value?.reset?.(); logDialogRef.value?.write?.(content); },
      status: state => { logConnectionState.value = state; },
    });
  });
};

const handleViewLogs = (env: Env) => {
  showInstallLogDialog(env.name, `环境日志 - ${env.name}`);
};

const handleJobFinished = () => {
  // Refresh environments whenever a system job finishes
  loadEnvironments();
};

watch(logDialogVisible, (visible) => {
  if (!visible) { eventSource?.close(); eventSource = null; }
});

watch(
  () => route.query.q,
  (newQ) => {
    if (typeof newQ === "string") {
      searchQuery.value = newQ;
    }
  },
);

onMounted(() => {
  if (typeof route.query.q === "string") {
    searchQuery.value = route.query.q;
  }
  loadEnvironments();
  window.addEventListener("niu:job-finished", handleJobFinished);
});

onUnmounted(() => {
  disposed = true;
  clearTimeout(environmentJobTimer);
  if (eventSource) eventSource.close();
  window.removeEventListener("niu:job-finished", handleJobFinished);
});
</script>
