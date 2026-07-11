import { computed, ref, type ComputedRef } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import * as variableApi from "../api/variable";
import {
  applyTaskVariablesSource,
  buildTaskVariableUpdatePayload,
  createTaskVariableRow,
  formatTaskVariablesSource,
  hasTaskVariableRowChanges,
  hydrateTaskVariableRow,
  type TaskVariableRow,
  isPersistedTaskVariableRow,
} from "./taskVariableEditorHelpers";

type UseTaskVariableEditorRowsOptions = {
  onSuccess: () => void;
  taskId: ComputedRef<number>;
};

export function useTaskVariableEditorRows({
  onSuccess,
  taskId,
}: UseTaskVariableEditorRowsOptions) {
  const loading = ref(false);
  const saving = ref(false);
  const variables = ref<TaskVariableRow[]>([]);
  const selectedIds = ref<number[]>([]);
  const sourceModeVisible = ref(false);
  const rawSource = ref("");

  const hasDraftRows = computed(() =>
    variables.value.some((variable) => variable.id === null || variable.isNew),
  );

  const hasChanges = computed(() =>
    variables.value.some(hasTaskVariableRowChanges),
  );

  const updateSelection = (id: number, checked: boolean) => {
    if (checked) {
      if (!selectedIds.value.includes(id)) {
        selectedIds.value.push(id);
      }
      return;
    }

    selectedIds.value = selectedIds.value.filter((item) => item !== id);
  };

  const openSourceMode = () => {
    rawSource.value = formatTaskVariablesSource(variables.value);
    sourceModeVisible.value = true;
  };

  const applySource = () => {
    variables.value = applyTaskVariablesSource(
      variables.value,
      rawSource.value,
      taskId.value,
    );

    selectedIds.value = [];
    sourceModeVisible.value = false;
    ElMessage.success("解析完成，请记得点击保存");
  };

  const fetchVariables = async () => {
    loading.value = true;

    try {
      const res = await variableApi.getVariablesByTaskId(taskId.value);
      const list = res.data.items || [];
      variables.value = list.map((variable) =>
        hydrateTaskVariableRow(variable, taskId.value),
      );
      selectedIds.value = [];
    } catch {
      ElMessage.error("加载变量失败");
    } finally {
      loading.value = false;
    }
  };

  const handleStatusChange = async (row: TaskVariableRow, val: boolean) => {
    if (row.isNew || row.id === null) return;

    row.statusLoading = true;

    try {
      await variableApi.toggleVariables([row.id], val);
    } catch {
      row.enabled = !val;
      ElMessage.error("状态修改失败");
    } finally {
      row.statusLoading = false;
    }
  };

  const handleDelete = async (row: TaskVariableRow) => {
    try {
      await ElMessageBox.confirm(`确定要删除变量 "${row.key}" 吗?`, "删除确认", {
        type: "warning",
      });

      loading.value = true;

      if (row.id !== null) {
        await variableApi.deleteVariables([row.id], taskId.value);
      }

      ElMessage.success("删除成功");
      await fetchVariables();
    } catch {
      loading.value = false;
    }
  };

  const batchDelete = async () => {
    if (selectedIds.value.length === 0) return;

    try {
      await ElMessageBox.confirm(
        `确定删除选中的 ${selectedIds.value.length} 个变量?`,
        "批量删除",
        { type: "warning" },
      );

      loading.value = true;
      await variableApi.deleteVariables(selectedIds.value, taskId.value);
      ElMessage.success("批量删除成功");
      await fetchVariables();
    } catch {
      loading.value = false;
    }
  };

  const batchToggle = async (enable: boolean) => {
    if (selectedIds.value.length === 0) return;

    loading.value = true;

    try {
      await variableApi.toggleVariables(selectedIds.value, enable);
      ElMessage.success("批量操作成功");
      await fetchVariables();
    } catch {
      loading.value = false;
    }
  };

  const saveAll = async () => {
    saving.value = true;

    try {
      const newItems = variables.value.filter(
        (variable) => variable.isNew && variable.key,
      );
      const modifiedItems = variables.value
        .filter(isPersistedTaskVariableRow)
        .filter(
          (variable) =>
            !variable.isNew && hasTaskVariableRowChanges(variable),
        );

      if (modifiedItems.length > 0) {
        await variableApi.updateVariables(
          modifiedItems.map(buildTaskVariableUpdatePayload),
        );
      }

      if (newItems.length > 0) {
        for (const item of newItems) {
          await variableApi.createVariable({
            key: item.key,
            value: item.value,
            remarks: item.remarks,
            enabled: item.enabled,
            scope: item.scope,
            scope_id: item.scope_id,
          });
        }
      }

      ElMessage.success("保存成功");
      await fetchVariables();
      onSuccess();
    } catch {
      ElMessage.error("保存失败");
    } finally {
      saving.value = false;
    }
  };

  const addVariableRow = () => {
    variables.value.unshift(createTaskVariableRow(taskId.value));
  };

  const cancelNewVariable = (index: number) => {
    variables.value.splice(index, 1);
  };

  return {
    addVariableRow,
    applySource,
    batchDelete,
    batchToggle,
    cancelNewVariable,
    fetchVariables,
    handleDelete,
    handleStatusChange,
    hasChanges,
    hasDraftRows,
    loading,
    openSourceMode,
    rawSource,
    saveAll,
    saving,
    selectedIds,
    sourceModeVisible,
    updateSelection,
    variables,
  };
}
