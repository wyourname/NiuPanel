<template>
  <ResponsiveDialog
    v-model:visible="visible"
    title="服务器设置"
    desktop-size="sm"
    content-preset="form"
    append-to-body
  >
    <div class="flex flex-col gap-4">
      <el-alert
        title="App 模式下需配置服务器地址"
        type="info"
        :closable="false"
        show-icon
      />
      <el-form label-position="top">
        <el-form-item label="服务器地址 (URL)">
          <el-input
            v-model="serverUrl"
            placeholder="http://192.168.1.x:7788"
          />
          <div class="text-xs text-muted mt-1">
            请输入完整的后端地址，包含 http/https 和端口。
          </div>
        </el-form-item>
      </el-form>
    </div>
    <template #footer>
      <span class="dialog-footer">
        <el-button @click="visible = false">取消</el-button>
        <el-button type="primary" @click="$emit('save')">
          保存并刷新
        </el-button>
      </span>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";

defineEmits<{
  (e: "save"): void;
}>();

const visible = defineModel<boolean>("visible", { required: true });
const serverUrl = defineModel<string>("serverUrl", { required: true });
</script>
