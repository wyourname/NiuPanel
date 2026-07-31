<template>
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

    <PullToRefresh :on-refresh="loadEnvironments" disabled class="flex min-h-0 flex-1 flex-col">
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
    <div class="module-panel flex-1 min-h-0 overflow-hidden flex flex-col">
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

      <PullToRefresh :on-refresh="loadEnvironments" :disabled="!appStore.isMobile" class="flex-1 flex flex-col min-h-0">
        <EnvTable :data="filteredEnvironments" :loading="loading" @view-logs="handleViewLogs"
          @manage-packages="showPackages" @delete="deleteEnvironment" @create="handleRestoreEnvironment"
          @set-default="handleSetNodeDefault" />
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
    @show-log="showInstallLogDialog"
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
    :is-mobile="appStore.isMobile"
  />
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, computed, watch } from "vue";
import { useRoute } from "vue-router";
import { ElMessage, ElMessageBox } from "element-plus";
import * as envApi from "../../api/environment";
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
let eventSource: EventSource | null = null;

const getEventSourceData = (event: Event) =>
  event instanceof MessageEvent && typeof event.data === "string"
    ? event.data
    : "";

const getLogContent = (event: Event) => {
  const data = getEventSourceData(event);
  if (!data) return "";

  try {
    const parsed: unknown = JSON.parse(data);
    if (typeof parsed === "string") return parsed;
    if (
      parsed &&
      typeof parsed === "object" &&
      "content" in parsed &&
      typeof parsed.content === "string"
    ) {
      return parsed.content;
    }
  } catch {
    // Plain text fallback for legacy payloads.
  }

  return data;
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
  if (!env.version) return;
  try {
    const envType: InstallableEnvType =
      env.env_type === "node" ? "node" : "python";
    await envApi.createEnvironment({ version: env.version }, envType);
    ElMessage.success("恢复任务已提交");
  } catch (e) {}
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
    clearLogViewer();
    if (eventSource) eventSource.close();

    // Use the same event listener pattern as Tasks module log streaming
    eventSource = new EventSource(
      `${request.defaults.baseURL}/jobs/${id}/logs`,
    );

    eventSource.addEventListener("log", (event) => {
      const data = getLogContent(event);
      if (data) logDialogRef.value?.write?.(data);
    });

    eventSource.addEventListener("history", (event) => {
      const data = getEventSourceData(event);
      logDialogRef.value?.reset?.();
      if (data) logDialogRef.value?.write?.(data);
    });

    eventSource.onerror = () => {
      if (eventSource) eventSource.close();
    };
  });
};

const handleViewLogs = (env: Env) => {
  showInstallLogDialog(env.name, `环境日志 - ${env.name}`);
};

const handleJobFinished = () => {
  // Refresh environments whenever a system job finishes
  loadEnvironments();
};

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
  if (eventSource) eventSource.close();
  window.removeEventListener("niu:job-finished", handleJobFinished);
});
</script>
