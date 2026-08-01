<template>
  <ResponsiveDialog
    v-model:visible="visible"
    :title="dialogTitle"
    width="920px"
    size="100%"
    destroy-on-close
    append-to-body
  >
    <div class="flex h-full min-h-[420px] flex-col overflow-hidden bg-[var(--editor-bg)] md:h-[min(680px,75vh)]">
      <div class="flex flex-col gap-3 border-b border-[var(--editor-border)] px-5 py-3.5 sm:flex-row sm:items-center sm:justify-between">
        <div class="flex min-w-0 items-center gap-3">
          <span class="h-9 w-9 shrink-0 rounded-md accent-subtle flex-center">
            <span class="i-ep-box text-[16px]"></span>
          </span>
          <div class="min-w-0">
            <div class="text-[12px] font-semibold text-[var(--editor-text)]">
              管理已安装依赖
            </div>
            <div class="mt-0.5 text-[10px] leading-4 text-[var(--editor-text)]/55">
              Node.js 依赖按运行时版本隔离；Python 依赖归属当前虚拟环境。
            </div>
          </div>
        </div>

        <div class="flex items-center gap-2 sm:shrink-0 sm:self-end">
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
            title="刷新依赖列表"
            aria-label="刷新依赖列表"
            @click="loadPackages()"
          >
            <div :class="['i-ep-refresh', loading ? 'animate-spin' : '']"></div>
          </button>
        </div>
      </div>

      <div class="border-b border-[var(--editor-border)] px-5 py-4">
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
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Env } from "@/types";
import ResponsiveDialog from "../../../../components/common/ResponsiveDialog.vue";
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
