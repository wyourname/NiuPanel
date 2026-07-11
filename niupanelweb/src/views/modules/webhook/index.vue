<template>
  <div
    class="h-full flex flex-col overflow-y-auto custom-scrollbar p-4 md:p-6 gap-6 max-w-[1000px] mx-auto relative w-full"
  >
    <!-- Header -->
    <div class="shrink-0">
      <h3
        class="flex items-center gap-2 text-lg font-bold text-default"
      >
        <div class="i-ep-notification text-primary"></div>
        Webhook 推送工具
      </h3>
      <p class="text-xs text-muted mt-1">
        手动触发系统通知，或测试第三方集成调用。
      </p>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
      <!-- Left: Form -->
      <div class="md:col-span-2 space-y-6">
        <div class="common-card p-2">
          <el-form :model="form" label-position="top">
            <el-form-item label="通知标题">
              <el-input
                v-model="form.title"
                placeholder="例如：手动运维通知"
                class="modern-input"
              />
            </el-form-item>

            <el-form-item label="通知级别">
              <el-radio-group v-model="form.level" class="modern-radio">
                <el-radio-button label="info">信息</el-radio-button>
                <el-radio-button label="warning">警告</el-radio-button>
                <el-radio-button label="error">错误</el-radio-button>
                <el-radio-button label="success">成功</el-radio-button>
              </el-radio-group>
            </el-form-item>

            <el-form-item label="通知正文">
              <el-input
                v-model="form.content"
                type="textarea"
                :rows="6"
                placeholder="在此输入推送内容..."
                class="modern-input"
              />
            </el-form-item>

            <div class="flex justify-end mt-4">
              <el-button
                type="primary"
                @click="handlePush"
                :loading="loading"
                class="!rounded-md px-8"
              >
                <div class="i-ep-promotion mr-2"></div>
                立即推送
              </el-button>
            </div>
          </el-form>
        </div>
      </div>

      <!-- Right: Integration Info -->
      <div class="space-y-6">
        <div
          class="accent-subtle rounded-md p-5"
        >
          <h4 class="text-sm font-bold mb-3 flex items-center gap-2">
            <div class="i-ep-link"></div>
            API 调用示例
          </h4>
          <p class="text-[11px] leading-relaxed opacity-80 mb-4">
            您可以通过系统 API 密钥在外部脚本中调用此接口：
          </p>

          <div
            class="group relative overflow-x-auto rounded-md bg-dark-900/90 p-3 font-mono text-[10px] text-slate-300"
          >
            <pre><code>POST /api/v1/webhook/push
X-API-Key: npk_xxx

{
  "title": "...",
  "content": "...",
  "level": "info"
}</code></pre>
          </div>
        </div>

        <div class="common-card p-2 border-dashed">
          <h4 class="text-sm font-bold text-default mb-3">注意事项</h4>
          <ul
            class="text-[11px] text-muted space-y-2 list-disc pl-4 leading-relaxed"
          >
            <li>推送内容会遵循您在<b>通知设置</b>中配置的转发规则。</li>
            <li>如果配置了 Webhook URL 或邮件，通知将同步发送至这些终端。</li>
            <li>
              外部集成调用需携带 <code>X-API-Key: [KEY]</code>。
            </li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref } from "vue";
import { ElMessage } from "element-plus";
import * as webhookApi from "@/api/webhook";
import { useHaptics } from "@/composables/useHaptics";

const haptics = useHaptics();
const loading = ref(false);

const form = reactive({
  title: "",
  content: "",
  level: "info",
});

const getErrorMessage = (error: unknown, fallback: string) => {
  return error instanceof Error && error.message ? error.message : fallback;
};

const handlePush = async () => {
  if (!form.title || !form.content) {
    return ElMessage.warning("标题和内容均不能为空");
  }

  loading.value = true;
  haptics.impact();

  try {
    await webhookApi.pushNotification(form);
    ElMessage.success("推送指令已提交");
    // Clear form but keep level
    form.title = "";
    form.content = "";
  } catch (e: unknown) {
    ElMessage.error(getErrorMessage(e, "推送失败"));
  } finally {
    loading.value = false;
  }
};
</script>

<style scoped>
:deep(.modern-input .el-input__wrapper),
:deep(.modern-input .el-textarea__inner) {
  background-color: var(--el-bg-color) !important;
  border-radius: 6px;
  box-shadow: 0 0 0 1px var(--el-border-color) inset;
  transition: box-shadow 0.2s;
  padding: 10px 14px;
}

:deep(.modern-input .el-input__wrapper.is-focus),
:deep(.modern-input .el-textarea__inner:focus) {
  box-shadow: 0 0 0 1px var(--el-color-primary) inset !important;
}

:deep(.modern-radio .el-radio-button__inner) {
  border-radius: 6px !important;
  margin-right: 8px;
  border: 1px solid var(--el-border-color) !important;
  font-size: 12px;
  font-weight: bold;
}

:deep(.modern-radio .el-radio-button:first-child .el-radio-button__inner) {
  border-left: 1px solid var(--el-border-color) !important;
}

:deep(
  .modern-radio
    .el-radio-button__original-radio:checked
    + .el-radio-button__inner
) {
  background-color: var(--el-color-primary) !important;
  border-color: var(--el-color-primary) !important;
  box-shadow: 0 4px 12px var(--el-color-primary-light-7) !important;
}
</style>
