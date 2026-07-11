<template>
  <div class="flex flex-col items-center text-center gap-6 py-4">
    <div class="h-14 w-14 rounded-lg bg-emerald-500/10 text-emerald-600 flex-center dark:text-emerald-300">
      <div class="i-ep-circle-check-filled text-2xl"></div>
    </div>
    <div>
      <h2 class="text-2xl font-bold text-default">初始化完成！</h2>
      <p class="text-muted mt-2 text-sm">
        您的 Python 和 Node.js 环境已开始创建。<br />
        您现在可以开始使用 NiuPanel 管理脚本了。
      </p>
    </div>
    <div class="w-full grid grid-cols-1 gap-2">
      <div
        class="flex items-center gap-3 rounded-lg border border-emerald-200 bg-emerald-50 p-3 dark:border-emerald-500/20 dark:bg-emerald-500/10"
      >
        <div class="i-ep-check text-green-500 text-base flex-shrink-0"></div>
        <div class="text-xs font-medium text-default">
          Python {{ hasPython ? "环境已存在" : `${pythonVersion} 虚拟环境` }}
        </div>
      </div>
      <div
        class="flex items-center gap-3 rounded-lg border border-emerald-200 bg-emerald-50 p-3 dark:border-emerald-500/20 dark:bg-emerald-500/10"
      >
        <div class="i-ep-check text-green-500 text-base flex-shrink-0"></div>
        <div class="text-xs font-medium text-default">
          Node.js {{ hasNode ? "环境已存在" : `${nodeVersion} 沙盒环境` }}
        </div>
      </div>
    </div>
    <div v-if="!(hasPython && hasNode)" class="text-[11px] text-muted">
      环境正在后台构建中，可在“环境管理”页面查看进度
    </div>
    <el-button
      type="primary"
      class="!h-11 !w-full !rounded-lg !text-base !font-semibold"
      @click="$emit('enter')"
    >
      进入面板
      <div class="i-ep-arrow-right ml-2"></div>
    </el-button>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  hasNode: boolean;
  hasPython: boolean;
  nodeVersion: string;
  pythonVersion: string;
}>();

defineEmits<{
  (e: "enter"): void;
}>();
</script>
