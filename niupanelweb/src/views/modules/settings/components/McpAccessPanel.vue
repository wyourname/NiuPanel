<template>
  <section class="module-panel shrink-0 overflow-hidden">
    <header class="flex items-center justify-between gap-3 border-b border-light px-4 py-3">
      <div class="flex min-w-0 items-center gap-3">
        <span class="h-9 w-9 shrink-0 rounded-md bg-emerald-500/10 text-emerald-600 flex-center dark:text-emerald-300">
          <span class="i-carbon-network-4 text-[16px]"></span>
        </span>
        <div class="min-w-0">
          <h3 class="truncate text-[14px] font-bold text-default">MCP 接入</h3>
          <p class="mt-0.5 truncate text-[11px] text-muted">Streamable HTTP · 与 API Key 共用权限</p>
        </div>
      </div>
      <div class="flex shrink-0 items-center gap-2">
        <span
          class="rounded-md px-2 py-1 text-[11px] font-semibold"
          :class="info?.enabled ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-300' : error ? 'bg-rose-500/10 text-rose-600 dark:text-rose-300' : 'bg-soft text-muted'"
        >
          {{ statusLabel }}
        </span>
        <el-button
          type="primary"
          class="!h-8 !rounded-lg !px-3 !text-[12px] font-bold"
          @click="emit('create-key')"
        >
          <span class="i-ep-plus mr-1 text-sm"></span>
          创建新密钥
        </el-button>
      </div>
    </header>

    <div class="p-4">
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

        <div class="rounded-md border border-emerald-500/20 bg-emerald-500/[0.055] px-3 py-2.5">
          <div class="flex items-center gap-1.5 text-[11px] font-semibold text-emerald-800 dark:text-emerald-200">
            <span class="i-carbon-security text-[13px]"></span>
            授权方式
          </div>
          <p class="mt-1 text-[10px] leading-4 text-emerald-700/85 dark:text-emerald-200/80">
            MCP 没有独立授权开关。客户端使用某把 API Key 后，可调用范围完全由这把 Key 的 API scope 决定。
          </p>
          <div class="mt-2 text-[10px] text-emerald-700/75 dark:text-emerald-200/70">
            {{ keys.length ? `当前已有 ${keys.length} 把 API Key，可在密钥历史中管理。` : "尚未创建 API Key；请创建一把最小权限密钥。" }}
          </div>
        </div>

        <div class="grid gap-2 sm:grid-cols-2">
          <button
            type="button"
            class="group flex min-w-0 items-center justify-between gap-3 rounded-md border border-light bg-subtle px-3 py-2.5 text-left transition-colors hover:border-primary/30 hover:bg-primary/5"
            @click="emit('view-keys')"
          >
            <span class="min-w-0">
              <span class="block text-[11px] font-semibold text-default">密钥历史</span>
              <span class="mt-0.5 block text-[10px] text-muted">{{ keys.length }} 把 API Key</span>
            </span>
            <span class="i-ep-arrow-right shrink-0 text-[14px] text-muted transition-transform group-hover:translate-x-0.5 group-hover:text-primary"></span>
          </button>
          <button
            type="button"
            class="group flex min-w-0 items-center justify-between gap-3 rounded-md border border-light bg-subtle px-3 py-2.5 text-left transition-colors hover:border-primary/30 hover:bg-primary/5"
            @click="emit('view-tools')"
          >
            <span class="min-w-0">
              <span class="block text-[11px] font-semibold text-default">工具目录</span>
              <span class="mt-0.5 block text-[10px] text-muted">{{ tools.length }} 个 MCP 工具</span>
            </span>
            <span class="i-ep-arrow-right shrink-0 text-[14px] text-muted transition-transform group-hover:translate-x-0.5 group-hover:text-primary"></span>
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { ElMessage } from "element-plus";
import useClipboard from "vue-clipboard3";
import type { ApiKey } from "@/api/keys";
import { useAppStore } from "@/stores/app";
import type { McpInfo } from "@/types";

const appStore = useAppStore();
const { toClipboard } = useClipboard();
const props = defineProps<{
  error: string;
  info: McpInfo | null;
  keys: ApiKey[];
  loading: boolean;
}>();

const emit = defineEmits<{
  (event: "create-key"): void;
  (event: "view-keys"): void;
  (event: "view-tools"): void;
}>();

const tools = computed(() => props.info?.tools ?? []);

const statusLabel = computed(() => {
  if (props.info?.enabled) return "已启用";
  if (props.error) return "读取失败";
  return props.loading ? "读取中" : "未启用";
});

const endpoint = computed(() => {
  const base = appStore.serverUrl?.replace(/\/$/, "") || window.location.origin;
  return `${base}${props.info?.endpoint ?? "/mcp"}`;
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
</script>
