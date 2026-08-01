<template>
  <div class="mx-auto mt-8 max-w-3xl border-t border-light pt-6">
    <section>
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div class="flex items-center gap-2">
            <span class="i-ep-cpu text-[16px] text-primary"></span>
            <h2 class="m-0 text-[14px] font-bold text-default">Core 版本</h2>
          </div>
          <p class="mt-1 text-[11px] leading-5 text-muted">
            Core 更新由 launcher 激活，并在启动失败时自动恢复 SQLite 快照。
          </p>
        </div>
        <el-button size="small" :loading="loading" @click="loadReleases">
          <span class="i-ep-refresh mr-1.5"></span>
          刷新
        </el-button>
      </div>

      <div
        v-if="core && !core.launcher_managed"
        class="mt-3 rounded-md border border-amber-300/70 bg-amber-50 px-3 py-2 text-[11px] leading-5 text-amber-800 dark:border-amber-700/60 dark:bg-amber-950/20 dark:text-amber-200"
      >
        当前开发进程未由 launcher 启动，可以查看版本，但不能执行 Core 更新或回退。
      </div>

      <div
        v-if="core?.last_failure"
        class="mt-3 rounded-md border border-rose-300/70 bg-rose-50 px-3 py-2 text-[11px] leading-5 text-rose-700 dark:border-rose-700/60 dark:bg-rose-950/20 dark:text-rose-200"
      >
        最近一次 Core {{ core.last_failure.version }} 激活失败：{{ core.last_failure.message }}
      </div>

      <div class="mt-3 overflow-hidden rounded-md border border-light">
        <div
          v-for="release in core?.releases || []"
          :key="release.version"
          class="flex flex-wrap items-center gap-3 border-b border-light px-3 py-3 last:border-b-0"
        >
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <span class="font-mono text-[13px] font-bold text-default">v{{ release.version }}</span>
              <span v-if="release.active" class="release-badge bg-emerald-500/10 text-emerald-600 dark:text-emerald-300">
                当前
              </span>
              <span v-else-if="release.previous" class="release-badge bg-blue-500/10 text-blue-600 dark:text-blue-300">
                上一版
              </span>
            </div>
            <p class="mt-1 text-[10px] text-muted">
              Schema {{ release.schema_epoch }}.{{ release.schema_revision }} · API {{ release.api_contract }} · {{ release.target }}
            </p>
          </div>
          <el-button
            v-if="!release.active"
            size="small"
            :disabled="!core?.launcher_managed || Boolean(core?.pending_version)"
            :loading="activatingCore === release.version"
            @click="activateCore(release)"
          >
            <span class="i-ep-refresh-left mr-1.5"></span>
            切换到此版本
          </el-button>
        </div>
        <div v-if="!loading && !core?.releases.length" class="px-3 py-6 text-center text-[11px] text-muted">
          暂无 Core 版本记录
        </div>
      </div>
    </section>

    <section class="mt-8 border-t border-light pt-6">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div class="flex items-center gap-2">
            <span class="i-ep-monitor text-[16px] text-primary"></span>
            <h2 class="m-0 text-[14px] font-bold text-default">Web UI 版本</h2>
          </div>
          <p class="mt-1 text-[11px] leading-5 text-muted">
            Web UI 独立安装和切换，无需重启 Core；激活前会校验兼容范围与文件哈希。
          </p>
        </div>
        <div class="flex gap-2">
          <el-button size="small" :loading="checkingWebUpdate" @click="checkOnlineWebUpdate">
            <span class="i-ep-search mr-1.5"></span>
            检查更新
          </el-button>
          <el-button
            size="small"
            :disabled="!web?.previous_version"
            :loading="rollingBackWeb"
            @click="rollbackWeb"
          >
            <span class="i-ep-refresh-left mr-1.5"></span>
            回退 UI
          </el-button>
          <el-button type="primary" size="small" :loading="uploadingWeb" @click="webFileInput?.click()">
            <span class="i-ep-upload mr-1.5"></span>
            上传 UI 包
          </el-button>
        </div>
      </div>

      <div
        v-if="webUpdateInfo"
        class="mt-3 flex flex-wrap items-center gap-3 rounded-md border border-light bg-subtle px-3 py-2"
      >
        <div class="min-w-0 flex-1 text-[11px] text-secondary">
          <span class="font-bold text-default">Web UI v{{ webUpdateInfo.version }}</span>
          <span class="ml-2">{{ formatFileSize(webUpdateInfo.size) }} · {{ webUpdateInfo.channel === "preview" ? "预览" : "正式" }}</span>
          <span v-if="!webUpdateInfo.update_available" class="ml-2 text-emerald-600 dark:text-emerald-300">当前已是最新版本</span>
        </div>
        <el-button
          v-if="webUpdateInfo.update_available"
          type="primary"
          size="small"
          :loading="installingWebUpdate"
          @click="installOnlineWebUpdate"
        >
          <span class="i-ep-download mr-1.5"></span>
          下载并激活
        </el-button>
      </div>

      <input
        ref="webFileInput"
        type="file"
        class="hidden"
        accept=".tar.gz,application/gzip"
        @change="uploadWeb"
      />

      <div
        v-if="uploadingWeb"
        class="mt-3 flex items-center gap-3 rounded-md border border-primary/15 bg-primary/[0.04] px-3 py-2"
        role="status"
        aria-live="polite"
      >
        <div class="min-w-0 flex-1">
          <div class="mb-1 flex items-center justify-between gap-3 text-[10px] font-semibold text-secondary">
            <span>正在上传并校验 UI 包</span>
            <span class="font-mono">{{ webUploadProgress }}%</span>
          </div>
          <el-progress :percentage="webUploadProgress" :show-text="false" :stroke-width="5" />
        </div>
        <el-button size="small" plain type="danger" @click="cancelWebUpload">
          取消
        </el-button>
      </div>

      <div class="mt-3 overflow-hidden rounded-md border border-light">
        <div
          v-for="release in web?.releases || []"
          :key="`${release.version}-${release.managed}`"
          class="flex flex-wrap items-center gap-3 border-b border-light px-3 py-3 last:border-b-0"
        >
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <span class="font-mono text-[13px] font-bold text-default">v{{ release.version }}</span>
              <span v-if="release.active" class="release-badge bg-emerald-500/10 text-emerald-600 dark:text-emerald-300">
                当前
              </span>
              <span v-else-if="release.previous" class="release-badge bg-blue-500/10 text-blue-600 dark:text-blue-300">
                上一版
              </span>
              <span
                v-if="!release.compatible"
                class="release-badge bg-rose-500/10 text-rose-600 dark:text-rose-300"
                :title="release.compatibility_error || ''"
              >
                不兼容
              </span>
              <span v-if="!release.managed" class="release-badge bg-soft text-secondary">内置</span>
            </div>
            <p class="mt-1 text-[10px] text-muted">
              API {{ release.manifest?.api_contract ?? "-" }} · Core {{ release.manifest?.core.min || "-" }}+
            </p>
          </div>
          <el-button
            v-if="!release.active && release.managed"
            size="small"
            :disabled="!release.compatible"
            :loading="activatingWeb === release.version"
            @click="activateWeb(release.version)"
          >
            <span class="i-ep-switch mr-1.5"></span>
            激活
          </el-button>
        </div>
        <div v-if="!loading && !web?.releases.length" class="px-3 py-6 text-center text-[11px] text-muted">
          暂无 Web UI 版本记录
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import {
  activateCoreRelease,
  activateWebRelease,
  checkWebUpdate,
  getCoreReleases,
  getWebReleases,
  installWebUpdate,
  rollbackWebRelease,
  uploadWebRelease,
} from "@/api/system";
import { createUploadFormData } from "@/api/upload";
import type {
  CoreReleaseList,
  CoreReleaseRecord,
  WebReleaseList,
  WebUpdateInfo,
} from "@/types";
import { useUploadTransfer } from "@/composables/useUploadTransfer";
import { formatFileSize } from "@/utils/format";

