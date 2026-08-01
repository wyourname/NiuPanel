<template>
  <article
    class="group rounded-lg border border-light bg-card p-3.5 transition-[border-color,background-color,box-shadow] duration-200 hover:border-primary/20 hover:shadow-sm md:p-4"
    :aria-label="`API 密钥 ${apiKey.name}，${expired ? '已过期' : '有效'}`"
  >
    <div class="flex items-start gap-3">
      <span
        class="mt-1 h-2.5 w-2.5 shrink-0 rounded-full ring-4"
        :class="expired ? 'bg-rose-500 ring-rose-500/10' : 'bg-emerald-500 ring-emerald-500/10'"
        aria-hidden="true"
      ></span>

      <div class="min-w-0 flex-1">
        <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
          <h3 class="min-w-0 truncate text-[14px] font-bold text-default">
            {{ apiKey.name }}
          </h3>
          <span
            class="rounded-md px-1.5 py-0.5 text-[9px] font-bold"
            :class="expired ? 'danger-subtle' : 'success-subtle'"
          >
            {{ expired ? "已过期" : "有效" }}
          </span>
        </div>
        <code class="mt-1 block truncate font-mono text-[10px] text-muted">
          {{ apiKey.prefix }}
        </code>
      </div>

      <div class="flex shrink-0 items-center gap-1">
        <button
          type="button"
          class="h-8 w-8 cursor-pointer rounded-md text-muted flex-center transition-colors hover:bg-primary/8 hover:text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
          :aria-label="`编辑密钥 ${apiKey.name}`"
          title="编辑"
          @click="emit('edit')"
        >
          <span class="i-ep-edit text-[14px]"></span>
        </button>
        <button
          type="button"
          class="h-8 w-8 cursor-pointer rounded-md text-muted flex-center transition-colors hover:bg-rose-500/10 hover:text-rose-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-rose-400/40"
          :aria-label="`吊销密钥 ${apiKey.name}`"
          title="吊销"
          @click="emit('delete')"
        >
          <span class="i-ep-delete text-[14px]"></span>
        </button>
      </div>
    </div>

    <div class="mt-3 grid gap-3 border-t border-light/70 pt-3 lg:grid-cols-[minmax(220px,1.15fr)_minmax(0,1fr)]">
      <section class="min-w-0" aria-label="权限范围">
        <div class="mb-2 flex items-center justify-between gap-2">
          <span class="text-[10px] font-semibold uppercase tracking-[0.08em] text-muted">权限范围</span>
          <span class="text-[10px] text-muted">{{ permissionSummary }}</span>
        </div>
        <div v-if="permissions.length" class="flex min-w-0 flex-wrap gap-1.5">
          <el-tag
            v-for="permission in visiblePermissions"
            :key="permission"
            size="small"
            effect="light"
            :type="getPermColor(permission)"
            class="!max-w-full !rounded-md !border-transparent !text-[9px]"
            :title="permission"
          >
            <span class="block max-w-[150px] truncate font-mono">{{ permission }}</span>
          </el-tag>
          <span
            v-if="hiddenPermissionCount"
            class="rounded-md bg-soft px-2 py-1 font-mono text-[9px] font-semibold text-secondary"
            :title="hiddenPermissions.join(', ')"
          >
            +{{ hiddenPermissionCount }}
          </span>
        </div>
        <div v-else class="flex items-center gap-2 rounded-md bg-soft/60 px-2.5 py-2 text-[10px] text-secondary">
          <span class="i-ep-lock text-muted"></span>
          未授予任何权限，此密钥无法访问 API
        </div>
        <div
          v-if="mcpTools.length"
          class="mt-2 flex items-center gap-2 rounded-md bg-emerald-500/[0.07] px-2.5 py-2 text-[10px] text-emerald-700 dark:text-emerald-300"
        >
          <span class="i-carbon-network-4 text-[13px]"></span>
          <span class="min-w-0 flex-1 truncate">
            {{ expired ? "密钥已过期，无法连接 MCP" : `MCP 可调用 ${mcpAccess.accessible}/${mcpAccess.total} 个工具` }}
          </span>
          <span v-if="!expired && mcpAccess.destructive" class="shrink-0 text-amber-600 dark:text-amber-300">
            含 {{ mcpAccess.destructive }} 个高风险操作
          </span>
        </div>
      </section>

      <dl class="grid grid-cols-2 gap-x-3 gap-y-2 sm:grid-cols-3">
        <div class="min-w-0 rounded-md bg-subtle px-2.5 py-2">
          <dt class="flex items-center gap-1 text-[9px] font-semibold text-muted">
            <span class="i-ep-calendar"></span>
            创建时间
          </dt>
          <dd class="mt-1 truncate text-[10px] font-medium text-secondary" :title="formatDate(apiKey.created_at)">
            {{ formatDate(apiKey.created_at) }}
          </dd>
        </div>
        <div class="min-w-0 rounded-md bg-subtle px-2.5 py-2">
          <dt class="flex items-center gap-1 text-[9px] font-semibold text-muted">
            <span class="i-ep-timer"></span>
            到期时间
          </dt>
          <dd
            class="mt-1 truncate text-[10px] font-medium"
            :class="expired ? 'text-rose-500' : 'text-secondary'"
            :title="expirationLabel"
          >
            {{ expirationLabel }}
          </dd>
        </div>
        <div class="col-span-2 min-w-0 rounded-md bg-subtle px-2.5 py-2 sm:col-span-1">
          <dt class="flex items-center gap-1 text-[9px] font-semibold text-muted">
            <span class="i-ep-position"></span>
            最后使用
          </dt>
          <dd class="mt-1 min-w-0 text-[10px] font-medium text-secondary">
            <span class="block truncate" :title="lastUsedLabel">{{ lastUsedLabel }}</span>
            <span v-if="apiKey.last_used_at" class="mt-0.5 block truncate font-mono text-[9px] text-muted">
              {{ apiKey.last_used_ip || "Internal" }}
            </span>
          </dd>
        </div>
      </dl>
    </div>
  </article>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { ApiKey } from "@/api/keys";
import type { McpToolInfo } from "@/types";
import { formatDate } from "@/utils/format";
import { getPermColor, isExpired, parsePerms } from "../utils/apiKeyPermissions";
import { getMcpAccessSummary } from "../utils/mcpPermissions";

const props = defineProps<{
  apiKey: ApiKey;
  mcpTools: McpToolInfo[];
}>();

const emit = defineEmits<{
  (event: "delete"): void;
  (event: "edit"): void;
}>();

const permissions = computed(() => parsePerms(props.apiKey.permissions));
const mcpAccess = computed(() =>
  getMcpAccessSummary(permissions.value, props.mcpTools),
);
const visiblePermissions = computed(() => permissions.value.slice(0, 5));
const hiddenPermissions = computed(() => permissions.value.slice(5));
const hiddenPermissionCount = computed(() => hiddenPermissions.value.length);
const permissionSummary = computed(() =>
  permissions.value.includes("*:*") ? "最高权限" : `${permissions.value.length} 项`,
);
const expired = computed(() => isExpired(props.apiKey.expires_at));
const expirationLabel = computed(() =>
  props.apiKey.expires_at ? formatDate(props.apiKey.expires_at) : "永不过期",
);
const lastUsedLabel = computed(() =>
  props.apiKey.last_used_at ? formatDate(props.apiKey.last_used_at) : "从未使用",
);
</script>
