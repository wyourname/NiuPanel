import { watch, type Ref } from "vue";
import { watchDebounced } from "@vueuse/core";
import { useTaskStore } from "../stores/tasks";

type UseTaskListLifecycleOptions = {
  searchQuery: Ref<string>;
  selectedIds: Ref<number[]>;
  selectionMode: Ref<boolean>;
  statusFilter: Ref<string>;
  taskStore: ReturnType<typeof useTaskStore>;
};

const toServerStatus = (status: string) => {
  return status === "all" || status === "enabled" || status === "disabled"
    ? undefined
    : status;
};

export function useTaskListLifecycle({
  searchQuery,
  selectedIds,
  selectionMode,
  statusFilter,
  taskStore,
}: UseTaskListLifecycleOptions) {
  watch(
    () => selectedIds.value.length,
    (newCount) => {
      if (newCount === 0 && selectionMode.value) {
        selectionMode.value = false;
      }
    },
  );

  watchDebounced(
    () => [searchQuery.value, statusFilter.value] as const,
    ([query, status]) => {
      taskStore.fetchTasks(false, query, toServerStatus(status));
    },
    { debounce: 300 },
  );
}
