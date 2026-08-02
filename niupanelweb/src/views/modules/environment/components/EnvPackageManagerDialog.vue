<template>
  <ResponsiveDialog
    v-model:visible="visible"
    :title="dialogTitle"
    desktop-size="xl"
    content-preset="workspace"
    size="100%"
    destroy-on-close
    append-to-body
  >
    <div
      class="dependency-manager-shell flex h-full min-h-[420px] flex-col overflow-hidden bg-[var(--editor-bg)] md:h-[min(680px,75vh)]"
    >
      <section
        class="min-h-0 flex flex-1 flex-col overflow-hidden bg-card"
        aria-labelledby="dependency-manager-list-title"
      >
        <div class="dependency-manager-toolbar shrink-0 p-3">
          <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div class="min-w-0">
              <div class="flex items-center gap-2">
                <h3
                  id="dependency-manager-list-title"
                  class="text-[12px] font-semibold text-default"
                >
                  已安装依赖
                </h3>
                <span class="rounded-md bg-soft px-2 py-0.5 text-[10px] font-semibold text-secondary">
                  {{ filteredPackages.length }}
                </span>
              </div>
              <p
                v-if="packageStatusLabel"
                class="mt-1 text-[10px] text-muted"
                aria-live="polite"
              >
                {{ packageStatusLabel }}
              </p>
            </div>

            <div class="flex w-full items-center gap-2 sm:w-auto">
              <button
                type="button"
                class="dependency-manager-refresh !h-11 !w-11 shrink-0 rounded-lg bg-soft text-secondary flex-center transition-colors hover:text-default disabled:cursor-not-allowed disabled:opacity-50"
                title="刷新依赖列表"
                aria-label="刷新依赖列表"
                :disabled="loading"
                @click="loadPackages()"
              >
                <span :class="['i-ep-refresh', loading ? 'animate-spin' : '']"></span>
              </button>
              <ToolbarButton
                variant="primary"
                class="!min-h-11 flex-1 sm:flex-none"
                @click="showInstallDialog"
              >
                <template #icon>
                  <span class="i-ep-plus"></span>
                </template>
                安装依赖
              </ToolbarButton>
            </div>
          </div>

          <el-input
            v-model="searchQuery"
            clearable
            placeholder="搜索包名或版本"
            class="dependency-manager-search modern-input mt-3 w-full"
          >
            <template #prefix>
              <span class="i-ep-search"></span>
            </template>
          </el-input>
        </div>

        <EnvPackageList
          :loading="loading"
          :packages="filteredPackages"
          :search-query="searchQuery"
          :uninstalling-package="uninstallingPackage"
          @clear-search="searchQuery = ''"
          @install="showInstallDialog"
          @uninstall="handleUninstallPackage"
        />
      </section>
    </div>

    <EnvPackageInstallDialog
      v-model:package-text="installForm.packages"
      v-model:visible="installDialogVisible"
      :installing="installing"
      @install="handleInstallPackages"
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
  handleUninstallPackage,
  installDialogVisible,
  installForm,
  installing,
  loadPackages,
  loading,
  packages,
  searchQuery,
  showInstallDialog,
  uninstallingPackage,
} = useEnvPackageManager({
  env: () => props.env,
  isVisible: visible,
  onShowLog: (id, name) => emit("show-log", id, name),
});

const packageStatusLabel = computed(() => {
  if (loading.value) return "正在更新依赖列表…";
  if (searchQuery.value) {
    return `找到 ${filteredPackages.value.length} 项，共 ${packages.value.length} 项`;
  }
  return "";
});
</script>

<style scoped>
.dependency-manager-search :deep(.el-input__wrapper) {
  min-height: 44px;
}
</style>
