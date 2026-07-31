import { computed, ref } from "vue";
import { useTaskStore } from "../stores/tasks";
import type { Task } from "@/types";

export function useTaskSelection(taskStore: ReturnType<typeof useTaskStore>) {
  const selectedIds = ref<number[]>([]);

  const selectedTasks = computed(() => {
    return taskStore.tasks.filter((task) => selectedIds.value.includes(task.id));
  });

  const handleSelectAll = () => {
    if (selectedIds.value.length === taskStore.tasks.length) {
      selectedIds.value = [];
    } else {
      selectedIds.value = taskStore.tasks.map((task) => task.id);
    }
  };

  const clearAllSelection = () => {
    selectedIds.value = [];
  };

  const handleMobileSelection = (task: Task, isSelected: boolean) => {
    if (isSelected) {
      if (!selectedIds.value.includes(task.id)) {
        selectedIds.value.push(task.id);
      }
    } else {
      selectedIds.value = selectedIds.value.filter((id) => id !== task.id);
    }
  };

  return {
    selectedIds,
    selectedTasks,
    handleSelectAll,
    clearAllSelection,
    handleMobileSelection,
  };
}
