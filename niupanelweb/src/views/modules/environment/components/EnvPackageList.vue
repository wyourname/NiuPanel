<template>
  <div class="dependency-package-list custom-scrollbar" :aria-busy="loading">
    <div v-if="loading" aria-label="正在加载依赖列表" class="px-5">
      <el-skeleton v-for="item in 7" :key="item" animated :loading="true">
        <template #template>
          <div class="flex items-center gap-3 border-b border-light py-3.5">
            <div class="min-w-0 flex-1">
              <el-skeleton-item variant="text" style="display: block; width: 55%" />
              <el-skeleton-item variant="text" style="display: block; width: 25%; margin-top: 8px" />
            </div>
            <el-skeleton-item variant="button" style="width: 60px; height: 32px" />
          </div>
        </template>
      </el-skeleton>
    </div>

    <div
      v-else-if="packages.length === 0"
      class="dependency-package-empty text-center flex-col-center"
    >
      <span
        class="i-ep-box mb-4 h-12 w-12 text-3xl text-secondary"
        aria-hidden="true"
      ></span>
      <div class="text-sm font-semibold text-default">
        暂未安装依赖
      </div>
      <div class="mt-2 max-w-[280px] break-all text-xs leading-5 text-secondary">
        安装依赖后，可在这里查看版本并按包卸载。
      </div>
      <ToolbarButton
        variant="soft"
        class="mt-4 !min-h-11 gap-2"
        :disabled="busy"
        @click="emit('install')"
      >
        <template #icon>
          <span class="i-ep-plus"></span>
        </template>
        安装第一个依赖
      </ToolbarButton>
    </div>

    <table v-else class="dependency-package-table" aria-label="已安装的依赖包">
      <colgroup>
        <col class="dependency-package-index" />
        <col />
        <col class="dependency-package-version" />
        <col class="dependency-package-action" />
      </colgroup>
      <thead>
        <tr>
          <th scope="col" class="dependency-package-index">序号</th>
          <th scope="col">依赖名称</th>
          <th scope="col" class="dependency-package-version">当前版本</th>
          <th scope="col" class="dependency-package-action">操作</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(row, index) in packages" :key="row.name" class="dependency-package-row">
          <td class="dependency-package-index tabular-nums text-secondary">{{ index + 1 }}</td>
          <td>
            <span class="break-all font-mono text-[13px] font-medium text-default">{{ row.name }}</span>
            <span class="mt-1 block break-all font-mono text-xs text-secondary md:hidden">版本 {{ row.version }}</span>
          </td>
          <td class="dependency-package-version break-all font-mono text-xs text-secondary">{{ row.version }}</td>
          <td class="dependency-package-action">
            <button
              type="button"
              class="dependency-package-uninstall min-h-11 cursor-pointer rounded-lg px-3 text-xs font-medium text-secondary inline-flex items-center justify-center gap-1.5 transition-colors hover:bg-rose-500/10 hover:text-rose-600 disabled:cursor-not-allowed disabled:opacity-50 dark:hover:text-rose-300"
              :disabled="busy || Boolean(uninstallingPackage)"
              :title="`卸载 ${row.name}`"
              :aria-label="`卸载 ${row.name}`"
              @click="emit('uninstall', row.name)"
            >
              <span
                :class="uninstallingPackage === row.name ? 'i-ep-loading animate-spin motion-reduce:animate-none' : 'i-ep-delete'"
                aria-hidden="true"
              ></span>
              {{ uninstallingPackage === row.name ? "处理中" : "卸载" }}
            </button>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import type { Package } from "@/types";
import ToolbarButton from "../../../../components/common/ToolbarButton.vue";

defineProps<{
  busy: boolean;
  loading: boolean;
  packages: Package[];
  uninstallingPackage: string;
}>();

const emit = defineEmits<{
  (event: "install"): void;
  (event: "uninstall", packageName: string): void;
}>();
</script>

<style scoped>
.dependency-package-list {
  min-width: 0;
  flex: 1 0 auto;
}

.dependency-package-table {
  width: 100%;
  table-layout: fixed;
  border-spacing: 0;
  text-align: left;
  font-size: 13px;
  line-height: 20px;
}

.dependency-package-table th {
  padding: 10px 16px;
  border-bottom: 1px solid var(--border-light);
  background: var(--bg-subtle);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
}

.dependency-package-table td {
  height: 52px;
  padding: 4px 16px;
  border-bottom: 1px solid var(--border-light);
}

.dependency-package-table .dependency-package-index {
  display: none;
  width: 64px;
}

.dependency-package-table .dependency-package-version {
  display: none;
  width: 28%;
}

.dependency-package-table .dependency-package-action {
  width: 100px;
  padding-right: 12px;
  padding-left: 8px;
  text-align: center;
}

.dependency-package-row:hover {
  background: var(--bg-subtle);
}

.dependency-package-row:last-child td {
  border-bottom: 0;
}

.dependency-package-empty {
  min-height: 180px;
  padding: 24px 20px;
}

.dependency-package-uninstall:focus-visible {
  outline: 2px solid var(--el-color-primary);
  outline-offset: 2px;
}

@media (min-width: 769px) {
  .dependency-package-list {
    min-height: 0;
    max-height: min(560px, calc(var(--app-viewport-height) - 220px));
    flex: 0 1 auto;
    overflow-y: auto;
  }

  .dependency-package-table th {
    position: sticky;
    z-index: 1;
    top: 0;
  }

  .dependency-package-table col.dependency-package-index,
  .dependency-package-table col.dependency-package-version {
    display: table-column;
  }

  .dependency-package-table th.dependency-package-index,
  .dependency-package-table td.dependency-package-index,
  .dependency-package-table th.dependency-package-version,
  .dependency-package-table td.dependency-package-version {
    display: table-cell;
  }
}
</style>