const core = ref<CoreReleaseList>();
const web = ref<WebReleaseList>();
const loading = ref(false);
const activatingCore = ref("");
const activatingWeb = ref("");
const rollingBackWeb = ref(false);
const webFileInput = ref<HTMLInputElement>();
const checkingWebUpdate = ref(false);
const installingWebUpdate = ref(false);
const webUpdateInfo = ref<WebUpdateInfo>();
const {
  cancel: cancelWebUpload,
  progress: webUploadProgress,
  run: runWebUpload,
  uploading: uploadingWeb,
} = useUploadTransfer();

const errorMessage = (error: unknown, fallback: string) =>
  error instanceof Error ? error.message : fallback;

const loadReleases = async () => {
  loading.value = true;
  try {
    const [coreResponse, webResponse] = await Promise.all([
      getCoreReleases(),
      getWebReleases(),
    ]);
    core.value = coreResponse.data;
    web.value = webResponse.data;
  } catch (error) {
    ElMessage.error(errorMessage(error, "版本信息加载失败"));
  } finally {
    loading.value = false;
  }
};

const activateCore = async (release: CoreReleaseRecord) => {
  const active = core.value?.releases.find((item) => item.active);
  const crossEpoch = Boolean(active && active.schema_epoch !== release.schema_epoch);
  const message = crossEpoch
    ? `切换到 Core v${release.version} 需要恢复旧数据库快照，升级后产生的数据会丢失。确定继续吗？`
    : `确定切换到 Core v${release.version} 吗？launcher 会在失败时自动恢复当前版本。`;
  await ElMessageBox.confirm(message, "切换 Core 版本", {
    type: crossEpoch ? "error" : "warning",
    confirmButtonText: crossEpoch ? "确认恢复快照" : "确认切换",
    cancelButtonText: "取消",
  });
  activatingCore.value = release.version;
  try {
    const response = await activateCoreRelease(release.version, crossEpoch);
    ElMessage.success(response.data?.message || "Core 版本切换已开始");
    window.setTimeout(() => window.location.reload(), 5000);
  } catch (error) {
    ElMessage.error(errorMessage(error, "Core 版本切换失败"));
  } finally {
    activatingCore.value = "";
  }
};

