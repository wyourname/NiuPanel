<template>
  <ResponsiveDialog
    v-model:visible="visible"
    title="身份验证"
    desktop-size="sm"
    content-preset="form"
    append-to-body
    :close-on-click-modal="false"
  >
    <div class="flex flex-col gap-4">
      <el-alert
        title="系统已向您的 Telegram 机器人下发了6位验证码"
        type="info"
        :closable="false"
        show-icon
      />
      <el-input
        v-model="code"
        placeholder="请输入6位验证码"
        maxlength="6"
        type="text"
        class="!h-11 font-mono text-center text-lg font-bold"
        @keyup.enter="$emit('verify')"
      />
    </div>

    <template #footer>
      <el-button type="primary" :loading="loading" @click="$emit('verify')">
        验证并登录
      </el-button>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";

defineProps<{
  loading: boolean;
}>();

defineEmits<{
  (e: "verify"): void;
}>();

const visible = defineModel<boolean>("visible", { required: true });
const code = defineModel<string>("code", { required: true });
</script>
