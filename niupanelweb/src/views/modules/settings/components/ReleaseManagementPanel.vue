<template>
  <section class="mx-auto mt-7 max-w-3xl border-t border-light pt-6">
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div class="min-w-0">
        <div class="flex items-center gap-2">
          <span class="i-ep-refresh-left text-[16px] text-primary"></span>
          <h2 class="m-0 text-[14px] font-bold text-default">版本与回退</h2>
        </div>
        <p class="mt-1 text-[11px] leading-5 text-muted">
          Launcher 以完整 Panel Release 为单位执行激活、健康检查与数据库回退。
        </p>
      </div>
      <div class="flex items-center gap-2">
        <el-button size="small" :loading="loading" @click="loadRuntime">
          <span class="i-ep-refresh mr-1.5"></span>
          刷新
        </el-button>
        <el-button size="small" @click="historyVisible = true">
          <span class="i-ep-clock mr-1.5"></span>
          版本记录
        </el-button>
      </div>
    </div>

    <div
      v-if="runtime && !runtime.launcher_managed"
      class="mt-3 rounded-md border border-amber-300/70 bg-amber-50 px-3 py-2 text-[11px] leading-5 text-amber-800 dark:border-amber-700/60 dark:bg-amber-950/20 dark:text-amber-200"
      role="alert"
    >
      当前进程未由 Launcher 管理，只能查看版本。请使用 <code>niupanel-launcher</code> 启动后再执行更新或回退。
    </div>

    <div
      v-if="runtime?.last_failure"
      class="mt-3 rounded-md border border-rose-300/70 bg-rose-50 px-3 py-2 text-[11px] leading-5 text-rose-700 dark:border-rose-700/60 dark:bg-rose-950/20 dark:text-rose-200"
      role="alert"
    >
      Panel v{{ runtime.last_failure.version }} 激活失败：{{ runtime.last_failure.message }}
    </div>

    <dl class="mt-3 grid overflow-hidden rounded-md border border-light bg-card sm:grid-cols-3 sm:divide-x sm:divide-light">
      <div class="border-b border-light p-3 sm:border-b-0">
        <dt class="text-[10px] font-semibold text-muted">当前版本</dt>
        <dd class="mt-1 font-mono text-[15px] font-bold text-default">
          v{{ runtime?.active_version || "-" }}
        </dd>
      </div>
      <div class="border-b border-light p-3 sm:border-b-0">
        <dt class="text-[10px] font-semibold text-muted">可回退版本</dt>
        <dd class="mt-1 font-mono text-[15px] font-bold text-default">
          {{ runtime?.previous_version ? `v${runtime.previous_version}` : "暂无" }}
        </dd>
      </div>
      <div class="p-3">
        <dt class="text-[10px] font-semibold text-muted">插件进程隔离</dt>
        <dd class="mt-1 flex items-center gap-2 text-[12px] font-bold text-default">
          <span class="h-2 w-2 rounded-full" :class="sandboxDotClass"></span>
          {{ sandboxLabel }}
        </dd>
      </div>
    </dl>

    <div class="mt-3 flex flex-wrap items-center justify-between gap-3 rounded-md border border-light bg-subtle/35 px-3 py-2.5">
      <p class="min-w-0 flex-1 text-[10px] leading-4 text-muted">
        回退会恢复目标版本离开时保存的数据库快照，因此该时间点之后产生的数据会丢失。
      </p>
      <el-button
        v-if="runtime?.previous_version"
        type="warning"
        plain
        size="small"
        :disabled="!runtime.launcher_managed || Boolean(runtime.pending_version) || !previousRelease?.rollback_available"
        :loading="rollingBack === runtime.previous_version"
        @click="rollback(runtime.previous_version)"
      >
        回退至 v{{ runtime.previous_version }}
      </el-button>
    </div>

    <ResponsiveDialog
      v-model:visible="historyVisible"
      title="Panel 版本记录"
      desktop-size="md"
      content-preset="list"
      size="82%"
      append-to-body
    >
      <div class="m-3 overflow-y-auto rounded-md border border-light md:m-4">
        <div
          v-for="release in runtime?.releases || []"
          :key="release.version"
          class="flex items-center gap-3 border-b border-light px-3 py-3 last:border-b-0"
        >
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <span class="font-mono text-[13px] font-bold text-default">v{{ release.version }}</span>
              <span v-if="release.active" class="release-badge bg-emerald-500/10 text-emerald-600 dark:text-emerald-300">当前</span>
              <span v-else-if="release.previous" class="release-badge bg-blue-500/10 text-blue-600 dark:text-blue-300">回退点</span>
            </div>
            <p class="mt-1 text-[10px] text-muted">安装于 {{ formatDate(release.installed_at) }}</p>
          </div>
          <el-button
            v-if="!release.active"
            size="small"
            :disabled="!runtime?.launcher_managed || Boolean(runtime?.pending_version) || !release.rollback_available"
            :loading="rollingBack === release.version"
            @click="rollback(release.version)"
          >
            {{ release.rollback_available ? "回退" : "无快照" }}
          </el-button>
        </div>
        <div v-if="!loading && !runtime?.releases.length" class="px-3 py-8 text-center text-[11px] text-muted">
          暂无版本记录
        </div>
      </div>
    </ResponsiveDialog>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { getPanelReleases, getSystemMeta, rollbackPanelRelease } from "@/api/system";