const activateWeb = async (version: string) => {
  await ElMessageBox.confirm(`确定激活 Web UI v${version} 吗？`, "切换 Web UI", {
    type: "warning",
    confirmButtonText: "激活",
    cancelButtonText: "取消",
  });
  activatingWeb.value = version;
  try {
    await activateWebRelease(version);
    window.location.reload();
  } catch (error) {
    ElMessage.error(errorMessage(error, "Web UI 激活失败"));
    activatingWeb.value = "";
  }
};

const rollbackWeb = async () => {
  await ElMessageBox.confirm(
    `确定回退到 Web UI v${web.value?.previous_version || "上一版"} 吗？`,
    "回退 Web UI",
    { type: "warning", confirmButtonText: "确认回退", cancelButtonText: "取消" },
  );
  rollingBackWeb.value = true;
  try {
    await rollbackWebRelease();
    window.location.reload();
  } catch (error) {
    ElMessage.error(errorMessage(error, "Web UI 回退失败"));
    rollingBackWeb.value = false;
  }
};

const uploadWeb = async (event: Event) => {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  try {
    const form = createUploadFormData([["file", file]]);
    const result = await runWebUpload(
      (options) => uploadWebRelease(form, true, options),
      { initialTotalBytes: file.size },
    );
    if (result.cancelled) {
      ElMessage.info("Web UI 包上传已取消");
      return;
    }
    window.location.reload();
  } catch (error) {
    ElMessage.error(errorMessage(error, "Web UI 包安装失败"));
  } finally {
    input.value = "";
  }
};

const checkOnlineWebUpdate = async () => {
  checkingWebUpdate.value = true;
  try {
    const response = await checkWebUpdate();
    if (response.code === 404) {
      webUpdateInfo.value = undefined;
      ElMessage.info(response.message || "当前发布没有独立 Web UI 包");
      return;
    }
    webUpdateInfo.value = response.data;
    if (!response.data.update_available) {
      ElMessage.success("Web UI 已是当前通道的最新版本");
    }
  } catch (error) {
    ElMessage.error(errorMessage(error, "Web UI 更新检查失败"));
  } finally {
    checkingWebUpdate.value = false;
  }
};

const installOnlineWebUpdate = async () => {
  if (!webUpdateInfo.value) return;
  await ElMessageBox.confirm(
    `下载并激活 Web UI v${webUpdateInfo.value.version} 吗？当前 UI 会保留为可回退版本。`,
    "更新 Web UI",
    { type: "warning", confirmButtonText: "下载并激活", cancelButtonText: "取消" },
  );
  installingWebUpdate.value = true;
  try {
    await installWebUpdate();
    window.location.reload();
  } catch (error) {
    ElMessage.error(errorMessage(error, "Web UI 在线更新失败"));
  } finally {
    installingWebUpdate.value = false;
  }
};

onMounted(loadReleases);
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
