<template>
  <div class="flex-1 overflow-hidden relative flex flex-col">
    <div
      v-if="!isMobile"
      class="h-full overflow-hidden rounded-md border border-light/50 bg-card"
    >
      <el-table
        :data="keys"
        v-loading="loading"
        height="100%"
        style="width: 100%"
        class="modern-table"
      >
        <el-table-column label="名称 / 前缀" min-width="200">
          <template #default="{ row }">
            <div class="flex flex-col gap-1 py-1">
              <span class="font-bold text-default text-sm">{{ row.name }}</span>
              <code class="text-[10px] text-muted opacity-60">{{ row.prefix }}</code>
            </div>
          </template>
        </el-table-column>

        <el-table-column label="权限范围" min-width="250">
          <template #default="{ row }">
            <div class="flex flex-wrap gap-1.5">
              <el-tag
                v-for="permission in parsePerms(row.permissions)"
                :key="permission"
                size="small"
                effect="light"
                :type="getPermColor(permission)"
                class="!rounded-md border-transparent"
              >
                {{ permission }}
              </el-tag>
              <span v-if="!row.permissions" class="text-xs text-muted italic">无限制</span>
            </div>
          </template>
        </el-table-column>

        <el-table-column label="状态 / 最后使用" width="240">
          <template #default="{ row }">
            <div class="flex flex-col gap-1.5 text-[11px] py-1">
              <div
                class="flex items-center gap-1.5"
                :class="isExpired(row.expires_at) ? 'text-danger' : 'text-muted'"
              >
                <div class="i-ep-clock"></div>
                <span>过期: {{ formatDate(row.expires_at) }}</span>
              </div>
              <div
                v-if="row.last_used_at"
                class="flex items-center gap-1.5 text-primary font-bold"
              >
                <div class="i-ep-position"></div>
                <span>
                  {{ row.last_used_ip || "Internal" }} ·
                  {{ formatDate(row.last_used_at) }}
                </span>
              </div>
              <div v-else class="text-muted italic opacity-40">未使用过</div>
            </div>
          </template>
        </el-table-column>

        <el-table-column label="操作" width="120" align="right" fixed="right">
          <template #default="{ row }">
            <div class="flex justify-end gap-2">
              <el-button type="primary" link @click="emit('edit', row)">
                <div class="i-ep-edit text-lg"></div>
              </el-button>
              <el-button type="danger" link @click="emit('delete', row)">
                <div class="i-ep-delete text-lg"></div>
              </el-button>
            </div>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <div v-else class="full">
      <PullToRefresh :on-refresh="onRefresh">
        <div class="mobile-dock-safe h-full overflow-y-auto border-y border-light bg-card custom-scrollbar">
          <article
            v-for="row in keys"
            :key="row.id"
            class="border-b border-light/70 p-4 last:border-b-0"
          >
            <div class="flex justify-between items-start mb-3">
              <div class="flex flex-col min-w-0">
                <span class="font-bold text-default truncate text-base">
                  {{ row.name }}
                </span>
                <span class="text-[10px] font-mono text-muted mt-0.5">
                  {{ row.prefix }}
                </span>
              </div>
              <div class="flex gap-1">
                <button
                  type="button"
                  class="h-8 w-8 rounded-md text-primary flex-center transition-colors hover:bg-soft"
                  aria-label="编辑 API Key"
                  @click="emit('edit', row)"
                >
                  <div class="i-ep-edit text-xl"></div>
                </button>
                <button
                  type="button"
                  class="h-8 w-8 rounded-md text-rose-500 flex-center transition-colors hover:bg-rose-500/10"
                  aria-label="删除 API Key"
                  @click="emit('delete', row)"
                >
                  <div class="i-ep-delete text-xl"></div>
                </button>
              </div>
            </div>

            <div class="flex flex-wrap gap-1 mb-3">
              <el-tag
                v-for="permission in parsePerms(row.permissions).slice(0, 4)"
                :key="permission"
                size="small"
                :type="getPermColor(permission)"
                class="!text-[9px] border-0"
              >
                {{ permission }}
              </el-tag>
            </div>

            <div
              class="flex items-center justify-between rounded-md border border-base/30 bg-base/50 p-2 text-[10px] text-muted"
            >
              <div class="flex items-center gap-1">
                <div class="i-ep-clock"></div>
                {{ formatDate(row.expires_at).split(" ")[0] }}
              </div>
              <div v-if="row.last_used_at" class="flex items-center gap-1">
                <div class="i-ep-position text-primary"></div>
                {{ row.last_used_ip }}
              </div>
            </div>
          </article>
        </div>
      </PullToRefresh>
    </div>
  </div>
</template>

<script setup lang="ts">
import PullToRefresh from "@/components/common/PullToRefresh.vue";
import type { ApiKey } from "@/api/keys";
import { formatDate } from "@/utils/format";
import {
  getPermColor,
  isExpired,
  parsePerms,
} from "../utils/apiKeyPermissions";

defineProps<{
  isMobile: boolean;
  keys: ApiKey[];
  loading: boolean;
  onRefresh: () => Promise<unknown> | unknown;
}>();

const emit = defineEmits<{
  (event: "delete", key: ApiKey): void;
  (event: "edit", key: ApiKey): void;
}>();
</script>
