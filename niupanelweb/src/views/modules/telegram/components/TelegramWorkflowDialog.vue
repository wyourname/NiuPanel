<template>
  <el-dialog
    v-model="visible"
    :title="form.id ? '编辑自动化' : '新建自动化'"
    :width="isMobile ? '95%' : '480px'"
    class="om-dialog"
  >
    <el-form :model="form" label-position="top">
      <el-form-item label="触发条件">
        <el-select v-model="form.event_type" class="w-full">
          <el-option label="任务失败" value="failed" />
          <el-option label="任务成功" value="success" />
          <el-option label="系统警报 (CPU/内存超载)" value="alert" />
          <el-option label="定时触发 (Cron)" value="cron" />
        </el-select>
      </el-form-item>
      <el-form-item label="执行动作">
        <el-select v-model="form.action_type" class="w-full">
          <el-option label="Telegram 消息通知" value="notify" />
          <el-option label="执行 Shell 命令" value="shell" />
          <el-option label="请求人工审批 (交互式)" value="approval" />
        </el-select>
      </el-form-item>
      <el-form-item :label="actionConfigLabel" class="mt-2">
        <el-input
          v-model="form.config_json"
          type="textarea"
          :rows="3"
          :placeholder="actionConfigPlaceholder"
          class="font-mono text-sm"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" @click="$emit('save')">保存</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import type { TelegramWorkflowForm } from "../composables/useTelegramWorkflows";

defineProps<{
  actionConfigLabel: string;
  actionConfigPlaceholder: string;
  isMobile: boolean;
}>();

defineEmits<{
  (e: "save"): void;
}>();

const visible = defineModel<boolean>("visible", { required: true });
const form = defineModel<TelegramWorkflowForm>("form", { required: true });
</script>

<style scoped>
@media (max-width: 768px) {
  .om-dialog :deep(.el-dialog__body) {
    padding: 15px !important;
  }

  .om-dialog :deep(.el-dialog__header) {
    margin-right: 0;
    padding-bottom: 10px;
  }
}
</style>
