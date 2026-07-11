import { ElMessage, ElMessageBox } from "element-plus";
import * as variableApi from "../api/variable";
import type { useHaptics } from "./useHaptics";
import type { VariablePageRow } from "./useVariablePageData";

type UseVariableMutationsOptions = {
  getScopedTaskId: () => number | null;
  haptics: ReturnType<typeof useHaptics>;
  reload: () => unknown;
  selectedIds: { value: number[] };
};

export function useVariableMutations({
  getScopedTaskId,
  haptics,
  reload,
  selectedIds,
}: UseVariableMutationsOptions) {
  const handleStatusChange = async (row: VariablePageRow, value: unknown) => {
    const enabled = !!value;

    try {
      await variableApi.toggleVariables([row.id], enabled);
      haptics.impact();
    } catch {
      row.enabled = !enabled;
    }
  };

  const handleDelete = async (row: VariablePageRow) => {
    try {
      await ElMessageBox.confirm("Permanent delete?", "Warning", {
        type: "warning",
        roundButton: true,
      });
      await variableApi.deleteVariables(
        [row.id],
        getScopedTaskId() ?? undefined,
      );
      ElMessage.success("Deleted");
      reload();
    } catch {
      // User cancellation and request errors keep the list as-is.
    }
  };

  const handleBulkDelete = async () => {
    try {
      await ElMessageBox.confirm(
        `Delete ${selectedIds.value.length} items?`,
        "Batch Action",
        {
          type: "warning",
          roundButton: true,
        },
      );
      await variableApi.deleteVariables(
        selectedIds.value,
        getScopedTaskId() ?? undefined,
      );
      ElMessage.success("Batch success");
      reload();
    } catch {
      // User cancellation and request errors keep the list as-is.
    }
  };

  const handleBulkToggle = async (enabled: boolean) => {
    try {
      await variableApi.toggleVariables(selectedIds.value, enabled);
      ElMessage.success("Batch success");
      reload();
    } catch {
      // Keep the current optimistic UI state unchanged on batch failure.
    }
  };

  return {
    handleBulkDelete,
    handleBulkToggle,
    handleDelete,
    handleStatusChange,
  };
}
