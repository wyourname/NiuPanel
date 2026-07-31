<template>
  <div class="flex flex-col items-center gap-6 text-center">
    <div
      class="h-14 w-14 rounded-lg bg-soft text-primary flex-center"
    >
      <div class="i-ep-setting text-2xl"></div>
    </div>
    <div>
      <h1 class="text-2xl font-bold text-default">欢迎来到 NiuPanel</h1>
      <p class="text-muted mt-2 text-sm leading-relaxed">
        在开始使用之前，我们需要帮您完成一些基础环境的初始化设置。<br />
        这将创建一个 <strong class="text-default">Python 虚拟环境</strong>
        和一个 <strong class="text-default">Node.js 环境</strong>，以便脚本正常运行。
      </p>
    </div>
    <div class="grid w-full grid-cols-1 overflow-hidden rounded-lg border border-light text-left sm:grid-cols-2">
      <div
        v-for="feature in features"
        :key="feature.title"
        class="border-b border-light p-3 last:border-b-0 sm:[&:nth-child(odd)]:border-r sm:[&:nth-last-child(-n+2)]:border-b-0"
      >
        <div class="flex items-start gap-3">
          <div :class="[feature.icon, 'mt-0.5 shrink-0 text-lg', feature.color]"></div>
          <div class="min-w-0">
        <div class="text-xs font-bold text-default">{{ feature.title }}</div>
            <div class="mt-0.5 text-[11px] text-muted">{{ feature.desc }}</div>
          </div>
        </div>
      </div>
    </div>
    <el-button
      type="primary"
      class="!h-11 !w-full !rounded-lg !text-base !font-semibold"
      :loading="checkingEnv"
      @click="$emit('start')"
    >
      {{ checkingEnv ? "正在检查环境..." : "开始初始化" }}
      <div v-if="!checkingEnv" class="i-ep-arrow-right ml-2"></div>
    </el-button>
  </div>
</template>

<script setup lang="ts">
import type { OnboardingFeature } from "../composables/useOnboardingFlow";

defineProps<{
  checkingEnv: boolean;
  features: OnboardingFeature[];
}>();

defineEmits<{
  (e: "start"): void;
}>();
</script>
