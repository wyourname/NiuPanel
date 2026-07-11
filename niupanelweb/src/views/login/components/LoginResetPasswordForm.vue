<template>
  <el-form
    ref="resetFormRef"
    :model="resetForm"
    :rules="rules"
    size="large"
    @submit.prevent="submit"
  >
    <el-form-item prop="password">
      <el-input
        v-model="resetForm.password"
        type="password"
        placeholder="新密码"
        prefix-icon="Lock"
        show-password
        clearable
        class="!h-12"
      />
    </el-form-item>
    <el-form-item prop="confirm">
      <el-input
        v-model="resetForm.confirm"
        type="password"
        placeholder="确认新密码"
        prefix-icon="Check"
        show-password
        clearable
        class="!h-12"
      />
    </el-form-item>

    <div class="mt-8 flex flex-col gap-3">
      <el-button
        type="primary"
        native-type="submit"
        :loading="loading"
        class="!h-11 !w-full !rounded-lg !text-base !font-semibold"
      >
        更新密码
      </el-button>
      <el-button link class="!w-full" @click="$emit('back')">
        返回登录
      </el-button>
    </div>
  </el-form>
</template>

<script setup lang="ts">
import { ref } from "vue";
import type { FormInstance, FormRules } from "element-plus";
import type { ResetForm } from "../composables/usePasswordResetFlow";

defineProps<{
  loading: boolean;
  rules: FormRules<ResetForm>;
}>();

const emit = defineEmits<{
  (e: "back"): void;
  (e: "submit", form: FormInstance | null): void;
}>();

const resetForm = defineModel<ResetForm>("resetForm", { required: true });
const resetFormRef = ref<FormInstance | null>(null);

const submit = () => {
  emit("submit", resetFormRef.value);
};
</script>
