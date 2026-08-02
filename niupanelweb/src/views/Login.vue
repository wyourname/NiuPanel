<template>
  <div class="app-viewport relative flex flex-col overflow-x-hidden overflow-y-auto bg-base">
    <header class="safe-top-header flex shrink-0 items-center justify-between border-b border-light bg-card px-4 pt-safe sm:px-6">
      <div class="flex items-center gap-2.5">
        <img src="/favicon.png" alt="NiuPanel" class="h-8 w-8 rounded-md" />
        <div>
          <div class="text-sm font-semibold text-default">NiuPanel</div>
          <div class="text-[11px] text-muted">面板控制台</div>
        </div>
      </div>
      <button
        type="button"
        class="mobile-touch-target h-8 w-8 cursor-pointer rounded-md text-secondary flex-center transition-colors hover:bg-soft hover:text-default"
        title="服务器设置"
        aria-label="打开服务器设置"
        @click="showServerSettings = true"
      >
        <div class="i-ep-setting text-[16px]"></div>
      </button>
    </header>

    <main class="flex flex-1 items-start justify-center px-4 py-5 sm:items-center sm:py-10">
      <div
        class="w-full max-w-[420px] rounded-lg border border-light bg-card p-5 shadow-sm sm:p-8"
      >
        <LoginBrandHeader
          :is-initialized="isSystemInitialized"
          :reset-mode="Boolean(resetToken)"
        />

        <LoginResetPasswordForm
          v-if="resetToken"
          v-model:reset-form="resetForm"
          :rules="resetRules"
          :loading="loading"
          @back="resetToken = null"
          @submit="handleResetSubmit"
        />

        <LoginCredentialForm
          v-else
          v-model:form="form"
          v-model:show-smtp-config="showSmtpConfig"
          :is-initialized="isSystemInitialized"
          :loading="loading"
          :rules="rules"
          @forgot-password="showForgotDialog = true"
          @submit="handleSubmit"
        />
      </div>
    </main>

    <footer class="shrink-0 py-4 text-center text-xs text-muted select-none">
      &copy; {{ new Date().getFullYear() }} NiuPanel
    </footer>

    <ForgotPasswordDialog
      v-model:code="forgotCode"
      v-model:email-prefix="forgotEmailPrefix"
      v-model:step="forgotStep"
      v-model:username="forgotUsername"
      v-model:visible="showForgotDialog"
      :title="getForgotTitle()"
      :countdown="countdown"
      :email-suffix="forgotEmailSuffix"
      :identifying="identifying"
      :sending-email="sendingEmail"
      :verifying-code="verifyingCode"
      @identify="handleIdentify"
      @reset-state="resetForgotState"
      @send-code="handleForgotSubmit"
      @verify-code="handleVerifyCode"
    />

    <TelegramTwoFactorDialog
      v-model:code="verifyCode"
      v-model:visible="show2faDialog"
      :loading="verifying2fa"
      @verify="handleVerify2FA"
    />

    <ServerSettingsDialog
      v-model:server-url="serverUrl"
      v-model:visible="showServerSettings"
      @save="saveServerSettings"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import ForgotPasswordDialog from "./login/components/ForgotPasswordDialog.vue";
import LoginBrandHeader from "./login/components/LoginBrandHeader.vue";
import LoginCredentialForm from "./login/components/LoginCredentialForm.vue";
import LoginResetPasswordForm from "./login/components/LoginResetPasswordForm.vue";
import ServerSettingsDialog from "./login/components/ServerSettingsDialog.vue";
import TelegramTwoFactorDialog from "./login/components/TelegramTwoFactorDialog.vue";
import { useLoginForm } from "./login/composables/useLoginForm";
import { usePasswordResetFlow } from "./login/composables/usePasswordResetFlow";
import { useServerSettings } from "./login/composables/useServerSettings";
import { useTelegramLogin2fa } from "./login/composables/useTelegramLogin2fa";


const {
  handleVerify2FA,
  openTwoFactorDialog,
  show2faDialog,
  verifyCode,
  verifying2fa,
} = useTelegramLogin2fa();

const {
  form,
  handleSubmit,
  isInitialized,
  loading,
  rules,
  showSmtpConfig,
} = useLoginForm({
  onTwoFactorRequired: openTwoFactorDialog,
});

const {
  countdown,
  forgotCode,
  forgotEmailPrefix,
  forgotEmailSuffix,
  forgotStep,
  forgotUsername,
  getForgotTitle,
  handleForgotSubmit,
  handleIdentify,
  handleResetSubmit,
  handleVerifyCode,
  identifying,
  resetForgotState,
  resetForm,
  resetRules,
  resetToken,
  sendingEmail,
  showForgotDialog,
  verifyingCode,
} = usePasswordResetFlow({ loading });

const {
  saveServerSettings,
  serverUrl,
  showServerSettings,
} = useServerSettings();

const isSystemInitialized = computed(() => Boolean(isInitialized.value));
</script>

<style>
/* Global Transitions */
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition:
    opacity 0.2s ease,
    transform 0.2s ease;
}

.fade-slide-enter-from {
  opacity: 0;
}

.fade-slide-leave-to {
  opacity: 0;
}

.animate-fade-in {
  animation: fadeIn 0.3s ease-out;
}
@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
