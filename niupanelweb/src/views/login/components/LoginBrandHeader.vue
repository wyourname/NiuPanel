<template>
  <div class="mb-8 flex flex-col items-center justify-center text-center">
    <div
      v-if="resetMode"
      class="mb-4 h-12 w-12 rounded-lg bg-amber-500/10 text-amber-600 flex-center dark:text-amber-300"
    >
      <el-icon size="22"><Lock /></el-icon>
    </div>
    <img
      v-else
      src="/favicon.png"
      alt="NiuPanel"
      class="mb-4 h-14 w-14 rounded-lg"
    />
    <h1 class="text-xl font-bold text-default">
      {{ resetMode ? "重置密码" : "NiuPanel" }}
    </h1>
    <p class="text-muted mt-2 text-sm">
      {{ subtitle }}
    </p>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Lock } from "@element-plus/icons-vue";

const props = defineProps<{
  isInitialized: boolean;
  resetMode: boolean;
}>();

const subtitle = computed(() => {
  if (props.resetMode) return "请为您账户设置一个新的安全密码";
  return props.isInitialized ? "欢迎回来" : "初始化系统";
});
</script>
