<template>
  <ResponsiveDialog
    v-model:visible="visible"
    :title="title"
    desktop-size="sm"
    content-preset="form"
    append-to-body
    :close-on-click-modal="false"
  >
    <div class="flex flex-col gap-4">
      <div class="flex justify-between px-1 mb-2">
        <div
          v-for="i in 3"
          :key="i"
          class="h-1 flex-1 mx-0.5 rounded-full transition-all duration-500"
          :class="
            step >= i - 1
              ? 'bg-primary'
              : 'bg-gray-200 dark:bg-dark-700'
          "
        ></div>
      </div>

      <el-form label-position="top">
        <div v-if="step === 0" class="animate-fade-in">
          <el-alert
            title="请输入您需要找回密码的账户名"
            type="info"
            :closable="false"
            show-icon
            class="mb-4"
          />
          <el-form-item label="账户名">
            <el-input
              v-model="username"
              placeholder="请输入用户名"
              prefix-icon="User"
              @keyup.enter="$emit('identify')"
            />
          </el-form-item>
        </div>

        <div v-else-if="step === 1" class="animate-fade-in">
          <el-alert
            title="身份确认成功！请输入绑定邮箱的完整前缀以发送验证码。"
            type="success"
            :closable="false"
            show-icon
            class="mb-4"
          />
          <el-form-item label="绑定邮箱">
            <div class="flex items-center">
              <el-input
                v-model="emailPrefix"
                type="email"
                placeholder="完整邮箱地址"
                class="flex-1 font-mono"
                @keyup.enter="$emit('sendCode')"
              />
              <div
                v-if="emailSuffix"
                class="bg-base border border-l-0 border-base px-3 h-10 flex items-center text-secondary font-mono rounded-r-lg"
              >
                @{{ emailSuffix }}
              </div>
            </div>
          </el-form-item>
        </div>

        <div v-else-if="step === 2" class="animate-fade-in">
          <el-alert
            title="验证码已发送，请在 10 分钟内完成核验。"
            type="success"
            :closable="false"
            show-icon
            class="mb-4"
          />
          <el-form-item label="6位验证码">
            <div class="flex gap-2">
              <el-input
                v-model="code"
                placeholder="混合码"
                maxlength="6"
                class="text-center font-mono text-lg font-bold"
                @keyup.enter="$emit('verifyCode')"
              />
              <el-button
                :disabled="countdown > 0"
                class="min-w-[100px]"
                @click="$emit('sendCode')"
              >
                {{ countdown > 0 ? `${countdown}s` : "重发" }}
              </el-button>
            </div>
          </el-form-item>
        </div>
      </el-form>
    </div>
    <template #footer>
      <div class="flex justify-between items-center w-full">
        <el-button link @click="goBack">
          {{ step > 0 ? "上一步" : "取消" }}
        </el-button>

        <div class="flex gap-2">
          <el-button
            v-if="step === 0"
            type="primary"
            :loading="identifying"
            :disabled="!username"
            @click="$emit('identify')"
          >
            下一步
          </el-button>
          <el-button
            v-if="step === 1"
            type="primary"
            :loading="sendingEmail"
            :disabled="!emailPrefix"
            @click="$emit('sendCode')"
          >
            发送验证码
          </el-button>
          <el-button
            v-if="step === 2"
            type="primary"
            :loading="verifyingCode"
            :disabled="code.length !== 6"
            @click="$emit('verifyCode')"
          >
            核验并重置
          </el-button>
        </div>
      </div>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";

defineProps<{
  countdown: number;
  emailSuffix: string;
  identifying: boolean;
  sendingEmail: boolean;
  title: string;
  verifyingCode: boolean;
}>();

const emit = defineEmits<{
  (e: "identify"): void;
  (e: "resetState"): void;
  (e: "sendCode"): void;
  (e: "verifyCode"): void;
}>();

const visible = defineModel<boolean>("visible", { required: true });
const step = defineModel<number>("step", { required: true });
const username = defineModel<string>("username", { required: true });
const emailPrefix = defineModel<string>("emailPrefix", { required: true });
const code = defineModel<string>("code", { required: true });

const goBack = () => {
  if (step.value > 0) {
    step.value -= 1;
    return;
  }
  emit("resetState");
};
</script>
