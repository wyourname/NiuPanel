import { ref } from "vue";
import { ElMessage } from "element-plus";
import * as taskApi from "../api/tasks";
import type { Task } from "@/types";

const defaultRandomConfig = () => ({
  start: "09:00",
  end: "18:00",
  count: 3,
});

export function useTaskCronEditor(refreshTasks: (silent?: boolean) => unknown) {
  const cronEditVisible = ref(false);
  const enableRandom = ref(false);
  const randomConfig = ref(defaultRandomConfig());
  const editingCronTask = ref<Task | null>(null);
  const cronInput = ref("");
  const cronSaving = ref(false);

  const openCronEditor = (task: Task) => {
    editingCronTask.value = task;
    cronInput.value = task.cron_schedule || "";

    if (task.random_config) {
      enableRandom.value = true;
      randomConfig.value = { ...task.random_config };
    } else {
      enableRandom.value = false;
      randomConfig.value = defaultRandomConfig();
    }

    cronEditVisible.value = true;
  };

  const saveCron = async () => {
    if (!editingCronTask.value) return;
    cronSaving.value = true;
    try {
      await taskApi.updateTask(editingCronTask.value.id, {
        cron_schedule: enableRandom.value ? "" : cronInput.value,
        random_config: enableRandom.value ? randomConfig.value : null,
      });
      ElMessage.success("Updated");
      cronEditVisible.value = false;
      refreshTasks(true);
    } finally {
      cronSaving.value = false;
    }
  };

  return {
    cronEditVisible,
    enableRandom,
    randomConfig,
    editingCronTask,
    cronInput,
    cronSaving,
    openCronEditor,
    saveCron,
  };
}
