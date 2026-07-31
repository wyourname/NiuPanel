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
  const pendingDeleteIds = ref<number[]>([]);
  const pendingSharedDeleteIds = ref<number[]>([]);
  const sourceModeVisible = ref(false);
  const rawSource = ref("");

  const hasDraftRows = computed(() =>
    variables.value.some((variable) => variable.id === null || variable.isNew),
  );

  const hasChanges = computed(
    () =>
      pendingDeleteIds.value.length > 0 ||
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
    try {
      const previousPersistedIds = variables.value
        .filter(isPersistedTaskVariableRow)
        .map((variable) => variable.id);
      const nextVariables = applyTaskVariablesSource(
        variables.value,
        rawSource.value,
        taskId.value,
      );
      const nextPersistedIds = new Set(
        nextVariables
          .filter(isPersistedTaskVariableRow)
          .map((variable) => variable.id),
      );
      const removedRows = variables.value
        .filter(isPersistedTaskVariableRow)
        .filter((variable) => !nextPersistedIds.has(variable.id));
      pendingDeleteIds.value = [
        ...new Set([
          ...pendingDeleteIds.value,
          ...previousPersistedIds.filter((id) => !nextPersistedIds.has(id)),
        ]),
      ];
      pendingSharedDeleteIds.value = [
        ...new Set([
          ...pendingSharedDeleteIds.value,
          ...removedRows
            .filter((variable) => (variable.task_ids?.length ?? 0) > 1)
            .map((variable) => variable.id),
        ]),
      ];
      variables.value = nextVariables;

      selectedIds.value = [];
      sourceModeVisible.value = false;
      ElMessage.success("解析完成，请记得点击保存");
    } catch (error) {
      ElMessage.error(
        error instanceof Error ? error.message : "变量源码格式错误",
      );
    }
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
      pendingDeleteIds.value = [];
      pendingSharedDeleteIds.value = [];
    } catch {
      ElMessage.error("加载变量失败");
    } finally {
      loading.value = false;
    }
  };

  const handleStatusChange = async (row: TaskVariableRow, val: boolean) => {
    if (row.isNew || row.id === null) return;

    if ((row.task_ids?.length ?? 0) > 1) {
      try {
        await ElMessageBox.confirm(
          `变量 "${row.key}" 同时被 ${row.task_ids?.length} 个任务使用，切换状态会影响所有关联任务。`,
          "共享变量确认",
          { type: "warning" },
        );
      } catch {
        row.enabled = !val;
        return;
      }
    }

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
      const shared = (row.task_ids?.length ?? 0) > 1;
      await ElMessageBox.confirm(
        shared
          ? `变量 "${row.key}" 被多个任务使用，本次只会从当前任务解除关联。`
          : `变量 "${row.key}" 只关联当前任务，解除后变量将被删除。`,
        "解除变量关联",
        { type: "warning" },
      );
    } catch {
      return;
    }

    if (row.id !== null) {
      pendingDeleteIds.value = [
        ...new Set([...pendingDeleteIds.value, row.id]),
      ];
      if ((row.task_ids?.length ?? 0) > 1) {
        pendingSharedDeleteIds.value = [
          ...new Set([...pendingSharedDeleteIds.value, row.id]),
        ];
      }
    }
    variables.value = variables.value.filter((variable) => variable !== row);
    selectedIds.value = selectedIds.value.filter((id) => id !== row.id);
    ElMessage.success("已标记解除关联，请点击“保存全部”提交");
  };

  const batchDelete = async () => {
    if (selectedIds.value.length === 0) return;

    const selectedSet = new Set(selectedIds.value);
    const sharedCount = variables.value.filter(
      (variable) =>
        variable.id !== null &&
        selectedSet.has(variable.id) &&
        (variable.task_ids?.length ?? 0) > 1,
    ).length;
    try {
      await ElMessageBox.confirm(
        `将从当前任务解除 ${selectedIds.value.length} 个变量关联${
          sharedCount > 0
            ? `，其中 ${sharedCount} 个共享变量仍会保留给其他任务`
            : "；仅关联当前任务的变量将被删除"
        }。`,
        "批量解除关联",
        { type: "warning" },
      );
    } catch {
      return;
    }

    pendingDeleteIds.value = [
      ...new Set([...pendingDeleteIds.value, ...selectedIds.value]),
    ];
    const sharedIds = variables.value
      .filter(
        (variable) =>
          variable.id !== null &&
          selectedSet.has(variable.id) &&
          (variable.task_ids?.length ?? 0) > 1,
      )
      .map((variable) => variable.id)
      .filter((id): id is number => id !== null);
    pendingSharedDeleteIds.value = [
      ...new Set([...pendingSharedDeleteIds.value, ...sharedIds]),
    ];
    variables.value = variables.value.filter(
      (variable) =>
        variable.id === null || !selectedSet.has(variable.id),
    );
    selectedIds.value = [];
    ElMessage.success("已批量标记解除关联，请点击“保存全部”提交");
  };

  const batchToggle = async (enable: boolean) => {
    if (selectedIds.value.length === 0) return;

    const selectedSet = new Set(selectedIds.value);
    const sharedCount = variables.value.filter(
      (variable) =>
        variable.id !== null &&
        selectedSet.has(variable.id) &&
        (variable.task_ids?.length ?? 0) > 1,
    ).length;
    if (sharedCount > 0) {
      try {
        await ElMessageBox.confirm(
          `选中项包含 ${sharedCount} 个共享变量，状态变化会影响其关联的全部任务。`,
          "共享变量确认",
          { type: "warning" },
        );
      } catch {
        return;
      }
    }

    loading.value = true;

    try {
      await variableApi.toggleVariables(selectedIds.value, enable);
      ElMessage.success("批量操作成功");
      await fetchVariables();
    } catch {
      ElMessage.error("批量操作失败");
    } finally {
      loading.value = false;
    }
  };

  const saveAll = async () => {
    if (saving.value || !hasChanges.value) return;
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

      const invalidItem = variables.value.find(
        (variable) =>
          !variable.key ||
          !/^[A-Za-z_][A-Za-z0-9_]*$/.test(variable.key.trim()) ||
          [
            "NIU_TASK_ID",
            "NIUPANEL_TASK_ID",
            "NIU_TASK_RUN_ID",
            "NIUPANEL_TASK_RUN_ID",
            "NIUPANEL_SDK_CONTEXT",
          ].includes(variable.key.trim()),
      );
      if (invalidItem) {
        ElMessage.error("变量名无效，或使用了 NiuPanel 运行时保留名称");
        return;
      }

      const sharedModifiedItems = modifiedItems.filter(
        (variable) => (variable.task_ids?.length ?? 0) > 1,
      );
      if (
        sharedModifiedItems.length > 0 ||
        pendingSharedDeleteIds.value.length > 0
      ) {
        try {
          await ElMessageBox.confirm(
            `有 ${sharedModifiedItems.length} 个共享变量会被修改，另有 ${pendingSharedDeleteIds.value.length} 个共享变量会从当前任务解绑。`,
            "共享变量确认",
            { type: "warning" },
          );
        } catch {
          return;
        }
      }

      const modifiedUpserts = modifiedItems.map((item) => {
        const { id, ...variable } = buildTaskVariableUpdatePayload(item);
        return { id, variable };
      });
      const newUpserts = newItems.map((item) => ({
        variable: {
          key: item.key.trim(),
          value: item.value,
          remarks: item.remarks,
          enabled: item.enabled,
          scope: "Script",
          scope_ids: [taskId.value],
        },
      }));

      await variableApi.saveTaskVariables({
        task_id: taskId.value,
        upserts: [...modifiedUpserts, ...newUpserts],
        delete_ids: pendingDeleteIds.value,
      });

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