import type { PanelReleaseList, PluginSandboxMode } from "@/types";
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";

const runtime = ref<PanelReleaseList>();
const sandboxMode = ref<PluginSandboxMode>("unsupported");
const loading = ref(false);
const rollingBack = ref("");
const historyVisible = ref(false);
const previousRelease = computed(() => runtime.value?.releases.find((release) => release.previous));

const sandboxLabel = computed(() => ({
  full: "完整隔离",
  compatible: "兼容隔离",
  degraded: "基础隔离",
  unsupported: "不支持",
}[sandboxMode.value]));

const sandboxDotClass = computed(() => ({
  full: "bg-emerald-500",
  compatible: "bg-blue-500",
  degraded: "bg-amber-500",
  unsupported: "bg-rose-500",
}[sandboxMode.value]));

const errorMessage = (error: unknown, fallback: string) =>
  error instanceof Error ? error.message : fallback;

const loadRuntime = async () => {
  loading.value = true;
  try {
    const [releaseResponse, metaResponse] = await Promise.all([
      getPanelReleases(),
      getSystemMeta(),
    ]);
    runtime.value = releaseResponse.data;
    sandboxMode.value = metaResponse.data.plugin_sandbox.mode;
  } catch (error) {
    ElMessage.error(errorMessage(error, "版本信息加载失败"));
  } finally {
    loading.value = false;
  }
};

const rollback = async (version: string) => {
  await ElMessageBox.confirm(
    `将完整回退到 Panel v${version}，并恢复对应数据库快照。目标快照之后产生的数据会永久丢失。`,
    "确认回退 Panel",
    {
      type: "error",
      confirmButtonText: "确认回退并恢复快照",
      cancelButtonText: "取消",
    },
  );
  rollingBack.value = version;
  try {
    const response = await rollbackPanelRelease(version, true);
    ElMessage.success(response.data.message || "Panel 回退已开始");
    window.setTimeout(() => window.location.reload(), 5000);
  } catch (error) {
    ElMessage.error(errorMessage(error, "Panel 回退失败"));
    rollingBack.value = "";
  }
};

const formatDate = (value: string) => {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN");
};

onMounted(loadRuntime);
</script>

<style scoped>
.release-badge {
  border-radius: 4px;
  padding: 2px 6px;
  font-size: 10px;
  font-weight: 700;
  line-height: 16px;
}
</style>
