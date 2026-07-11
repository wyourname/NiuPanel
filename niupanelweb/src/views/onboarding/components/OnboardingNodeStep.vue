<template>
  <div class="flex flex-col gap-6">
    <div class="flex items-center gap-4">
      <div class="h-11 w-11 shrink-0 rounded-lg bg-emerald-500/10 text-emerald-600 flex-center dark:text-emerald-300">
        <div class="i-carbon-logo-nodejs text-xl"></div>
      </div>
      <div>
        <h2 class="text-lg font-bold text-default">创建 Node.js 环境</h2>
        <p class="text-xs text-muted mt-0.5">
          通过 fnm 管理多版本运行时，每个版本使用独立共享依赖目录
        </p>
      </div>
    </div>

    <div
      class="rounded-lg border border-light bg-subtle p-4"
    >
      <div class="flex items-center gap-2 mb-3">
        <div class="text-xs font-bold text-default">Node.js 版本</div>
        <div class="ml-auto text-[11px] text-muted">推荐 20 (LTS)</div>
      </div>
      <el-input
        v-model="version"
        placeholder="例如: 20.11.0"
        size="large"
        :disabled="loading || done"
      >
        <template #prefix>
          <span class="text-muted font-mono text-sm">v</span>
        </template>
      </el-input>
    </div>

    <div
      v-if="jobId || done"
      class="flex items-center gap-3 rounded-lg border p-4"
      :class="
        done
          ? 'bg-green-50 dark:bg-green-500/10 border-green-200 dark:border-green-500/20'
          : 'bg-blue-50 dark:bg-blue-500/10 border-blue-200 dark:border-blue-500/20'
      "
    >
      <div
        :class="[
          done
            ? 'i-ep-check text-green-500'
            : 'i-ep-loading animate-spin text-blue-500',
          'text-xl flex-shrink-0',
        ]"
      ></div>
      <div class="flex-1 min-w-0">
        <div
          class="text-xs font-bold"
          :class="
            done
              ? 'text-green-600 dark:text-green-400'
              : 'text-blue-600 dark:text-blue-400'
          "
        >
          {{ done ? "Node.js 环境创建任务已提交 ✓" : "正在创建 Node.js 环境..." }}
        </div>
        <div class="mt-0.5 truncate text-[11px] text-muted">{{ status }}</div>
      </div>
    </div>

    <div class="flex gap-3">
      <el-button class="flex-1 !h-11 !rounded-lg" :disabled="loading" @click="$emit('back')">
        <div class="i-ep-arrow-left mr-1"></div>
        返回
      </el-button>
      <el-button
        v-if="!done"
        type="primary"
        class="flex-1 !h-11 !rounded-lg !font-semibold"
        :loading="loading"
        :disabled="!version.trim()"
        @click="$emit('create')"
      >
        创建并继续
      </el-button>
      <el-button
        v-else
        type="success"
        class="flex-1 !h-11 !rounded-lg !font-semibold"
        @click="$emit('finish')"
      >
        完成初始化
        <div class="i-ep-check ml-1"></div>
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  done: boolean;
  jobId: string | null;
  loading: boolean;
  status: string;
}>();

defineEmits<{
  (e: "back"): void;
  (e: "create"): void;
  (e: "finish"): void;
}>();

const version = defineModel<string>("version", { required: true });
</script>
