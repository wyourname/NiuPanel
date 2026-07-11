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
    key: [{ required: true, message: "Required", trigger: "blur" }],
    value: [{ required: true, message: "Required", trigger: "blur" }],
  };

  const handleCreate = () => {
    haptics.impact();
    editingId.value = null;
    Object.assign(form, createEmptyForm(activeTab.value));
    dialogVisible.value = true;
  };

  const handleEdit = (row: VariablePageRow) => {
    haptics.impact();
    editingId.value = row.id;
    Object.assign(form, {
      key: row.key,
      value: row.value,
      remarks: row.remarks,
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
    submitting.value = true;
    form.scope_id = form.scope_ids.length > 0 ? form.scope_ids[0] : null;

    try {
      if (editingId.value !== null) {
        await variableApi.updateVariable(editingId.value, form);
      } else {
        await variableApi.createVariable(form);
      }
      ElMessage.success("Saved");
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
