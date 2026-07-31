<template>
  <el-tabs v-model="activeTabValue" class="mirror-tabs">
    <el-tab-pane label="UV 全局设置" name="uv">
      <el-form label-position="top" size="small">
        <el-alert
          title="这些设置将影响系统通过 uv 下载 Python 解释器和安装依赖的速度。"
          type="info"
          :closable="false"
          class="mb-4"
        />

        <el-form-item label="Python 解释器下载镜像 (UV_PYTHON_INSTALL_MIRROR)">
          <EnvMirrorPresetSelect
            :is-mobile="isMobile"
            :model-value="uvPythonMirror"
            :presets="uvPythonMirrors"
            @update:model-value="emit('update:uvPythonMirror', $event)"
          />
          <el-input
            :model-value="uvPythonMirror"
            placeholder="自定义地址 https://..."
            @update:model-value="emit('update:uvPythonMirror', String($event))"
          />
        </el-form-item>

        <el-form-item label="PyPI 包下载镜像 (UV_INDEX_URL)">
          <EnvMirrorPresetSelect
            :is-mobile="isMobile"
            :model-value="uvPypiMirror"
            :presets="pythonMirrors"
            @update:model-value="emit('update:uvPypiMirror', $event)"
          />
          <el-input
            :model-value="uvPypiMirror"
            placeholder="自定义地址 https://..."
            @update:model-value="emit('update:uvPypiMirror', String($event))"
          />
        </el-form-item>
      </el-form>
    </el-tab-pane>

    <el-tab-pane label="Pip 传统配置" name="pip">
      <el-form label-position="top">
        <el-alert
          title="这是通过执行 `pip config set` 设置的镜像，仅对系统 pip 有效。"
          type="warning"
          :closable="false"
          class="mb-4"
        />
        <el-form-item label="常用源">
          <EnvMirrorPresetSelect
            :is-mobile="isMobile"
            :model-value="legacyUrl"
            :presets="pipMirrors"
            class="!mb-0"
            @update:model-value="emit('update:legacyUrl', $event)"
          />
        </el-form-item>
        <el-form-item label="自定义地址">
          <el-input
            :model-value="legacyUrl"
            placeholder="https://..."
            @update:model-value="emit('update:legacyUrl', String($event))"
          />
        </el-form-item>
      </el-form>
    </el-tab-pane>
  </el-tabs>
</template>

<script setup lang="ts">
import { computed } from "vue";
import EnvMirrorPresetSelect from "./EnvMirrorPresetSelect.vue";
import type { MirrorPreset } from "./envMirrorTypes";

const props = defineProps<{
  activeTab: string;
  isMobile: boolean;
  legacyUrl: string;
  pipMirrors: readonly MirrorPreset[];
  pythonMirrors: readonly MirrorPreset[];
  uvPypiMirror: string;
  uvPythonMirror: string;
  uvPythonMirrors: readonly MirrorPreset[];
}>();

const emit = defineEmits<{
  (event: "update:activeTab", value: string): void;
  (event: "update:legacyUrl", value: string): void;
  (event: "update:uvPypiMirror", value: string): void;
  (event: "update:uvPythonMirror", value: string): void;
}>();

const activeTabValue = computed({
  get: () => props.activeTab,
  set: (value) => emit("update:activeTab", value),
});
</script>
