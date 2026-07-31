<template>
  <section class="module-panel shrink-0 overflow-hidden">
    <header class="flex items-center justify-between gap-3 border-b border-light px-4 py-3">
      <div class="flex min-w-0 items-center gap-3">
        <span class="h-9 w-9 shrink-0 rounded-md bg-emerald-500/10 text-emerald-600 flex-center dark:text-emerald-300">
          <span class="i-carbon-network-4 text-[16px]"></span>
        </span>
        <div class="min-w-0">
          <h3 class="truncate text-[14px] font-bold text-default">MCP 接入</h3>
          <p class="mt-0.5 truncate text-[11px] text-muted">Streamable HTTP · X-API-Key</p>
        </div>
      </div>
      <span
        class="rounded-md px-2 py-1 text-[11px] font-semibold"
        :class="info?.enabled ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-300' : 'bg-soft text-muted'"
      >
        {{ info?.enabled ? "已启用" : "读取中" }}
      </span>
    </header>

    <div class="grid gap-4 p-4 lg:grid-cols-[minmax(0,1fr)_minmax(300px,0.8fr)]">
      <div class="min-w-0 space-y-3">
        <div>
          <div class="mb-1 text-[11px] font-semibold text-muted">服务地址</div>
          <div class="flex items-center gap-2">
            <code class="min-w-0 flex-1 truncate rounded-md border border-light bg-subtle px-3 py-2 font-mono text-[12px] text-default">
              {{ endpoint }}
            </code>
            <button
              type="button"
              class="h-9 w-9 shrink-0 rounded-md border border-light bg-card text-secondary flex-center transition-colors hover:bg-soft hover:text-primary"
              title="复制 MCP 地址"
              aria-label="复制 MCP 地址"
              @click="copyText(endpoint, 'MCP 地址已复制')"
            >
              <span class="i-ep-copy-document text-[14px]"></span>
            </button>
          </div>
        </div>

        <div>
          <div class="mb-1 text-[11px] font-semibold text-muted">客户端配置</div>
          <div class="relative rounded-md border border-light bg-subtle p-3 pr-11">
            <pre class="m-0 overflow-x-auto whitespace-pre-wrap break-all font-mono text-[11px] leading-5 text-secondary">{{ clientConfig }}</pre>
            <button
              type="button"
              class="absolute right-2 top-2 h-8 w-8 rounded-md text-muted flex-center transition-colors hover:bg-card hover:text-primary"
              title="复制客户端配置"
              aria-label="复制客户端配置"
              @click="copyText(clientConfig, '客户端配置已复制')"
            >
              <span class="i-ep-copy-document text-[14px]"></span>
            </button>
          </div>
        </div>
      </div>

      <div class="min-w-0">
        <div class="mb-2 flex items-center justify-between gap-2">
          <span class="text-[11px] font-semibold text-muted">当前工具</span>
          <span class="text-[11px] text-muted">{{ info?.tools.length ?? 0 }} 个</span>
        </div>
        <div class="max-h-[360px] space-y-3 overflow-y-auto pr-1 custom-scrollbar">
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
                  <span class="mt-1 block truncate font-mono text-[10px] text-muted">{{ tool.permission }}</span>
                </span>
              </div>
            </div>
          </section>
          <div v-if="!groupedTools.length" class="py-10 text-center text-[11px] text-muted">
            正在读取 MCP 工具...
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { ElMessage } from "element-plus";
import useClipboard from "vue-clipboard3";
import { getMcpInfo } from "@/api/mcp";
import { useAppStore } from "@/stores/app";
import type { McpInfo } from "@/types";

const appStore = useAppStore();
const { toClipboard } = useClipboard();
const info = ref<McpInfo | null>(null);
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

const categoryLabel = (category: string) => categoryNames[category] ?? category;
const groupedTools = computed(() => {
  const groups = new Map<string, NonNullable<McpInfo["tools"]>>();
  for (const tool of info.value?.tools ?? []) {
    const tools = groups.get(tool.category) ?? [];
    tools.push(tool);
    groups.set(tool.category, tools);
  }
  return Array.from(groups, ([category, tools]) => ({ category, tools }));
});

const endpoint = computed(() => {
  const base = appStore.serverUrl?.replace(/\/$/, "") || window.location.origin;
  return `${base}${info.value?.endpoint ?? "/mcp"}`;
});

const clientConfig = computed(() =>
  JSON.stringify(
    {
      mcpServers: {
        niupanel: {
          type: "streamable-http",
          url: endpoint.value,
          headers: { "X-API-Key": "<NIUPANEL_API_KEY>" },
        },
      },
    },
    null,
    2,
  ),
);

const copyText = async (value: string, message: string) => {
  await toClipboard(value);
  ElMessage.success(message);
};

onMounted(async () => {
  const response = await getMcpInfo();
  info.value = response.data;
});
</script>
