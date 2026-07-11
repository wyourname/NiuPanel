<template>
  <ResponsiveDialog
    v-model:visible="visibleValue"
    title="从 URL 导入"
    width="460px"
    append-to-body
    destroy-on-close
  >
    <div class="flex flex-col gap-5 p-5 md:p-6">
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-position="top"
        @submit.prevent="submit"
      >
        <el-form-item prop="url" class="!mb-0">
          <el-input
            v-model="form.url"
            placeholder="粘贴脚本链接或 Git 地址 (HTTP/HTTPS)"
            clearable
            size="large"
            class="modern-input"
            @keyup.enter="submit"
          >
            <template #prefix>
              <div class="i-ep-link text-[14px] text-primary opacity-50"></div>
            </template>
          </el-input>
        </el-form-item>
      </el-form>

      <div class="flex items-start gap-3 rounded-md border border-light bg-soft p-4">
        <div class="accent-subtle h-8 w-8 shrink-0 rounded-md flex-center">
          <div class="i-ep-info-filled"></div>
        </div>
        <p class="text-[10px] font-medium leading-relaxed text-muted">
          请输入脚本的公开访问地址，系统将自动分析运行环境并尝试下载执行。
        </p>
      </div>
    </div>

    <template #footer>
      <div class="flex gap-3 w-full">
        <ToolbarButton block :disabled="creating" @click="visibleValue = false">取消</ToolbarButton>
        <ToolbarButton
          block
          variant="primary"
          :disabled="creating"
          @click="submit"
        >
          {{ creating ? "正在分析..." : "开始导入" }}
        </ToolbarButton>
      </div>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { reactive, ref, watch, computed } from "vue";
import type { FormInstance, FormRules } from "element-plus";
import ResponsiveDialog from "../common/ResponsiveDialog.vue";
import ToolbarButton from "../common/ToolbarButton.vue";

const props = defineProps<{
  creating: boolean;
  url: string;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "submit", url: string): void;
  (event: "update:url", url: string): void;
  (event: "update:visible", visible: boolean): void;
}>();

const formRef = ref<FormInstance | null>(null);
const form = reactive({ url: "" });
const rules: FormRules = {
  url: [
    { required: true, message: "请输入 URL", trigger: "blur" },
    {
      validator: (_rule, value: string, callback) => {
        try {
          const url = new URL(value);
          if (!["http:", "https:"].includes(url.protocol)) {
            callback(new Error("仅支持 HTTP/HTTPS URL"));
            return;
          }
          callback();
        } catch {
          callback(new Error("请输入有效 URL"));
        }
      },
      trigger: "blur",
    },
  ],
};

const visibleValue = computed({
  get: () => props.visible,
  set: (value: boolean) => emit("update:visible", value),
});

watch(
  () => props.url,
  (url) => {
    form.url = url;
  },
  { immediate: true },
);

watch(
  () => form.url,
  (url) => emit("update:url", url),
);

const submit = async () => {
  if (!formRef.value) return;
  const valid = await formRef.value.validate();
  if (!valid) return;
  emit("submit", form.url);
};
</script>
