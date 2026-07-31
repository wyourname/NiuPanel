import { ref } from "vue";
import { ElMessage } from "element-plus";
import { useAppStore } from "../stores/app";
import { useTaskStore } from "../stores/tasks";
import {
  isTaskScriptFileMode,
  readTaskScriptContent,
  writeTaskScriptContent,
} from "../utils/taskScriptContent";
import type { Task } from "@/types";

type UseTaskScriptDialogOptions = {
  appStore: ReturnType<typeof useAppStore>;
  taskStore: ReturnType<typeof useTaskStore>;
};

export function useTaskScriptDialog({
  appStore,
  taskStore,
}: UseTaskScriptDialogOptions) {
  const scriptEditorVisible = ref(false);
  const scriptEditorContent = ref("");
  const scriptEditorLoading = ref(false);
  const currentScriptTask = ref<Task | null>(null);
  const isFileMode = ref(false);

  const handleEditScript = async (task: Task) => {
    currentScriptTask.value = task;
    scriptEditorContent.value = "";
    scriptEditorVisible.value = true;
    scriptEditorLoading.value = true;

    isFileMode.value = isTaskScriptFileMode(task);
    try {
      scriptEditorContent.value = await readTaskScriptContent(task);
    } catch {
      scriptEditorContent.value = "";
    } finally {
      scriptEditorLoading.value = false;
    }
  };

  const saveScript = async () => {
    if (!currentScriptTask.value) return;

    scriptEditorLoading.value = true;
    try {
      await writeTaskScriptContent(
        currentScriptTask.value,
        scriptEditorContent.value,
        isFileMode.value,
      );
      ElMessage.success("保存成功");
      scriptEditorVisible.value = false;
      taskStore.refreshTasks(true);
    } finally {
      scriptEditorLoading.value = false;
    }
  };

  return {
    currentScriptTask,
    handleEditScript,
    isFileMode,
    saveScript,
    scriptEditorContent,
    scriptEditorLoading,
    scriptEditorVisible,
  };
}
