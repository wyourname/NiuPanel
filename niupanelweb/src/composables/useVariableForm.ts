import { reactive, ref, type Ref } from "vue";
import { ElMessage, type FormRules } from "element-plus";
import * as variableApi from "../api/variable";
import type { useHaptics } from "./useHaptics";
import type { VariablePageRow } from "./useVariablePageData";

export type VariableFormState = {
  key: string;
  value: string;
  remarks: string;
  enabled: boolean;
  scope: string;
  scope_ids: number[];
  scope_id: number | null;
};

type UseVariableFormOptions = {
  activeTab: Ref<string>;
  haptics: ReturnType<typeof useHaptics>;
  reload: () => unknown;
};

const createEmptyForm = (scope: string): VariableFormState => ({
  key: "",
  value: "",
  remarks: "",
  enabled: true,
  scope,
  scope_ids: [],
  scope_id: null,
});

const RESERVED_VARIABLE_KEYS = new Set([
  "NIU_TASK_ID",
  "NIUPANEL_TASK_ID",
  "NIU_TASK_RUN_ID",
  "NIUPANEL_TASK_RUN_ID",
  "NIUPANEL_SDK_CONTEXT",
]);

export function useVariableForm({
  activeTab,
  haptics,
  reload,
}: UseVariableFormOptions) {
  const dialogVisible = ref(false);
  const submitting = ref(false);
  const editingId = ref<number | null>(null);
  const form = reactive<VariableFormState>(createEmptyForm("Global"));

  const rules: FormRules = {
    key: [
      { required: true, message: "请输入变量名", trigger: "blur" },
      {
        pattern: /^[A-Za-z_][A-Za-z0-9_]*$/,
        message: "变量名只能包含字母、数字和下划线，且不能以数字开头",
        trigger: "blur",
      },
      { max: 128, message: "变量名不能超过 128 个字符", trigger: "blur" },
      {
        validator: (_rule, value, callback) => {
          if (RESERVED_VARIABLE_KEYS.has(String(value).trim())) {
            callback(new Error("该变量名由 NiuPanel 运行时保留"));
            return;
          }
          callback();
        },
        trigger: "blur",
      },
    ],
    scope_ids: [
      {
        validator: (_rule, value, callback) => {
          if (
            activeTab.value === "Script" &&
            (!Array.isArray(value) || value.length === 0)
          ) {
            callback(new Error("脚本变量必须至少关联一个任务"));
            return;
          }
          callback();
        },
        trigger: "change",
      },
    ],
  };

  const handleCreate = () => {
    haptics.impact();
    editingId.value = null;
    Object.assign(form, createEmptyForm(activeTab.value));
    dialogVisible.value = true;
  };

  const handleEdit = async (row: VariablePageRow) => {
    haptics.impact();
    let value = row.value;
    if (typeof value !== "string") {
      try {
        value = (await variableApi.getVariableValue(row.id)).data.value;
      } catch {
        ElMessage.error("变量值加载失败，暂时无法编辑");
        return;
      }
    }
    editingId.value = row.id;
    Object.assign(form, {
      key: row.key,
      value,
      remarks: row.remarks ?? "",
      enabled: row.enabled,
      scope: row.scope,
      scope_ids: row.task_ids
        ? [...row.task_ids]
        : row.scope_id
          ? [row.scope_id]
          : [],
      scope_id: row.scope_id,
    });
    dialogVisible.value = true;
  };

  const submitForm = async () => {
    if (submitting.value) return;
    submitting.value = true;
    form.key = form.key.trim();
    form.scope_id = form.scope_ids.length > 0 ? form.scope_ids[0] : null;

    try {
      if (editingId.value !== null) {
        await variableApi.updateVariable(editingId.value, form);
      } else {
        await variableApi.createVariable(form);
      }
      ElMessage.success("保存成功");
      dialogVisible.value = false;
      reload();
    } finally {
      submitting.value = false;
    }
  };

  return {
    dialogVisible,
    editingId,
    form,
    handleCreate,
    handleEdit,
    rules,
    submitForm,
    submitting,
  };
}
