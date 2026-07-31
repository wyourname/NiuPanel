<template>
  <component
    :is="appStore.isMobile ? 'el-drawer' : 'el-dialog'"
    v-model="visible"
    :title="appStore.isMobile ? '' : dialogTitle"
    :size="appStore.isMobile ? '100%' : undefined"
    :width="appStore.isMobile ? '100%' : '920px'"
    :align-center="!appStore.isMobile"
    direction="btt"
    destroy-on-close
    append-to-body
    class="log-modal"
  >
    <div class="flex-1 flex flex-col overflow-hidden bg-[var(--editor-bg)]">
      <div class="px-5 py-4 border-b border-[var(--editor-border)] flex items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="text-sm font-semibold text-[var(--editor-text)] truncate">
            {{ dialogTitle }}
          </div>
          <div class="text-xs text-[var(--editor-text)]/55 mt-1">
            支持批量安装、单个卸载和任务日志回看。Node.js 为全局依赖管理，Python 走当前环境。
          </div>
        </div>

        <div class="flex items-center gap-2 shrink-0">
          <ToolbarButton variant="soft" @click="showInstallDialog">
            <template #icon>
              <div class="i-ep-download"></div>
            </template>
            安装依赖
          </ToolbarButton>
          <ToolbarButton @click="showUninstallDialog">
            <template #icon>
              <div class="i-ep-delete"></div>
            </template>
            手动卸载
          </ToolbarButton>
          <button
            type="button"
            class="btn-icon !w-9 !h-9"
            @click="loadPackages()"
          >
            <div :class="['i-ep-refresh', loading ? 'animate-spin' : '']"></div>
          </button>
        </div>
      </div>

      <div class="px-5 py-4 border-b border-[var(--editor-border)]">
        <div class="grid grid-cols-1 md:grid-cols-[minmax(0,1fr)_220px] gap-3">
          <el-input
            v-model="searchQuery"
            size="small"
            clearable
            placeholder="搜索已安装依赖"
            class="modern-input"
          >
            <template #prefix>
              <div class="i-ep-search"></div>
            </template>
          </el-input>

          <div class="rounded-md border border-[var(--editor-border)] bg-black/5 px-4 py-3 dark:bg-white/5">
            <div class="text-[11px] text-[var(--editor-text)]/45">已安装数量</div>
            <div class="text-2xl font-semibold text-[var(--editor-text)] mt-1">{{ filteredPackages.length }}</div>
          </div>
        </div>
      </div>

      <EnvPackageList
        :loading="loading"
        :packages="filteredPackages"
        :search-query="searchQuery"
        @uninstall="handleUninstallPackage"
      />
    </div>

    <EnvPackageInstallDialog
      v-model:package-text="installForm.packages"
      v-model:visible="installDialogVisible"
      :installing="installing"
      @install="handleInstallPackages"
    />

    <EnvPackageUninstallDialog
      v-model:package-name="uninstallForm.package"
      v-model:visible="uninstallDialogVisible"
      @uninstall="handleManualUninstall"
    />
  </component>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../../../../stores/app";
import type { Env } from "@/types";
import ToolbarButton from "../../../../components/common/ToolbarButton.vue";
import EnvPackageInstallDialog from "./EnvPackageInstallDialog.vue";
import EnvPackageList from "./EnvPackageList.vue";
import EnvPackageUninstallDialog from "./EnvPackageUninstallDialog.vue";
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
const appStore = useAppStore();

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit("update:modelValue", value),
});

const {
  dialogTitle,
  filteredPackages,
  handleInstallPackages,
  handleManualUninstall,
  handleUninstallPackage,
  installDialogVisible,
  installForm,
  installing,
  loadPackages,
  loading,
  searchQuery,
  showInstallDialog,
  showUninstallDialog,
  uninstallDialogVisible,
  uninstallForm,
} = useEnvPackageManager({
  env: () => props.env,
  isVisible: visible,
  onShowLog: (id, name) => emit("show-log", id, name),
});
</script>
