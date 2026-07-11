<template>
  <component
    :is="appStore.isMobile ? 'el-drawer' : 'el-dialog'"
    v-model="visible"
    :title="appStore.isMobile ? '创建环境' : ''"
    :size="appStore.isMobile ? 'auto' : '500px'"
    :width="appStore.isMobile ? '90%' : '500px'"
    :align-center="!appStore.isMobile"
    direction="btt"
    destroy-on-close
    append-to-body
    class="log-modal"
  >
    <div class="flex-1 flex flex-col bg-[var(--editor-bg)] overflow-hidden">
      <!-- Internal Header -->
      <div
        class="flex items-center justify-between px-4 shrink-0 border-b border-[var(--editor-border)] h-12"
      >
        <div class="flex items-center gap-4 overflow-hidden">
          <button
            v-if="!appStore.isMobile"
            type="button"
            class="h-8 w-8 rounded-md text-muted flex-center transition-colors hover:bg-black/5 hover:text-default dark:hover:bg-white/5"
            title="关闭"
            aria-label="关闭创建环境对话框"
            @click="visible = false"
          >
            <div class="i-ep-close text-lg"></div>
          </button>
          <span class="text-xs font-bold text-[var(--editor-text)] truncate">
            {{ appStore.isMobile ? "" : "创建环境" }}
          </span>
        </div>
      </div>

      <div class="p-6 flex flex-col gap-6">
        <el-form :model="form" :rules="rules" ref="formRef" label-position="top">
          <!-- Type Selector -->
          <el-form-item label="环境类型" prop="envType">
            <div class="flex gap-2 w-full">
              <button
                v-for="t in envTypeOptions"
                :key="t.value"
                class="flex-1 rounded-md border py-2 text-xs font-bold transition-colors"
                :class="form.envType === t.value
                  ? 'bg-primary border-primary text-white shadow-sm'
                  : 'border-gray-200 dark:border-gray-700 text-muted hover:border-primary/50 hover:text-default'"
                @click="form.envType = t.value"
                type="button"
              >{{ t.label }}</button>
            </div>
          </el-form-item>

          <!-- Version Input -->
          <el-form-item :label="form.envType === 'python' ? 'Python 版本' : 'Node.js 版本'" prop="version">
            <el-input
              v-model="form.version"
              :placeholder="form.envType === 'python' ? '例如: 3.10, 3.11' : '例如: 20.11.0, 22.0.0'"
              size="large"
              class="modern-input"
            >
              <template #prefix>v</template>
            </el-input>
            <div class="text-[10px] text-muted mt-2 opacity-60">
              <span v-if="form.envType === 'python'">通过 uv 工具自动拉取对应 Python 版本并创建虚拟环境。</span>
              <span v-else>通过 fnm 下载指定 Node.js 版本，依赖会安装到该版本共享目录。</span>
            </div>
          </el-form-item>
        </el-form>
        <div class="flex justify-end gap-3">
          <el-button @click="visible = false" size="small">取消</el-button>
          <el-button type="primary" @click="handleSubmit" :loading="loading" size="small" class="!px-6"
            >{{ form.envType === 'node' ? '下载安装' : '创建' }}</el-button>
        </div>
      </div>
    </div>
  </component>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from "vue";
import { ElMessage, type FormInstance } from "element-plus";
import * as envApi from "../../../../api/environment";
import { useAppStore } from "../../../../stores/app";
import type { InstallableEnvType } from "@/types";

const envTypeOptions = [
  { label: "Python", value: "python" },
  { label: "Node.js", value: "node" },
] as const;

const props = withDefaults(
  defineProps<{
    modelValue?: boolean;
    defaultEnvType?: InstallableEnvType;
  }>(),
  {
    modelValue: false,
    defaultEnvType: "python",
  },
);

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
  (event: "show-log", id: number | string, name: string): void;
}>();
const appStore = useAppStore();

const visible = ref(false);
const loading = ref(false);
const form = reactive<{ version: string; envType: InstallableEnvType }>({
  version: "",
  envType: "python",
});
const formRef = ref<FormInstance | null>(null);
const rules = {
  version: [{ required: true, message: "请输入版本号", trigger: "blur" }],
};

watch(
  () => props.modelValue,
  (val: boolean) => {
    visible.value = val;
    if (val) {
      form.version = "";
      form.envType = props.defaultEnvType;
    }
  },
);

watch(visible, (val: boolean) => emit("update:modelValue", val));

const handleSubmit = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async (valid) => {
    if (!valid) return;
    loading.value = true;
    try {
      const res = await envApi.createEnvironment(
        { version: form.version },
        form.envType,
      );
      ElMessage.success("指令已发送，正在后台创建...");
      emit(
        "show-log",
        res.data,
        `${form.envType === "node" ? "安装 Node.js" : "创建 Python"} ${form.version}`,
      );
      visible.value = false;
    } catch (error) {
    } finally {
      loading.value = false;
    }
  });
};
</script>
