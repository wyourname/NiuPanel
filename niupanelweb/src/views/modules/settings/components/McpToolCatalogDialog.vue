<template>
  <ResponsiveDialog
    v-model:visible="visibleValue"
    title="MCP 工具目录"
    width="860px"
    append-to-body
    destroy-on-close
    custom-class="cloud-dialog"
  >
    <div class="flex h-full min-h-[320px] flex-col bg-card md:h-[min(640px,72vh)]">
      <div class="flex shrink-0 items-center justify-between gap-3 border-b border-light px-4 py-3">
        <p class="min-w-0 text-[11px] leading-5 text-secondary">工具可用范围由客户端所用 API Key 的 scope 决定。</p>
        <span class="shrink-0 rounded-md bg-soft px-2 py-1 text-[10px] font-semibold text-secondary">{{ tools.length }} 个</span>
      </div>

      <div v-loading="loading" class="min-h-0 flex-1 overflow-y-auto px-4 py-3 custom-scrollbar" :aria-busy="loading">
        <div v-if="error" class="py-12 text-center text-[11px] text-rose-500">{{ error }}</div>
        <div v-else-if="groupedTools.length" class="space-y-4">
          <section v-for="group in groupedTools" :key="group.category">
            <div class="mb-1.5 flex items-center justify-between px-1">
              <span class="text-[10px] font-semibold text-secondary">{{ categoryLabel(group.category) }}</span>
              <span class="text-[10px] text-muted">{{ group.tools.length }}</span>
            </div>
            <div class="surface-list">
              <div
                v-for="tool in group.tools"
                :key="tool.name"
                class="surface-list__row !items-start !gap-2 !px-3 !py-2.5"
              >
                <span class="mt-0.5 h-7 w-7 shrink-0 rounded-md bg-soft text-primary flex-center">
                  <span :class="tool.destructive ? 'i-ep-warning text-amber-500' : 'i-ep-connection'" class="text-[13px]"></span>
                </span>
                <span class="min-w-0 flex-1">
                  <span class="block truncate font-mono text-[11px] font-semibold text-default">{{ tool.name }}</span>
                  <span class="mt-0.5 block text-[10px] leading-4 text-secondary">{{ tool.description }}</span>
                  <span class="mt-1 block truncate font-mono text-[10px] text-muted">需 API scope · {{ tool.permission }}</span>
                </span>
              </div>
            </div>
          </section>
        </div>
        <div v-else class="py-12 text-center text-[11px] text-muted">
          {{ loading ? "正在读取 MCP 工具..." : "暂无 MCP 工具" }}
        </div>
      </div>
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";
import type { McpInfo, McpToolInfo } from "@/types";

const props = defineProps<{
  visible: boolean;
  error: string;
  info: McpInfo | null;
  loading: boolean;
}>();

const emit = defineEmits<{
  (event: "update:visible", value: boolean): void;
}>();

const categoryNames: Record<string, string> = {
  system: "系统",
  tasks: "任务",
  environments: "运行环境",
  variables: "变量",
  audit: "审计",
  files: "文件",
  jobs: "后台作业",
  notifications: "通知",
  share: "分享",
  git: "Git",
};

const visibleValue = computed({
  get: () => props.visible,
  set: (value) => emit("update:visible", value),
});
const tools = computed(() => props.info?.tools ?? []);
const groupedTools = computed(() => {
  const groups = new Map<string, McpToolInfo[]>();
  for (const tool of tools.value) {
    const groupTools = groups.get(tool.category) ?? [];
    groupTools.push(tool);
    groups.set(tool.category, groupTools);
  }
  return Array.from(groups, ([category, groupTools]) => ({
    category,
    tools: groupTools,
  }));
});

const categoryLabel = (category: string) => categoryNames[category] ?? category;
</script>
