<template>
  <div class="flex flex-col gap-4">
    <div
      class="flex gap-3 rounded-md border border-blue-100 bg-blue-50 p-3 dark:border-blue-800/30 dark:bg-blue-900/10"
    >
      <div class="i-ep-info-filled text-blue-500 text-lg shrink-0"></div>
      <div class="text-[11px] leading-relaxed text-blue-600 dark:text-blue-300">
        解析远程配置包后，您可以选择性保存其中的脚本和任务。请确保来源可靠。
      </div>
    </div>

    <div class="flex flex-col gap-4">
      <el-form label-position="top">
        <el-form-item label="分享链接 / Token" required>
          <el-input
            :model-value="url"
            placeholder="粘贴链接或 Token..."
            autofocus
            @keyup.enter="emit('submit')"
            @update:model-value="emit('update:url', String($event))"
          >
            <template #prefix>
              <div class="i-ep-link text-muted"></div>
            </template>
          </el-input>
        </el-form-item>

        <el-form-item label="提取密码 (可选)">
          <el-input
            :model-value="password"
            type="password"
            show-password
            placeholder="输入提取密码"
            @keyup.enter="emit('submit')"
            @update:model-value="emit('update:password', String($event))"
          >
            <template #prefix>
              <div class="i-ep-lock text-muted"></div>
            </template>
          </el-input>
        </el-form-item>
      </el-form>

      <el-button
        type="primary"
        class="w-full !h-9 !rounded-md font-bold"
        :loading="downloading"
        :disabled="!url"
        @click="emit('submit')"
      >
        解析资源
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  downloading: boolean;
  password: string;
  url: string;
}>();

const emit = defineEmits<{
  (event: "submit"): void;
  (event: "update:password", password: string): void;
  (event: "update:url", url: string): void;
}>();
</script>

<style scoped>
:deep(.el-form-item) {
  margin-bottom: 12px;
}

:deep(.el-form-item__label) {
  font-size: 12px;
  font-weight: 600;
  margin-bottom: 4px !important;
  line-height: 1.2;
}
</style>
