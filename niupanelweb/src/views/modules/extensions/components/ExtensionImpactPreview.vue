<template>
  <article class="max-h-[58vh] w-full space-y-3 overflow-y-auto pr-1 text-left custom-scrollbar">
    <header class="rounded-lg border border-light bg-subtle/60 p-3">
      <div class="flex items-start justify-between gap-3">
        <div class="min-w-0">
          <h3 class="m-0 truncate text-[13px] font-bold text-default">
            {{ preview.name }}
          </h3>
          <code class="mt-1 block truncate font-mono text-[10px] text-muted">
            {{ preview.plugin_id }}
          </code>
        </div>
        <span
          class="shrink-0 rounded-md px-2 py-1 text-[10px] font-bold"
          :class="
            preview.install_allowed
              ? 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-300'
              : 'bg-rose-500/10 text-rose-700 dark:text-rose-300'
          "
        >
          {{ preview.install_allowed ? "可以继续" : "已阻止" }}
        </span>
      </div>

      <div class="mt-3 grid gap-2 sm:grid-cols-2">
        <div class="rounded-md border border-light bg-card px-2.5 py-2">
          <div class="text-[9px] font-semibold text-muted">版本变化</div>
          <div class="mt-1 flex items-center gap-1.5 font-mono text-[11px] font-bold text-default">
            <span v-if="preview.current_version">v{{ preview.current_version }}</span>
            <span v-if="preview.current_version" class="i-ep-right text-[10px] text-muted"></span>
            <span class="text-primary">v{{ preview.target_version }}</span>
          </div>
        </div>
        <div class="flex items-center gap-2 rounded-md border border-light bg-card px-2.5 py-2">
          <span
            v-for="feature in featureStates"
            :key="feature.label"
            class="flex flex-1 items-center gap-1.5 text-[10px] font-semibold"
            :class="feature.enabled ? 'text-secondary' : 'text-muted'"
          >
            <span
              class="h-1.5 w-1.5 rounded-full"
              :class="feature.enabled ? 'bg-emerald-500' : 'bg-muted'"
            ></span>
            {{ feature.label }}{{ feature.enabled ? "启用" : "未启用" }}
          </span>
        </div>
      </div>
    </header>

    <section v-if="preview.routes.length" class="overflow-hidden rounded-lg border border-light bg-card">
      <div class="flex items-center justify-between border-b border-light bg-subtle/45 px-3 py-2">
        <span class="flex items-center gap-1.5 text-[10px] font-bold text-secondary">
          <span class="i-ep-guide text-[12px] text-primary"></span>
          应用路由
        </span>
        <span class="text-[9px] text-muted">{{ preview.routes.length }} 项</span>
      </div>
      <div
        v-for="route in preview.routes"
        :key="route.path"
        class="flex items-start gap-2 border-b border-light/70 px-3 py-2 last:border-b-0"
      >
        <code class="min-w-0 flex-1 break-all font-mono text-[10px] leading-4 text-default">
          {{ route.path }}
        </code>
        <span class="max-w-[36%] shrink-0 truncate text-[9px] text-muted">
          {{ route.title || (route.hidden ? "隐藏路由" : "应用页面") }}
        </span>
      </div>
    </section>

    <section
      v-if="preview.route_conflicts.length"
      class="rounded-lg border border-rose-300/70 bg-rose-50/70 p-3 dark:border-rose-700/55 dark:bg-rose-950/15"
    >
      <div class="flex items-center gap-1.5 text-[10px] font-bold text-rose-700 dark:text-rose-300">
        <span class="i-ep-warning-filled text-[12px]"></span>
        路由冲突
      </div>
      <ul class="mt-2 space-y-1.5">
        <li
          v-for="conflict in preview.route_conflicts"
          :key="`${conflict.path}:${conflict.plugin_id}`"
          class="flex items-start gap-2 text-[10px] leading-4 text-rose-700 dark:text-rose-200"
        >
          <span class="mt-1.5 h-1 w-1 shrink-0 rounded-full bg-current"></span>
          <span>
            <code class="font-mono">{{ conflict.path }}</code>
            已被 {{ conflict.name || conflict.plugin_id }} 使用
          </span>
        </li>
      </ul>
    </section>

    <section v-if="changeGroups.length" class="space-y-2">
      <div
        v-for="group in changeGroups"
        :key="group.label"
        class="rounded-lg border border-light bg-card p-3"
      >
        <div class="flex items-center justify-between gap-2">
          <span class="text-[10px] font-bold text-secondary">{{ group.label }}</span>
          <span class="text-[9px] text-muted">{{ group.items.length }} 项</span>
        </div>
        <div class="mt-2 flex flex-wrap gap-1.5">
          <code
            v-for="item in group.items"
            :key="item"
            class="max-w-full break-all rounded-md px-2 py-1 font-mono text-[9px] leading-4"
            :class="
              group.added
                ? 'bg-amber-500/10 text-amber-700 dark:text-amber-300'
                : 'bg-soft text-secondary'
            "
          >
            {{ item }}
          </code>
        </div>
      </div>
    </section>

    <section
      v-if="preview.warnings.length"
      class="rounded-lg border border-amber-300/70 bg-amber-50/70 p-3 dark:border-amber-700/55 dark:bg-amber-950/15"
    >
      <div class="flex items-center gap-1.5 text-[10px] font-bold text-amber-800 dark:text-amber-300">
        <span class="i-ep-warning text-[12px]"></span>
        注意事项
      </div>
      <ul class="mt-2 space-y-1.5">
        <li
          v-for="warning in preview.warnings"
          :key="warning"
          class="flex items-start gap-2 text-[10px] leading-4 text-amber-800 dark:text-amber-200"
        >
          <span class="mt-1.5 h-1 w-1 shrink-0 rounded-full bg-current"></span>
          <span>{{ warning }}</span>
        </li>
      </ul>
    </section>

    <section
      v-if="preview.blockers.length"
      class="rounded-lg border border-rose-300/70 bg-rose-50/70 p-3 dark:border-rose-700/55 dark:bg-rose-950/15"
    >
      <div class="flex items-center gap-1.5 text-[10px] font-bold text-rose-700 dark:text-rose-300">
        <span class="i-ep-circle-close-filled text-[12px]"></span>
        无法继续
      </div>
      <ul class="mt-2 space-y-1.5">
        <li
          v-for="blocker in preview.blockers"
          :key="blocker"
          class="flex items-start gap-2 text-[10px] leading-4 text-rose-700 dark:text-rose-200"
        >
          <span class="mt-1.5 h-1 w-1 shrink-0 rounded-full bg-current"></span>
          <span>{{ blocker }}</span>
        </li>
      </ul>
    </section>

    <div
      v-if="!preview.warnings.length && !preview.blockers.length && !preview.route_conflicts.length"
      class="flex items-center gap-2 rounded-lg border border-emerald-300/60 bg-emerald-50/60 px-3 py-2 text-[10px] font-semibold text-emerald-700 dark:border-emerald-700/50 dark:bg-emerald-950/15 dark:text-emerald-300"
    >
      <span class="i-ep-circle-check-filled text-[12px]"></span>
      未发现冲突或风险项
    </div>
  </article>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { PluginImpactPreview } from "@/types";

const props = defineProps<{
  preview: PluginImpactPreview;
}>();

const featureStates = computed(() => [
  { label: "应用页面", enabled: props.preview.ui_enabled },
  { label: "面板主题", enabled: props.preview.theme_enabled },
]);

const changeGroups = computed(() =>
  [
    { label: "新增权限", items: props.preview.permissions_added, added: true },
    { label: "移除权限", items: props.preview.permissions_removed, added: false },
    { label: "新增 API 访问", items: props.preview.api_allow_added, added: true },
    { label: "移除 API 访问", items: props.preview.api_allow_removed, added: false },
  ].filter((group) => group.items.length),
);
</script>

<style>
.extension-impact-preview-dialog {
  width: min(580px, calc(100vw - 32px));
  max-width: calc(100vw - 32px);
}

.extension-impact-preview-dialog .el-message-box__content {
  align-items: flex-start;
  padding-top: 10px;
}

.extension-impact-preview-dialog .el-message-box__message {
  min-width: 0;
  width: 100%;
}
</style>
