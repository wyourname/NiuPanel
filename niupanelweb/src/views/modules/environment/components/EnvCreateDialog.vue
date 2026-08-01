<template>
  <ResponsiveDialog
    v-model:visible="visible"
    title="创建运行环境"
    width="520px"
    size="auto"
    destroy-on-close
    append-to-body
  >
    <form class="flex min-h-0 flex-col" @submit.prevent="handleSubmit">
      <div class="flex min-h-0 flex-col gap-5 p-5 sm:p-6">
        <div class="flex items-start gap-3 rounded-lg border border-light bg-soft/50 p-3.5">
          <span class="h-8 w-8 shrink-0 rounded-md accent-subtle flex-center">
            <span :class="envTypeMeta.icon" class="text-[16px]"></span>
          </span>
          <div class="min-w-0">
            <p class="text-[12px] font-semibold text-default">{{ envTypeMeta.title }}</p>
            <p class="mt-0.5 text-[10px] leading-4 text-secondary">{{ envTypeMeta.description }}</p>
          </div>
        </div>

        <el-form ref="formRef" :model="form" :rules="rules" label-position="top">
          <el-form-item label="环境类型" prop="envType" class="!mb-5">
            <div class="grid w-full grid-cols-2 gap-2" role="radiogroup" aria-label="环境类型">
              <button
                v-for="t in envTypeOptions"
                :key="t.value"
                type="button"
                role="radio"
                :aria-checked="form.envType === t.value"
                class="flex min-w-0 items-center gap-2 rounded-lg border p-3 text-left transition-colors"
                :class="form.envType === t.value
                  ? 'border-primary bg-primary/10 text-primary ring-1 ring-primary/20'
                  : 'border-light bg-card text-secondary hover:border-primary/40 hover:bg-soft hover:text-default'"
                @click="form.envType = t.value"
              >
                <span :class="t.icon" class="text-[18px]"></span>
                <span class="min-w-0">
                  <span class="block text-[11px] font-semibold">{{ t.label }}</span>
                  <span class="mt-0.5 block truncate text-[9px] font-medium opacity-70">{{ t.hint }}</span>
                </span>
              </button>
            </div>
          </el-form-item>

          <el-form-item :label="envTypeMeta.versionLabel" prop="version" class="!mb-0">
            <el-input
              v-model.trim="form.version"
              :placeholder="envTypeMeta.placeholder"
              size="large"
              autocomplete="off"
              class="modern-input"
            >
              <template #prefix>v</template>
            </el-input>
            <p class="mt-2 text-[10px] leading-4 text-muted">{{ envTypeMeta.help }}</p>
          </el-form-item>
        </el-form>
      </div>
    </form>

    <template #footer>
      <div class="flex w-full gap-3">
        <el-button class="h-9 flex-1 !rounded-md sm:flex-none" :disabled="loading" @click="visible = false">取消</el-button>
        <el-button native-type="submit" type="primary" :loading="loading" class="h-9 flex-1 !rounded-md !px-6 sm:flex-none" @click="handleSubmit">
          {{ form.envType === 'node' ? '下载安装' : '创建环境' }}
        </el-button>
      </div>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed, ref, reactive, watch } from "vue";
import { ElMessage, type FormInstance } from "element-plus";
import * as envApi from "../../../../api/environment";
import ResponsiveDialog from "../../../../components/common/ResponsiveDialog.vue";
import type { InstallableEnvType } from "@/types";

const envTypeOptions = [
  { label: "Python", value: "python", icon: "i-logos-python", hint: "uv 虚拟环境" },
  { label: "Node.js", value: "node", icon: "i-logos-nodejs-icon", hint: "独立运行时" },
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
const envTypeMeta = computed(() =>
  form.envType === "node"
    ? {
        description: "下载指定 Node.js 版本，并为该版本准备共享依赖目录。",
        help: "例如 20.11.0、22.0.0。完成后可在任务中选择该运行时。",
        icon: "i-logos-nodejs-icon",
        placeholder: "例如 20.11.0",
        title: "安装 Node.js 运行时",
        versionLabel: "Node.js 版本",
      }
    : {
        description: "通过 uv 下载 Python 解释器并创建隔离的虚拟环境。",
        help: "例如 3.10、3.11。系统会自动准备解释器与虚拟环境。",
        icon: "i-logos-python",
        placeholder: "例如 3.11",
        title: "创建 Python 环境",
        versionLabel: "Python 版本",
      },
);

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
