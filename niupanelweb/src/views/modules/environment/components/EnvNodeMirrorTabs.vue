<template>
  <el-tabs v-model="activeTabValue" class="mirror-tabs">
    <el-tab-pane label="Node.js 下载源" name="node_dist">
      <el-form label-position="top" size="small">
        <el-alert
          title="配置 pnpm runtime 下载 Node.js 二进制时使用的镜像源（PNPM_NODE_DIST_MIRROR），在创建环境时生效。"
          type="info"
          :closable="false"
          class="mb-4"
        />
        <el-form-item label="Node.js 发行版镜像 (PNPM_NODE_DIST_MIRROR)">
          <EnvMirrorPresetSelect
            :is-mobile="isMobile"
            :model-value="distMirror"
            :presets="nodeDistMirrors"
            @update:model-value="emit('update:distMirror', $event)"
          />
          <el-input
            :model-value="distMirror"
            placeholder="自定义地址 https://..."
            @update:model-value="emit('update:distMirror', String($event))"
          />
        </el-form-item>
      </el-form>
    </el-tab-pane>

    <el-tab-pane label="pnpm 包镜像" name="pnpm">
      <el-form label-position="top">
        <el-alert
          title="作为 pnpm Registry 使用，影响 pnpm add 时包的下载速度。"
          type="info"
          :closable="false"
          class="mb-4"
        />
        <el-form-item label="常用 pnpm Registry">
          <EnvMirrorPresetSelect
            :is-mobile="isMobile"
            :model-value="registryUrl"
            :presets="nodeRegistryMirrors"
            @update:model-value="emit('update:registryUrl', $event)"
          />
          <el-input
            :model-value="registryUrl"
            placeholder="自定义地址 https://..."
            @update:model-value="emit('update:registryUrl', String($event))"
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

const nodeDistMirrors = [
  { name: "官方源", url: "https://nodejs.org/dist/" },
  { name: "中科大镜像（推荐）", url: "https://mirrors.ustc.edu.cn/node/" },
  {
    name: "清华源",
    url: "https://mirrors.tuna.tsinghua.edu.cn/nodejs-release/",
  },
  { name: "阿里源", url: "https://mirrors.aliyun.com/nodejs-release/" },
] as const satisfies readonly MirrorPreset[];

const props = defineProps<{
  activeTab: string;
  distMirror: string;
  isMobile: boolean;
  nodeRegistryMirrors: readonly MirrorPreset[];
  registryUrl: string;
}>();

const emit = defineEmits<{
  (event: "update:activeTab", value: string): void;
  (event: "update:distMirror", value: string): void;
  (event: "update:registryUrl", value: string): void;
}>();

const activeTabValue = computed({
  get: () => props.activeTab,
  set: (value) => emit("update:activeTab", value),
});
</script>
