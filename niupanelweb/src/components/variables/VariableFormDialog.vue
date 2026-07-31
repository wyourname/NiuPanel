<template>
  <ResponsiveDialog
    v-model:visible="visibleValue"
    :title="editingId ? '编辑变量配置' : '新建变量'"
    :width="isMobile ? '100%' : '480px'"
    :size="isMobile ? '100%' : 'auto'"
    modal-class="!bg-transparent"
    append-to-body
    destroy-on-close
  >
    <div class="flex flex-1 flex-col gap-4 overflow-y-auto p-4 sm:p-5">
      <el-form ref="formRef" :model="form" label-position="top" :rules="rules">
        <el-form-item v-if="activeTab === 'Script'" prop="scope_ids">
          <template #label>
            <span class="label-xs">关联任务</span>
          </template>
          <el-select
            v-model="form.scope_ids"
            placeholder="分配到具体任务"
            class="w-full modern-input"
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

        <el-form-item prop="key">
          <template #label>
            <span class="label-xs">变量键名</span>
          </template>
          <el-input
            v-model="form.key"
            :maxlength="128"
            autocomplete="off"
            placeholder="例如: API_ENDPOINT"
            class="modern-input font-mono !text-xs"
          />
        </el-form-item>

        <el-form-item prop="value">
          <template #label>
            <span class="label-xs">变量取值</span>
          </template>
          <el-input
            v-model="form.value"
            type="textarea"
            :maxlength="1048576"
            :rows="isMobile ? 6 : 6"
            :autosize="{ minRows: isMobile ? 6 : 5, maxRows: isMobile ? 12 : 12 }"
            placeholder="在此输入 Token、路径或配置值"
            class="modern-input font-mono !text-xs"
          />
        </el-form-item>

        <el-form-item prop="remarks">
          <template #label>
            <span class="label-xs">备注说明</span>
          </template>
          <el-input
            v-model="form.remarks"
            :maxlength="2000"
            placeholder="简要说明此变量用途"
            class="modern-input"
          />
        </el-form-item>

        <div class="flex items-center justify-between rounded-md border border-light bg-subtle px-3 py-2.5">
          <span class="text-[13px] font-semibold text-default">立即启用该变量</span>
          <el-switch v-model="form.enabled" />
        </div>
      </el-form>
    </div>

    <template #footer>
      <div class="flex w-full gap-2">
        <ToolbarButton block :disabled="submitting" @click="visibleValue = false">取消</ToolbarButton>
        <ToolbarButton block variant="primary" :disabled="submitting" @click="submit">
          {{ submitting ? "保存中..." : editingId ? "保存修改" : "确认创建" }}
        </ToolbarButton>
      </div>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import type { FormInstance, FormRules } from "element-plus";
import type { VariableFormState } from "../../composables/useVariableForm";
import ResponsiveDialog from "../common/ResponsiveDialog.vue";
import ToolbarButton from "../common/ToolbarButton.vue";

type VariableFormTask = {
  id: number;
  name: string;
};

const props = defineProps<{
  activeTab: string;
  editingId: number | null;
  form: VariableFormState;
  isMobile: boolean;
  rules: FormRules;
  submitting: boolean;
  tasks: VariableFormTask[];
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "submit"): void;
  (event: "update:visible", visible: boolean): void;
}>();

const formRef = ref<FormInstance | null>(null);

const visibleValue = computed({
  get: () => props.visible,
  set: (visible: boolean) => emit("update:visible", visible),
});

const submit = async () => {
  if (!formRef.value) return;

  const valid = await formRef.value.validate().catch(() => false);
  if (!valid) return;

  emit("submit");
};
</script>
