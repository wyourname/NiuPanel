<template>
  <div class="flex min-h-screen w-full flex-col overflow-hidden bg-base">
    <header class="flex h-14 shrink-0 items-center border-b border-light bg-card px-4 sm:px-6">
      <div class="flex items-center gap-2.5">
        <img src="/favicon.png" alt="NiuPanel" class="h-8 w-8 rounded-md" />
        <div>
          <div class="text-sm font-semibold text-default">NiuPanel</div>
          <div class="text-[10px] text-muted">首次配置</div>
        </div>
      </div>
    </header>

    <main class="flex flex-1 items-center justify-center px-4 py-6 sm:py-8">
      <div class="w-full max-w-[640px]">

        <OnboardingStepProgress :step="step" :steps="steps" />

        <div
          class="rounded-lg border border-light bg-card p-5 shadow-sm sm:p-8"
        >
          <transition name="fade-slide" mode="out-in">
            <OnboardingWelcomeStep
              v-if="step === 0"
              key="step0"
              :checking-env="checkingEnv"
              :features="features"
              @start="startInitialization"
            />
            <OnboardingPythonStep
              v-else-if="step === 1"
              key="step1"
              v-model:version="pythonVersion"
              :done="pythonDone"
              :job-id="pythonJobId"
              :loading="pythonLoading"
              :status="pythonStatus"
              @back="step = 0"
              @create="createPythonEnv"
              @next="goNextFromPython"
            />
            <OnboardingNodeStep
              v-else-if="step === 2"
              key="step2"
              v-model:version="nodeVersion"
              :done="nodeDone"
              :job-id="nodeJobId"
              :loading="nodeLoading"
              :status="nodeStatus"
              @back="goBackFromNode"
              @create="createNodeEnv"
              @finish="handleFinish"
            />
            <OnboardingDoneStep
              v-else
              key="step3"
              :has-node="hasNode"
              :has-python="hasPython"
              :node-version="nodeVersion"
              :python-version="pythonVersion"
              @enter="goToDashboard"
            />
          </transition>
        </div>

        <div v-if="step < 3" class="mt-4 text-center">
          <button type="button" class="cursor-pointer text-xs text-muted transition-colors hover:text-default" @click="skipOnboarding">
            跳过初始化，直接进入面板
          </button>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import OnboardingDoneStep from "./onboarding/components/OnboardingDoneStep.vue";
import OnboardingNodeStep from "./onboarding/components/OnboardingNodeStep.vue";
import OnboardingPythonStep from "./onboarding/components/OnboardingPythonStep.vue";
import OnboardingStepProgress from "./onboarding/components/OnboardingStepProgress.vue";
import OnboardingWelcomeStep from "./onboarding/components/OnboardingWelcomeStep.vue";
import { useOnboardingFlow } from "./onboarding/composables/useOnboardingFlow";

const {
  checkingEnv,
  createNodeEnv,
  createPythonEnv,
  features,
  goBackFromNode,
  goNextFromPython,
  goToDashboard,
  handleFinish,
  hasNode,
  hasPython,
  nodeDone,
  nodeJobId,
  nodeLoading,
  nodeStatus,
  nodeVersion,
  pythonDone,
  pythonJobId,
  pythonLoading,
  pythonStatus,
  pythonVersion,
  skipOnboarding,
  startInitialization,
  step,
  steps,
} = useOnboardingFlow();
</script>

<style scoped>
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: opacity 0.3s ease, transform 0.3s ease;
}
.fade-slide-enter-from {
  opacity: 0;
}
.fade-slide-leave-to {
  opacity: 0;
}
</style>
