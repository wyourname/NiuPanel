<script setup lang="ts">
import { useAppStore } from "../../../../stores/app";
import { useHaptics } from "../../../../composables/useHaptics";
import type { Env } from "@/types";

defineProps<{
  data: Env[];
  loading?: boolean;
}>();

const emit = defineEmits<{
  (event: "view-logs", env: Env): void;
  (event: "manage-packages", env: Env): void;
  (event: "delete", env: Env): void;
  (event: "create", env: Env): void;
  (event: "set-default", env: Env): void;
}>();
const appStore = useAppStore();
const haptics = useHaptics();
const isNodeDefault = (env: Env) =>
  env.env_type === "node" &&
  (env.path === "System Default" || env.path === "Active Default (Global)");

const getSummary = (env: Env) => {
  if (env.env_type === "node") {
    return env.version
      ? `依赖安装在该版本共享目录 data/runtimes/node/shared/${env.version}/node_modules`
      : "依赖安装在对应 Node 版本共享目录";
  }
  if (env.is_installed) {
    return env.path || "已安装";
  }
  return env.recorded_packages
    ? `已记录 ${env.recorded_packages} 个依赖`
    : "已记录配置快照";
};

const handlePackages = (env: Env) => {
  haptics.impact();
  emit("manage-packages", env);
};

const handleDelete = (env: Env) => {
  haptics.notification();
  emit("delete", env);
};

const handleCreate = (env: Env) => {
  haptics.impact();
  emit("create", env);
};

type MobileEnvCommand = {
  action: "packages" | "default" | "restore" | "delete";
  env: Env;
};

const handleMobileCommand = ({ action, env }: MobileEnvCommand) => {
  if (action === "packages") handlePackages(env);
  else if (action === "default") emit("set-default", env);
  else if (action === "restore") handleCreate(env);
  else handleDelete(env);
};
</script>

