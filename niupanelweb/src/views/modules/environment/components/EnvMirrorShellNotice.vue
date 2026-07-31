<template>
  <div
    class="flex flex-col items-center justify-center gap-4 py-8 text-center"
  >
    <div
      class="flex h-14 w-14 items-center justify-center rounded-md bg-orange-500/10"
    >
      <div class="i-ep-warning text-3xl text-orange-500"></div>
    </div>
    <div>
      <div class="text-sm font-bold text-default mb-1">
        Linux Shell 暂不支持换源
      </div>
      <div class="text-xs text-muted leading-relaxed max-w-xs">
        Linux 系统依赖（apt 包管理）的镜像配置需要修改
        <code class="bg-base px-1 rounded">/etc/apt/sources.list</code>，
        目前面板不支持自动修改系统 apt 源。<br /><br />
        请在容器中手动修改，或在 Dockerfile 中预先配置好镜像源。
      </div>
    </div>
    <el-button @click="emit('close')">关闭</el-button>
  </div>
</template>

<script setup lang="ts">
const emit = defineEmits<{
  (event: "close"): void;
}>();
</script>
