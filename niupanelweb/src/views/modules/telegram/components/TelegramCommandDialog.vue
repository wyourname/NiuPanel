<template>
  <ResponsiveDialog
    v-model:visible="visible"
    :title="form.id ? '编辑指令' : '新建指令'"
    desktop-size="sm"
    content-preset="form"
    append-to-body
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
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import type { TelegramCommandForm } from "../composables/useTelegramCommands";
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";

defineEmits<{
  (e: "save"): void;
}>();

const visible = defineModel<boolean>("visible", { required: true });
const form = defineModel<TelegramCommandForm>("form", { required: true });
</script>
