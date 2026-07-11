<template>
  <div class="flex h-full min-h-0 flex-col bg-card">
    <div class="flex h-16 shrink-0 items-center justify-between border-b border-light/80 px-6">
      <div class="flex flex-col">
        <h2 class="text-lg font-bold text-default">
          {{ editingId ? "编辑变量" : "新增变量" }}
        </h2>
        <span class="text-[11px] font-medium text-muted mt-0.5">
          {{ editingId ? "修改现有的环境变量配置" : "添加一个新的全局或局部环境变量" }}
        </span>
      </div>
      <div class="flex items-center gap-3">
        <el-switch
          v-model="form.enabled"
          inline-prompt
          active-text="开"
          inactive-text="关"
        />
        <el-button @click="emit('cancel')">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="submit">
          {{ submitting ? "保存中..." : "保存配置" }}
        </el-button>
      </div>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto p-6 custom-scrollbar">
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-position="top"
        class="flex flex-col gap-6"
      >
        <div class="grid grid-cols-1 md:grid-cols-2 gap-x-6">
          <el-form-item prop="key">
            <template #label>
              <span class="label-xs block mb-1">变量键名</span>
            </template>
            <el-input
              v-model="form.key"
              placeholder="例如: API_ENDPOINT"
              class="font-mono text-sm"
            />
          </el-form-item>

          <el-form-item v-if="form.scope === 'Script'" prop="scope_ids">
            <template #label>
              <span class="label-xs block mb-1">关联任务</span>
            </template>
            <el-select
              v-model="form.scope_ids"
              placeholder="分配到具体任务"
              class="w-full"
              filterable
              multiple
              collapse-tags
              collapse-tags-tooltip
            >
              <el-option
                v-for="item in tasks"
                :key="item.id"
                :label="item.name"
                :value="item.id"
              >
                <span class="float-left font-bold text-xs">{{ item.name }}</span>
                <span class="float-right text-gray-400 text-[10px] font-mono">#{{ item.id }}</span>
              </el-option>
            </el-select>
          </el-form-item>
        </div>

        <div class="rounded-md border border-base bg-gray-50 p-2 dark:bg-gray-800/20">
          <div class="flex items-center gap-2 mb-2 px-1 pt-1">
            <div class="i-ep-lock text-primary opacity-50 text-sm"></div>
            <span class="text-[10px] font-bold text-muted">
              变量取值
            </span>
          </div>
          <el-form-item prop="value" class="!mb-0">
            <el-input
              v-model="form.value"
              type="textarea"
              :rows="10"
              placeholder="在此输入 Token、路径或配置值"
              class="font-mono text-sm border-none"
            />
          </el-form-item>
        </div>

        <el-form-item prop="remarks">
          <template #label>
            <span class="label-xs block mb-1">备注说明</span>
          </template>
          <el-input
            v-model="form.remarks"
            type="textarea"
            :rows="2"
            placeholder="简要说明此变量用途..."
          />
        </el-form-item>
      </el-form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import type { FormInstance, FormRules } from "element-plus";
import type { VariableFormState } from "../../composables/useVariableForm";

type VariableFormTask = {
  id: number;
  name: string;
};

defineProps<{
  editingId: number | null;
  form: VariableFormState;
  rules: FormRules;
  submitting: boolean;
  tasks: VariableFormTask[];
}>();

const emit = defineEmits<{
  (event: "cancel"): void;
  (event: "submit"): void;
}>();

const formRef = ref<FormInstance | null>(null);

const submit = async () => {
  if (!formRef.value) return;

  const valid = await formRef.value.validate().catch(() => false);
  if (!valid) return;

  emit("submit");
};
</script>

<style scoped>
:deep(.el-input__wrapper),
:deep(.el-textarea__inner) {
  background-color: var(--el-fill-color-light) !important;
  box-shadow: none !important;
  border: 1px solid transparent;
  transition: all 0.2s;
}

:deep(.el-input__wrapper:hover),
:deep(.el-textarea__inner:hover) {
  background-color: var(--el-fill-color) !important;
}

:deep(.el-input__wrapper.is-focus),
:deep(.el-textarea__inner:focus) {
  background-color: var(--el-bg-color) !important;
  box-shadow: 0 0 0 1px var(--el-color-primary) !important;
  border-color: var(--el-color-primary);
}

:deep(.el-textarea__inner) {
  padding: 12px 16px;
  line-height: 1.6;
}
</style>
