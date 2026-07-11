import { computed, ref } from "vue";
import * as envApi from "../api/environment";
import { useHaptics } from "./useHaptics";
import { useTaskStore } from "../stores/tasks";
import type { TaskDetailTab } from "./taskPageTypes";
import type { Env, Task } from "@/types";

export function useTasksPageState(taskStore: ReturnType<typeof useTaskStore>) {
  const haptics = useHaptics();

  const selectedTaskId = ref<number | null>(null);
  const activeDetailTab = ref<TaskDetailTab>("log");
  const searchQuery = ref("");
  const statusFilter = ref("all");
  const environments = ref<Env[]>([]);

  const selectionMode = ref(false);
  const actionSheetVisible = ref(false);
  const createActionSheetVisible = ref(false);
  const currentActiveTask = ref<Task | null>(null);

  const currentTask = computed(() => {
    return taskStore.tasks.find((task) => task.id === selectedTaskId.value);
  });

  const filteredTasks = computed(() => {
    const query = searchQuery.value.trim().toLowerCase();
    return taskStore.tasks.filter((task) => {
      const matchesStatus =
        statusFilter.value === "all" || task.status === statusFilter.value;
      if (!matchesStatus) return false;
      if (!query) return true;
      return [task.name, task.description, task.path, task.env_type, task.env_version]
        .filter(Boolean)
        .join(" ")
        .toLowerCase()
        .includes(query);
    });
  });

  const sortedTasks = computed(() => {
    return [...filteredTasks.value].sort((a, b) => {
      if (a.is_pinned !== b.is_pinned) return a.is_pinned ? -1 : 1;

      const aRunning = a.status === "Running" ? 1 : 0;
      const bRunning = b.status === "Running" ? 1 : 0;
      if (aRunning !== bRunning) return bRunning - aRunning;

      return 0;
    });
  });

  const fetchEnvironments = async () => {
    try {
      const res = await envApi.getEnvironments();
      environments.value = res.data || [];
    } catch {
      environments.value = [];
    }
  };

  const selectTask = (task: Task) => {
    if (selectedTaskId.value === task.id) return;
    selectedTaskId.value = task.id;
    haptics.impact();
  };

  const openActionSheet = (task: Task) => {
    currentActiveTask.value = task;
    actionSheetVisible.value = true;
  };

  return {
    actionSheetVisible,
    activeDetailTab,
    createActionSheetVisible,
    currentActiveTask,
    currentTask,
    environments,
    fetchEnvironments,
    filteredTasks,
    searchQuery,
    selectedTaskId,
    selectTask,
    selectionMode,
    sortedTasks,
    statusFilter,
    openActionSheet,
  };
}
