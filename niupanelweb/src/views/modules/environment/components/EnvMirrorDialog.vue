<template>
  <ResponsiveDialog
    v-model:visible="visible"
    :title="title"
    desktop-size="md"
    content-preset="form"
    :show-close="!submitting"
    :close-on-click-modal="false"
    :close-on-press-escape="!submitting"
    append-to-body
  >
    <EnvMirrorShellNotice v-if="filterType === 'sh'" @close="visible = false" />
    <div v-else class="flex min-h-60 flex-col gap-4" v-loading="loading">
      <div v-if="error" role="alert" class="rounded-md border border-light p-4 text-sm text-secondary">
        <p>{{ error }}</p>
        <el-button @click="loadCurrentSettings">重新加载</el-button>
      </div>
      <el-form v-else label-position="top" :disabled="loading || submitting || !settings">
        <p class="mb-4 text-xs leading-5 text-secondary">保存后用于当前面板后续的环境创建和依赖安装，已安装的包不受影响。</p>
        <el-form-item :label="filterType === 'python' ? 'Python 包下载源（PyPI）' : 'Node.js 包下载源（npm Registry）'">
          <div class="w-full">
            <EnvMirrorPresetSelect :is-mobile="appStore.isMobile" v-model="form.package_url" :presets="packagePresets" :disabled="Boolean(settings?.package_override)" />
            <el-input v-model="form.package_url" aria-label="包下载源地址" placeholder="https://..." :disabled="Boolean(settings?.package_override)" />
            <p v-if="settings?.package_override" class="mt-2 break-all text-xs text-secondary">由 Docker 环境变量 {{ settings.package_override }} 指定。修改该变量并重新创建容器后生效。</p>
          </div>
        </el-form-item>
        <el-form-item :label="filterType === 'python' ? 'Python 解释器下载源' : 'Node.js 运行时下载源'">
          <div class="w-full">
            <EnvMirrorPresetSelect :is-mobile="appStore.isMobile" v-model="form.runtime_url" :presets="runtimePresets" :disabled="Boolean(settings?.runtime_override)" />
            <el-input v-model="form.runtime_url" aria-label="运行时下载源地址" placeholder="https://..." :disabled="Boolean(settings?.runtime_override)" />
            <p v-if="settings?.runtime_override" class="mt-2 break-all text-xs text-secondary">由 Docker 环境变量 {{ settings.runtime_override }} 指定。修改该变量并重新创建容器后生效。</p>
          </div>
        </el-form-item>
        <p v-if="settings && validationError" role="status" class="text-xs text-rose-600 dark:text-rose-400">{{ validationError }}</p>
      </el-form>
    </div>
    <template v-if="filterType !== 'sh'" #footer>
      <el-button :disabled="submitting" @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="submitting" :disabled="loading || !settings || Boolean(validationError)" @click="handleSubmit">保存全部</el-button>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { useAppStore } from "@/stores/app";
import { useMobileBackCloseAction } from "@/composables/useMobileBackCloseAction";
import { computed, toRef, watch } from "vue";
import { NODE_MIRRORS, NODE_DIST_MIRRORS, PYTHON_MIRRORS, UV_PYTHON_MIRRORS } from "@/constants/mirrors";
import { useEnvMirrorSettings } from "../composables/useEnvMirrorSettings";
import type { EnvType } from "@/types";
import EnvMirrorShellNotice from "./EnvMirrorShellNotice.vue";
import EnvMirrorPresetSelect from "./EnvMirrorPresetSelect.vue";
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";

const props = defineProps<{ modelValue?: boolean; filterType: EnvType }>();
const emit = defineEmits<{ (event: "update:modelValue", value: boolean): void }>();
const appStore = useAppStore();
const visible = computed({ get: () => Boolean(props.modelValue), set: value => { if (!submitting.value) emit("update:modelValue", value); } });
const { form, title, settings, error, loading, submitting, validationError, loadCurrentSettings, handleSubmit, cancelLoad } = useEnvMirrorSettings({ filterType: toRef(props, "filterType"), onClose: () => emit("update:modelValue", false) });
const packagePresets = computed(() => props.filterType === "node" ? NODE_MIRRORS : PYTHON_MIRRORS);
const runtimePresets = computed(() => props.filterType === "node" ? NODE_DIST_MIRRORS : UV_PYTHON_MIRRORS);
watch(() => [props.modelValue, props.filterType], () => {
  if (props.modelValue) void loadCurrentSettings();
  else cancelLoad();
}, { immediate: true });
useMobileBackCloseAction({ appStore: useAppStore(), visible, close: () => { if (submitting.value) return false; visible.value = false; } });
</script>
