<template>
  <el-dialog
    v-model="visible"
    :title="form.id ? '编辑指令' : '新建指令'"
    :width="isMobile ? '95%' : '480px'"
    class="om-dialog"
  >
    <el-form :model="form" label-position="top">
      <el-form-item label="指令名称 (不含/)">
        <el-input v-model="form.name" placeholder="例如: restart_nginx" />
      </el-form-item>
      <el-form-item label="Shell 脚本">
        <el-input
          v-model="form.script"
          type="textarea"
          :rows="4"
          placeholder="systemctl restart nginx"
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
import type { TelegramCommandForm } from "../composables/useTelegramCommands";

defineProps<{
  isMobile: boolean;
}>();

defineEmits<{
  (e: "save"): void;
}>();

const visible = defineModel<boolean>("visible", { required: true });
const form = defineModel<TelegramCommandForm>("form", { required: true });
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
