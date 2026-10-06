<template>
  <ResponsiveDialog
    v-model:visible="visible"
    :title="dialogTitle"
    desktop-size="xl"
    content-preset="workspace"
    size="100%"
    destroy-on-close
    :close-on-click-modal="false"
    append-to-body
  >
    <div class="dependency-manager-shell custom-scrollbar">
      <div v-if="restoringJobs || jobRestoreError" class="flex shrink-0 items-center gap-2 border-b border-light px-4 py-2 text-xs text-secondary" role="status">
        <span class="flex-1">{{ jobRestoreError || '正在同步安装状态…' }}</span>
        <button v-if="jobRestoreError" class="min-h-11 px-2 text-primary" type="button" @click="restoreJobs">重试</button>
      </div>
      <section
        class="dependency-manager-library"
        aria-labelledby="dependency-manager-list-title"
      >
        <div class="dependency-manager-toolbar">
          <div class="flex min-h-11 shrink-0 flex-wrap items-center gap-x-3 gap-y-1">
            <h3 id="dependency-manager-list-title" class="m-0 text-sm font-semibold text-default">
              已安装依赖
            </h3>
            <span class="text-xs tabular-nums text-secondary" role="status">{{ packageStatusLabel }}</span>
          </div>
          <div class="dependency-manager-install-actions">
            <EnvPackageInstallForm
              ref="installFormRef"
              v-model:package-text="installForm.packages"
              :env="env"
              :busy="busy"
              :installing="installing"
              :package-count="parsedInput.packages.length"
              :validation-error="parsedInput.error"
              @install="handleInstallPackages"
            />
            <ToolbarButton
              class="dependency-manager-refresh !h-11 !w-11 !p-0"
              variant="soft"
              title="刷新依赖列表"
              aria-label="刷新依赖列表"
              :disabled="loading"
              @click="loadPackages()"
            >
              <span :class="['i-ep-refresh', loading ? 'animate-spin' : '']" aria-hidden="true"></span>
            </ToolbarButton>
          </div>
        </div>

        <section v-if="currentJob" class="dependency-manager-job" aria-label="最近的依赖操作">
          <div class="flex min-w-0 flex-1 flex-wrap items-center gap-x-3 gap-y-1" role="status">
            <span class="flex items-center gap-1.5 text-xs font-medium" :class="jobAppearance.color">
              <span :class="jobAppearance.icon" class="shrink-0" aria-hidden="true"></span>
              {{ jobStatusLabel }}
            </span>
            <span class="min-w-0 truncate text-xs text-secondary" :title="currentJob.name">{{ currentJob.name }}</span>
          </div>
          <ToolbarButton class="!min-h-11 shrink-0 !px-2" @click="showJobLog">查看日志</ToolbarButton>
        </section>

        <div v-if="loadError" class="dependency-manager-load-error" role="alert">
          <span class="i-ep-warning text-2xl text-secondary" aria-hidden="true"></span>
          <p class="text-sm text-secondary">{{ loadError }}</p>
          <ToolbarButton variant="soft" class="!min-h-11" @click="loadPackages()">重试</ToolbarButton>
        </div>
        <EnvPackageList
          v-else
          :loading="loading"
          :busy="busy"
          :packages="packages"
          :uninstalling-package="uninstallingPackage"
          @install="installFormRef?.focus()"
          @uninstall="handleUninstallPackage"
        />
      </section>
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { useMobileBackCloseAction } from "@/composables/useMobileBackCloseAction";
import { useAppStore } from "@/stores/app";
import { computed, ref } from "vue";
import type { Env } from "@/types";
import ResponsiveDialog from "../../../../components/common/ResponsiveDialog.vue";
import ToolbarButton from "@/components/common/ToolbarButton.vue";
import EnvPackageInstallForm from "./EnvPackageInstallForm.vue";
import EnvPackageList from "./EnvPackageList.vue";
import { useEnvPackageManager } from "../composables/useEnvPackageManager";

const props = withDefaults(
  defineProps<{
    modelValue?: boolean;
    env?: Env | null;
  }>(),
  {
    modelValue: false,
    env: null,
  },
);

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
  (event: "show-log", id: number | string, name: string): void;
}>();

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit("update:modelValue", value),
});

const {
  busy,
  currentJob,
  jobStatusLabel,
  showJobLog,
  parsedInput,
  loadError,
  restoringJobs,
  jobRestoreError,
  restoreJobs,
  dialogTitle,
  handleInstallPackages,
  handleUninstallPackage,
  installForm,
  installing,
  loadPackages,
  loading,
  packages,
  uninstallingPackage,
} = useEnvPackageManager({
  env: () => props.env,
  isVisible: visible,
  onShowLog: (id, name) => emit("show-log", id, name),
});

const installFormRef = ref<InstanceType<typeof EnvPackageInstallForm>>();

const packageStatusLabel = computed(() => {
  if (loading.value) return "正在更新依赖列表…";
  if (loadError.value) return "暂时无法获取";
  return `共 ${packages.value.length} 项`;
});

const jobAppearance = computed(() => {
  const job = currentJob.value;
  if (job?.pollError || job?.status === "Failed") {
    return { icon: "i-ep-warning", color: "text-rose-600 dark:text-rose-400" };
  }
  if (job?.status === "Pending" || job?.status === "Running") {
    return { icon: "i-ep-loading animate-spin motion-reduce:animate-none", color: "text-primary" };
  }
  if (job?.status === "Success" || job?.status === "Finished") {
    return { icon: "i-ep-circle-check", color: "text-emerald-700 dark:text-emerald-400" };
  }
  return { icon: "i-ep-info-filled", color: "text-secondary" };
});
useMobileBackCloseAction({ appStore: useAppStore(), visible, close: () => { visible.value = false; } });
</script>

<style scoped>
.dependency-manager-shell {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  overflow-y: auto;
  background: var(--bg-card);
}

.dependency-manager-job {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  gap: 12px;
  padding: 4px 20px;
  border-bottom: 1px solid var(--border-light);
  background: var(--bg-subtle);
}

.dependency-manager-library {
  display: flex;
  min-width: 0;
  flex: 1 0 auto;
  flex-direction: column;
  padding-bottom: env(safe-area-inset-bottom, 0px);
}

.dependency-manager-toolbar {
  display: flex;
  flex-shrink: 0;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 20px;
  border-bottom: 1px solid var(--border-light);
}

.dependency-manager-install-actions {
  display: flex;
  min-width: 0;
  width: 100%;
  align-items: flex-start;
  gap: 8px;
}

.dependency-manager-load-error {
  display: flex;
  min-height: 180px;
  flex: 1;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 24px;
  text-align: center;
}

@media (min-width: 769px) {
  .dependency-manager-shell {
    max-height: calc(var(--app-viewport-height) - 100px);
    overflow: hidden;
  }

  .dependency-manager-install-actions {
    width: auto;
    max-width: 680px;
    flex: 1;
  }

  .dependency-manager-library {
    min-height: 0;
    flex: 0 1 auto;
  }

  .dependency-manager-load-error {
    min-height: 0;
    overflow-y: auto;
  }
}
</style>
