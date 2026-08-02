<template>
  <div class="min-h-0 flex-1 overflow-y-auto px-3 pb-3 custom-scrollbar sm:px-4 sm:pb-4">
    <div v-if="loading" class="grid grid-cols-1 gap-3">
      <el-skeleton v-for="item in 5" :key="item" animated :loading="true">
        <template #template>
          <div class="flex items-center gap-3 rounded-xl bg-soft p-3.5">
            <el-skeleton-item variant="circle" style="width: 40px; height: 40px" />
            <div class="min-w-0 flex-1">
              <el-skeleton-item variant="text" style="width: 38%" />
              <el-skeleton-item variant="text" style="width: 22%; margin-top: 10px" />
            </div>
            <el-skeleton-item variant="button" style="width: 70px; height: 40px" />
          </div>
        </template>
      </el-skeleton>
    </div>

    <div
      v-else-if="packages.length === 0"
      class="h-full min-h-[240px] rounded-xl bg-soft px-5 py-10 text-center flex-col-center"
    >
      <span
        :class="searchQuery ? 'i-ep-search' : 'i-ep-box'"
        class="mb-4 h-12 w-12 rounded-xl bg-card text-xl text-primary flex-center"
        aria-hidden="true"
      ></span>
      <div class="text-[13px] font-semibold text-default">
        {{ searchQuery ? "没有匹配的依赖" : "暂未安装依赖" }}
      </div>
      <div class="mt-2 max-w-[280px] text-[10px] leading-5 text-secondary">
        {{
          searchQuery
            ? `没有找到与“${searchQuery}”相关的包，请换个关键词。`
            : "安装依赖后，可在这里查看版本并按包卸载。"
        }}
      </div>
      <ToolbarButton
        v-if="searchQuery"
        variant="soft"
        class="mt-4 !min-h-11"
        @click="emit('clear-search')"
      >
        清除搜索
      </ToolbarButton>
      <ToolbarButton
        v-else
        variant="primary"
        class="mt-4 !min-h-11"
        @click="emit('install')"
      >
        <template #icon>
          <span class="i-ep-plus"></span>
        </template>
        安装第一个依赖
      </ToolbarButton>
    </div>

    <div v-else class="surface-list !rounded-xl">
      <article
        v-for="row in packages"
        :key="row.name"
        class="surface-list__row !items-start !gap-3 !px-3 !py-3 sm:!items-center sm:!px-4"
      >
        <span
          class="h-10 w-10 shrink-0 rounded-lg bg-soft text-primary flex-center"
          aria-hidden="true"
        >
          <span class="i-ep-box"></span>
        </span>

        <div class="min-w-0 flex-1 self-center">
          <div class="break-all text-[13px] font-semibold leading-5 text-default">
            {{ row.name }}
          </div>
          <div class="mt-1 flex flex-wrap items-center gap-2">
            <span
              class="rounded-md bg-soft px-2 py-0.5 font-mono text-[10px] font-semibold text-secondary"
              :title="`版本 ${row.version}`"
            >
              {{ row.version }}
            </span>
            <span class="text-[10px] text-muted">已安装</span>
          </div>
        </div>

        <button
          type="button"
          class="min-h-11 shrink-0 self-center rounded-lg px-3 text-[11px] font-semibold text-rose-600 flex-center gap-1.5 transition-colors hover:bg-rose-500/10 disabled:cursor-not-allowed disabled:opacity-50 dark:text-rose-300"
          :disabled="Boolean(uninstallingPackage)"
          :title="`卸载 ${row.name}`"
          :aria-label="`卸载 ${row.name}`"
          @click="emit('uninstall', row.name)"
        >
          <span
            :class="
              uninstallingPackage === row.name
                ? 'i-ep-loading animate-spin'
                : 'i-ep-delete'
            "
          ></span>
          <span>
            {{ uninstallingPackage === row.name ? "处理中" : "卸载" }}
          </span>
        </button>
      </article>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Package } from "@/types";
import ToolbarButton from "../../../../components/common/ToolbarButton.vue";

defineProps<{
  loading: boolean;
  packages: Package[];
  searchQuery: string;
  uninstallingPackage: string;
}>();

const emit = defineEmits<{
  (event: "clear-search"): void;
  (event: "install"): void;
  (event: "uninstall", packageName: string): void;
}>();
</script>
