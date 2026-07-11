<template>
  <el-form
    ref="formRef"
    :model="form"
    :rules="rules"
    size="large"
    @submit.prevent="submit"
  >
    <el-form-item prop="username">
      <el-input
        v-model="form.username"
        placeholder="用户名"
        prefix-icon="User"
        clearable
        class="!h-12"
      />
    </el-form-item>

    <el-form-item prop="password">
      <el-input
        v-model="form.password"
        type="password"
        placeholder="密码"
        prefix-icon="Lock"
        show-password
        clearable
        class="!h-12"
      />
    </el-form-item>

    <el-form-item v-if="!isInitialized" prop="confirm_password">
      <el-input
        v-model="form.confirm_password"
        type="password"
        placeholder="确认密码"
        prefix-icon="Check"
        show-password
        clearable
        class="!h-12"
      />
    </el-form-item>

    <div v-if="!isInitialized" class="mb-6">
      <div
        class="mb-3 flex cursor-pointer select-none items-center justify-center gap-2 rounded-md border border-dashed border-primary/25 py-2 text-xs font-semibold text-primary/80 transition-colors hover:bg-soft hover:text-primary"
        @click="showSmtpConfig = !showSmtpConfig"
      >
        <div
          :class="showSmtpConfig ? 'i-ep-arrow-down' : 'i-ep-arrow-right'"
        ></div>
        <span>{{
          showSmtpConfig
            ? "隐藏高级配置"
            : "配置找回密码邮箱 (SMTP, 可选)"
        }}</span>
      </div>

      <transition name="el-zoom-in-top">
        <div
          v-if="showSmtpConfig"
          class="animate-fade-in space-y-4 rounded-lg border border-light bg-subtle p-3"
        >
          <el-form-item prop="mail_host" class="!mb-0">
            <el-input
              v-model="form.mail_host"
              placeholder="SMTP 服务器 (如 smtp.qq.com)"
              prefix-icon="Connection"
            />
          </el-form-item>
          <el-form-item prop="mail_username" class="!mb-0">
            <el-input
              v-model="form.mail_username"
              placeholder="SMTP 用户名 (兼找回邮箱)"
              prefix-icon="Message"
            />
          </el-form-item>
          <el-form-item prop="mail_password" class="!mb-0">
            <el-input
              v-model="form.mail_password"
              type="password"
              show-password
              placeholder="SMTP 授权码"
              prefix-icon="Key"
            />
          </el-form-item>
          <div class="text-center text-[10px] text-muted">
            建议配置，以便通过邮箱找回密码
          </div>
        </div>
      </transition>
    </div>

    <div v-if="isInitialized" class="flex justify-end -mt-2 mb-4">
      <el-button
        link
        type="primary"
        size="small"
        @click="$emit('forgotPassword')"
      >
        忘记密码？
      </el-button>
    </div>

    <div class="mt-4">
      <el-button
        type="primary"
        native-type="submit"
        :loading="loading"
        class="!h-11 !w-full !rounded-lg !text-base !font-semibold"
      >
        {{ isInitialized ? "登录" : "创建管理员账号" }}
      </el-button>
    </div>
  </el-form>
</template>

<script setup lang="ts">
import { ref } from "vue";
import type { FormInstance, FormRules } from "element-plus";
import type { LoginForm } from "../composables/useLoginForm";

defineProps<{
  isInitialized: boolean;
  loading: boolean;
  rules: FormRules<LoginForm>;
}>();

const emit = defineEmits<{
  (e: "forgotPassword"): void;
  (e: "submit", form: FormInstance | null): void;
}>();

const form = defineModel<LoginForm>("form", { required: true });
const showSmtpConfig = defineModel<boolean>("showSmtpConfig", {
  required: true,
});
const formRef = ref<FormInstance | null>(null);

const submit = () => {
  emit("submit", formRef.value);
};
</script>