<template>
  <div class="flex-1 overflow-hidden relative flex flex-col">
    <div
      v-if="!appStore.isMobile"
      v-loading="loading"
      class="flex-1 overflow-y-auto custom-scrollbar"
    >
      <div v-if="data.length === 0" class="h-full min-h-[320px] flex-col-center text-muted opacity-60">
        <span class="text-sm font-semibold">暂无环境记录</span>
      </div>

      <div v-else class="divide-y divide-light/80 border-y border-light/70">
        <div
          v-for="row in data"
          :key="row.name"
          class="flex items-start gap-3 px-4 py-3 transition-colors hover:bg-soft/45"
        >
          <div class="h-10 w-10 shrink-0 rounded-lg border border-light/70 bg-base/70 flex-center text-primary">
            <div
              :class="
                row.env_type === 'python'
                  ? 'i-logos-python'
                  : row.env_type === 'node'
                    ? 'i-logos-nodejs-icon'
                    : 'i-carbon-terminal'
              "
              class="text-2xl"
            ></div>
          </div>

          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <span class="text-[14px] font-semibold text-default">
                {{ row.env_type === 'node' ? row.name.replace(' (Active Default)', '') : row.name }}
              </span>
              <span
                v-if="row.version"
                class="inline-flex items-center rounded-md border border-light/70 bg-base/70 px-1.5 py-0.5 font-mono text-[10px] font-bold text-secondary"
              >
                {{ row.version }}
              </span>
              <span
                v-if="isNodeDefault(row)"
                class="inline-flex items-center rounded-md bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-semibold text-emerald-600 dark:text-emerald-300"
              >
                当前默认
              </span>
              <span
                v-if="!row.is_installed"
                class="inline-flex items-center rounded-md bg-rose-500/10 px-1.5 py-0.5 text-[10px] font-semibold text-rose-500"
              >
                未安装
              </span>
            </div>

            <div class="mt-1 break-all text-[12px] leading-5 text-secondary">
              {{ getSummary(row) }}
            </div>
          </div>

          <div class="flex shrink-0 items-center gap-1">
            <button
              v-if="row.is_installed"
              class="h-8 rounded-md px-2.5 text-[11px] font-bold text-secondary flex-center gap-1 transition-colors hover:bg-soft hover:text-default"
              @click="handlePackages(row)"
            >
              <div class="i-ep-box"></div>
              包管理
            </button>
            <button
              v-if="row.env_type === 'node' && !isNodeDefault(row)"
              class="h-8 rounded-md px-2.5 text-[11px] font-bold text-secondary flex-center gap-1 transition-colors hover:bg-soft hover:text-default"
              @click="emit('set-default', row)"
            >
              <div class="i-ep-aim"></div>
              设为默认
            </button>
            <button
              v-if="row.env_type !== 'node' && !row.is_installed"
              class="h-8 rounded-md px-2.5 text-[11px] font-bold text-secondary flex-center gap-1 transition-colors hover:bg-soft hover:text-default"
              @click="handleCreate(row)"
            >
              <div class="i-ep-refresh"></div>
              恢复安装
            </button>
            <button
              v-if="row.env_type === 'python' || row.env_type === 'node'"
              class="h-8 rounded-md px-2.5 text-[11px] font-bold text-rose-600 flex-center gap-1 transition-colors hover:bg-rose-500/10 dark:text-rose-300"
              @click="handleDelete(row)"
            >
              <div class="i-ep-delete"></div>
              删除
            </button>
          </div>
        </div>
      </div>
    </div>

    <div
      v-else
      class="mobile-dock-safe flex-1 overflow-y-auto custom-scrollbar"
    >
      <div
        v-if="data.length === 0 && !loading"
        class="h-60 flex-col-center text-muted opacity-50 select-none"
      >
        <div class="i-ep-box text-5xl mb-4"></div>
        <span class="text-sm font-semibold">暂无环境记录</span>
      </div>

      <div v-else class="divide-y divide-light/70 border-b border-light/70 bg-card">
        <article
          v-for="env in data"
          :key="env.name"
          class="px-4 py-3"
        >
          <div class="flex min-w-0 items-start gap-3">
            <div class="h-10 w-10 shrink-0 rounded-md border border-light/70 bg-base/70 flex-center">
                <div
                  :class="
                    env.env_type === 'python'
                      ? 'i-logos-python'
                      : env.env_type === 'node'
                        ? 'i-logos-nodejs-icon'
                        : 'i-carbon-terminal'
                  "
                  class="text-2xl"
                ></div>
            </div>

            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <h3 class="m-0 truncate text-[13px] font-semibold text-default">{{ env.name }}</h3>
                <span
                  v-if="isNodeDefault(env)"
                  class="rounded bg-emerald-500/10 px-1.5 py-0.5 text-[9px] font-bold text-emerald-600 dark:text-emerald-300"
                >当前默认</span>
                <span
                  v-if="!env.is_installed"
                  class="rounded bg-rose-500/10 px-1.5 py-0.5 text-[9px] font-bold text-rose-500"
                >未安装</span>
              </div>
              <div class="mt-0.5 font-mono text-[10px] text-muted">{{ env.version || env.env_type }}</div>
              <p class="mt-1.5 break-all text-[11px] leading-5 text-secondary">{{ getSummary(env) }}</p>
            </div>

            <el-dropdown trigger="click" @command="handleMobileCommand">
              <button
                type="button"
                class="h-8 w-8 shrink-0 rounded-md text-secondary flex-center transition-colors hover:bg-soft hover:text-default"
                title="环境操作"
                aria-label="环境操作"
              >
                <span class="i-ep-more-filled"></span>
              </button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item v-if="env.is_installed" :command="{ action: 'packages', env }">
                    <span class="i-ep-box mr-2"></span>包管理
                  </el-dropdown-item>
                  <el-dropdown-item
                    v-if="env.env_type === 'node' && !isNodeDefault(env)"
                    :command="{ action: 'default', env }"
                  >
                    <span class="i-ep-aim mr-2"></span>设为默认
                  </el-dropdown-item>
                  <el-dropdown-item
                    v-if="env.env_type !== 'node' && !env.is_installed"
                    :command="{ action: 'restore', env }"
                  >
                    <span class="i-ep-refresh mr-2"></span>恢复安装
                  </el-dropdown-item>
                  <el-dropdown-item
                    v-if="env.env_type === 'python' || env.env_type === 'node'"
                    divided
                    :command="{ action: 'delete', env }"
                  >
                    <span class="i-ep-delete mr-2 text-rose-500"></span>
                    <span class="text-rose-600">删除环境</span>
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
        </article>
      </div>
    </div>
  </div>
</template>
