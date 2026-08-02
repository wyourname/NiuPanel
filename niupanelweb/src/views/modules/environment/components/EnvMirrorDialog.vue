<template>
  <ResponsiveDialog
    v-model:visible="visible"
    :title="`设置 ${getMirrorTitle()} 镜像`"
    desktop-size="md"
    content-preset="form"
    append-to-body
  >
    <div class="flex min-h-60 flex-col gap-4" v-loading="loading">
        <EnvMirrorShellNotice
          v-if="filterType === 'sh'"
          @close="visible = false"
        />

        <EnvPythonMirrorTabs
          v-else-if="filterType === 'python'"
          v-model:active-tab="activeTab"
          v-model:legacy-url="form.url"
          v-model:uv-pypi-mirror="uvForm.pypiMirror"
          v-model:uv-python-mirror="uvForm.pythonMirror"
          :is-mobile="appStore.isMobile"
          :pip-mirrors="currentMirrorPresets"
          :python-mirrors="PYTHON_MIRRORS"
          :uv-python-mirrors="UV_PYTHON_MIRRORS"
        />

        <EnvNodeMirrorTabs
          v-else-if="filterType === 'node'"
          v-model:active-tab="activeTab"
          v-model:dist-mirror="nodeForm.distMirror"
          v-model:registry-url="nodeForm.registryMirror"
          :is-mobile="appStore.isMobile"
          :node-registry-mirrors="NODE_MIRRORS"
        />

    </div>

    <template v-if="filterType !== 'sh'" #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit">
        确定
      </el-button>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { ref, toRef, watch } from "vue";
import { useAppStore } from "../../../../stores/app";
import {
  NODE_MIRRORS,
  PYTHON_MIRRORS,
  UV_PYTHON_MIRRORS,
} from "../../../../constants/mirrors";
import { useEnvMirrorSettings } from "../composables/useEnvMirrorSettings";
import type { EnvType } from "@/types";
import EnvMirrorShellNotice from "./EnvMirrorShellNotice.vue";
import EnvNodeMirrorTabs from "./EnvNodeMirrorTabs.vue";
import EnvPythonMirrorTabs from "./EnvPythonMirrorTabs.vue";
import ResponsiveDialog from "../../../../components/common/ResponsiveDialog.vue";

const props = withDefaults(
  defineProps<{
    modelValue?: boolean;
    filterType: EnvType;
  }>(),
  {
    modelValue: false,
  },
);

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
}>();
const appStore = useAppStore();

const visible = ref(false);

const {
  activeTab,
  currentMirrorPresets,
  form,
  getMirrorTitle,
  handleSubmit,
  loadCurrentSettings,
  loading,
  nodeForm,
  resetForm,
  submitting,
  uvForm,
} = useEnvMirrorSettings({
  filterType: toRef(props, "filterType"),
  onClose: () => {
    visible.value = false;
  },
});

watch(
  () => props.modelValue,
  (val: boolean) => {
    visible.value = val;
    if (val) {
      resetForm();
      void loadCurrentSettings();
    }
  },
);

watch(visible, (val: boolean) => emit("update:modelValue", val));
</script>

<style scoped>
.mirror-tabs :deep(.el-tabs__header) {
  margin-bottom: 15px;
}
</style>

<!-- Global style for teleported poppers -->
<style>
/* 移动端专用修复 */
@media (max-width: 767px) {
  .mirror-select-popper {
    max-width: calc(100vw - 32px) !important;
    left: 16px !important;
  }

  .mirror-select-popper .el-select-dropdown__item {
    height: auto !important;
    line-height: 1.4 !important;
    padding: 10px 12px !important;
    white-space: normal !important;
    word-break: break-all !important;
    display: flex !important;
    align-items: center !important;
  }
}

/* PC 端通用优化：限制最大宽度但不强制定位 */
@media (min-width: 769px) {
  .mirror-select-popper {
    max-width: 500px !important;
  }

  .mirror-select-popper .el-select-dropdown__item {
    display: flex !important;
    align-items: center !important;
    justify-content: space-between !important;
  }
}

.mirror-select-popper .el-select-dropdown__list {
  padding: 4px 0 !important;
}
</style>
