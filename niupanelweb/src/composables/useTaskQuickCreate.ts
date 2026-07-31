import { ref } from "vue";
import { ElMessage } from "element-plus";
import * as taskApi from "../api/tasks";

export function useTaskQuickCreate(refreshTasks: (silent?: boolean) => unknown) {
  const quickCreateVisible = ref(false);
  const quickCreating = ref(false);
  const quickCreateForm = ref({ url: "" });

  const openQuickCreate = () => {
    quickCreateForm.value.url = "";
    quickCreateVisible.value = true;
  };

  const handleQuickCreate = async (url = quickCreateForm.value.url) => {
    quickCreateForm.value.url = url;
    quickCreating.value = true;
    try {
      await taskApi.quickCreateFromUrl(quickCreateForm.value.url);
      ElMessage.success("Success");
      quickCreateVisible.value = false;
      refreshTasks(true);
    } finally {
      quickCreating.value = false;
    }
  };

  return {
    quickCreateVisible,
    quickCreating,
    quickCreateForm,
    openQuickCreate,
    handleQuickCreate,
  };
}
