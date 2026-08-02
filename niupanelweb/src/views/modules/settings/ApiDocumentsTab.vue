<template>
  <div class="flex h-full min-h-[420px] flex-col overflow-hidden rounded-md border border-light bg-card">
    <div class="flex shrink-0 items-center justify-between gap-2 border-b border-light/50 px-3 py-2.5 md:px-4 md:py-3">
      <div class="flex min-w-0 items-center gap-2 md:gap-3">
        <div class="i-ep-document text-primary text-xl"></div>
        <span class="truncate text-[13px] font-bold text-default md:text-sm">开发者文档（Scalar）</span>
      </div>
      <div class="flex items-center gap-2">
        <el-button
          type="primary"
          size="small"
          plain
          @click="openExternal"
        >
          <template #icon><div class="i-ep-full-screen"></div></template>
          独立查看
        </el-button>
      </div>
    </div>
    <div class="flex-1 relative bg-white">
      <iframe
        ref="iframeRef"
        :srcdoc="scalarHtml"
        class="w-full h-full border-none"
        allow="clipboard-write"
      ></iframe>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useAppStore } from '@/stores/app';

const appStore = useAppStore();

// 构造 OpenAPI JSON 的绝对路径，确保 iframe 能够跨上下文拉取
const getOpenApiUrl = () => {
  const base = appStore.serverUrl ? appStore.serverUrl.replace(/\/$/, '') : window.location.origin;
  return `${base}/api/v1/documents/openapi.json`;
};

const scalarHtml = computed(() => {
  const url = getOpenApiUrl();
  return `
<!doctype html>
<html>
  <head>
    <title>NiuPanel API Documentation</title>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <style>
      html, body { height: 100%; }
      body { margin: 0; }
      /* 隐藏 Scalar 默认的一些边距，使其更像内嵌组件 */
      .scalar-api-reference { min-height: 100%; height: 100%; }
    </style>
  </head>
  <body>
    <script id="api-reference" data-url="${url}"><\/script>
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"><\/script>
  </body>
</html>
  `;
});

const openExternal = () => {
  const url = getOpenApiUrl();
  // 使用 Scalar 官方的在线预览工具打开
  window.open(`https://api.scalar.com/v1/docs?url=${encodeURIComponent(url)}`, '_blank');
};
</script>
