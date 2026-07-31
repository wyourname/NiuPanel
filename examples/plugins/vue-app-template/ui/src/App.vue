<template>
  <section class="h-full min-h-0 overflow-auto bg-base p-5 text-default">
    <div class="mx-auto flex max-w-4xl flex-col gap-4">
      <header class="flex flex-wrap items-center justify-between gap-3 border-b border-light pb-4">
        <div>
          <h1 class="text-lg font-bold">{{ context.app.name }}</h1>
          <p class="mt-1 text-xs text-muted">{{ context.app.description }}</p>
        </div>
        <button
          type="button"
          class="rounded-lg bg-primary px-3 py-2 text-xs font-bold text-white"
          @click="loadApps"
        >
          调用面板 API
        </button>
      </header>

      <div class="rounded-lg border border-light bg-card p-4">
        <div class="text-xs font-bold uppercase tracking-wide text-muted">
          Plugin Context
        </div>
        <pre class="mt-3 overflow-auto rounded-lg bg-soft p-3 text-xs">{{ contextPreview }}</pre>
      </div>

      <div v-if="apiResult" class="rounded-lg border border-light bg-card p-4">
        <div class="text-xs font-bold uppercase tracking-wide text-muted">
          API Result
        </div>
        <pre class="mt-3 overflow-auto rounded-lg bg-soft p-3 text-xs">{{ apiResult }}</pre>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import type { NiuPanelPluginContext } from "@niupanel/plugin-sdk";

const props = defineProps<{
  context: NiuPanelPluginContext;
}>();

const apiResult = ref("");

const contextPreview = computed(() =>
  JSON.stringify(
    {
      pluginId: props.context.pluginId,
      route: props.context.route,
      capabilities: props.context.app.capabilities,
      permissions: props.context.app.ui.permissions,
    },
    null,
    2,
  ),
);

const loadApps = async () => {
  try {
    const result = await props.context.api.request({
      path: "/plugins/apps",
    });
    apiResult.value = JSON.stringify(result, null, 2);
    props.context.ui.toast("插件 API 调用完成", "success");
  } catch (error) {
    apiResult.value = String(error instanceof Error ? error.message : error);
    props.context.ui.toast("插件 API 调用失败", "error");
  }
};
</script>
