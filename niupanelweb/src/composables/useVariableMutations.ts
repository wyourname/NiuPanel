import { ElMessage, ElMessageBox } from "element-plus";
import * as variableApi from "../api/variable";
import type { useHaptics } from "./useHaptics";
import type { VariablePageRow } from "./useVariablePageData";
import type { Ref } from "vue";

type UseVariableMutationsOptions = {
  getScopedTaskId: () => number | null;
  haptics: ReturnType<typeof useHaptics>;
  reload: () => unknown;
  selectedIds: { value: number[] };
  variables: Ref<VariablePageRow[]>;
};

export function useVariableMutations({
  getScopedTaskId,
  haptics,
  reload,
  selectedIds,
  variables,
}: UseVariableMutationsOptions) {
  const handleStatusChange = async (row: VariablePageRow, value: unknown) => {
    const enabled = !!value;

    try {
      if (getScopedTaskId() && (row.task_ids?.length ?? 0) > 1) {
        await ElMessageBox.confirm(
          `变量 "${row.key}" 被 ${row.task_ids?.length} 个任务共享，切换状态会影响全部关联任务。`,
          "共享变量确认",
          { type: "warning" },
        );
      }
      await variableApi.toggleVariables([row.id], enabled);
      haptics.impact();
    } catch {
      row.enabled = !enabled;
    }
  };

  const handleDelete = async (row: VariablePageRow) => {
    const scopedTaskId = getScopedTaskId();
    try {
      await ElMessageBox.confirm(
        scopedTaskId
          ? (row.task_ids?.length ?? 0) > 1
            ? `变量 "${row.key}" 被多个任务使用，本次只会从当前任务解除关联。`
            : `变量 "${row.key}" 只关联当前任务，解除后变量将被删除。`
          : `确定永久删除变量 "${row.key}" 吗？`,
        scopedTaskId ? "解除变量绑定" : "删除变量",
        {
          type: "warning",
          roundButton: true,
        },
      );
    } catch {
      return;
    }

    try {
      await variableApi.deleteVariables(
        [row.id],
        scopedTaskId ?? undefined,
      );
      ElMessage.success(scopedTaskId ? "已解除绑定" : "已删除");
      reload();
    } catch {
      ElMessage.error(scopedTaskId ? "解除绑定失败" : "删除失败");
    }
  };

  const handleBulkDelete = async () => {
    if (selectedIds.value.length === 0) return;
    const scopedTaskId = getScopedTaskId();
    const selectedSet = new Set(selectedIds.value);
    const sharedCount = variables.value.filter(
      (variable) =>
        selectedSet.has(variable.id) &&
        (variable.task_ids?.length ?? 0) > 1,
    ).length;
    try {
      await ElMessageBox.confirm(
        scopedTaskId
          ? `将从当前任务解除 ${selectedIds.value.length} 个变量关联${
              sharedCount > 0
                ? `，其中 ${sharedCount} 个共享变量仍保留给其他任务`
                : "；仅关联当前任务的变量将被删除"
            }。`
          : `确定永久删除选中的 ${selectedIds.value.length} 个变量吗？`,
        scopedTaskId ? "批量解除关联" : "批量删除",
        {
          type: "warning",
          roundButton: true,
        },
      );
    } catch {
      return;
    }

    try {
      await variableApi.deleteVariables(
        selectedIds.value,
        scopedTaskId ?? undefined,
      );
      ElMessage.success(scopedTaskId ? "批量解除关联成功" : "批量删除成功");
      reload();
    } catch {
      ElMessage.error(scopedTaskId ? "批量解除关联失败" : "批量删除失败");
    }
  };

  const handleBulkToggle = async (enabled: boolean) => {
    if (selectedIds.value.length === 0) return;
    const selectedSet = new Set(selectedIds.value);
    const sharedCount = variables.value.filter(
      (variable) =>
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

    try {
      await variableApi.toggleVariables(selectedIds.value, enabled);
      ElMessage.success("批量状态更新成功");
      reload();
    } catch {
      ElMessage.error("批量状态更新失败");
    }
  };

  return {
    handleBulkDelete,
    handleBulkToggle,
    handleDelete,
    handleStatusChange,
  };
}
