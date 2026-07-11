import { ref, type Ref } from "vue";
import type { useHaptics } from "./useHaptics";

type VariableSelectableRow = {
  id: number;
};

type UseVariablePageSelectionOptions<T extends VariableSelectableRow> = {
  haptics: ReturnType<typeof useHaptics>;
  variables: Ref<T[]>;
};

export function useVariablePageSelection<T extends VariableSelectableRow>({
  haptics,
  variables,
}: UseVariablePageSelectionOptions<T>) {
  const selectedIds = ref<number[]>([]);

  const updateSelection = (row: T, selected: boolean) => {
    const isSelected = selectedIds.value.includes(row.id);

    if (selected === isSelected) {
      return;
    }

    if (selected) {
      selectedIds.value.push(row.id);
      haptics.selectionChanged();
      return;
    }

    selectedIds.value = selectedIds.value.filter((id) => id !== row.id);
  };

  const toggleMobileSelection = (row: T) => {
    updateSelection(row, !selectedIds.value.includes(row.id));
  };

  const handleCardClick = (row: T) => {
    if (selectedIds.value.length > 0) {
      toggleMobileSelection(row);
    }
  };

  const handleSelectAll = () => {
    if (selectedIds.value.length === variables.value.length) {
      selectedIds.value = [];
      return;
    }

    selectedIds.value = variables.value.map((variable) => variable.id);
  };

  const clearSelection = () => {
    selectedIds.value = [];
  };

  return {
    clearSelection,
    handleCardClick,
    handleSelectAll,
    selectedIds,
    toggleMobileSelection,
    updateSelection,
  };
}
