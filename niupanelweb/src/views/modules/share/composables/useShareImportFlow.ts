import { onUnmounted, reactive, ref } from "vue";
import { ElMessage } from "element-plus";
import * as shareApi from "../../../../api/share";
import type { ImportStatus, NiuPackage, SubmitImportRequest } from "@/types";

type ShareImportForm = Required<Pick<SubmitImportRequest, "url">> & {
  password: string;
};

const createEmptyForm = (): ShareImportForm => ({
  url: "",
  password: "",
});

export function useShareImportFlow() {
  const importForm = reactive<ShareImportForm>(createEmptyForm());
  const downloading = ref(false);
  const importing = ref(false);
  const isUpdateMode = ref(false);
  const stagingId = ref<string | null>(null);
  const downloadStatus = ref<ImportStatus | null>(null);
  const previewPackage = ref<NiuPackage | null>(null);
  const selectedTasks = ref<string[]>([]);
  const downloadStep = ref(0);
  const pollTimer = ref<ReturnType<typeof setInterval> | null>(null);

  const clearPollTimer = () => {
    if (!pollTimer.value) return;
    clearInterval(pollTimer.value);
    pollTimer.value = null;
  };

  const resetImport = () => {
    Object.assign(importForm, createEmptyForm());
    downloading.value = false;
    importing.value = false;
    isUpdateMode.value = false;
    stagingId.value = null;
    downloadStatus.value = null;
    previewPackage.value = null;
    selectedTasks.value = [];
    downloadStep.value = 0;
    clearPollTimer();
  };

  const fetchPreview = async () => {
    if (!stagingId.value) return;

    try {
      const res = await shareApi.getImportPreview(stagingId.value);
      previewPackage.value = res.data;
      selectedTasks.value = res.data.tasks.map((task) => task.meta.name);
      downloading.value = false;
      downloadStep.value = 2;
    } catch {
      downloading.value = false;
    }
  };

  const startPolling = () => {
    clearPollTimer();
    pollTimer.value = setInterval(async () => {
      if (!stagingId.value) return;

      try {
        const res = await shareApi.getImportStatus(stagingId.value);
        downloadStatus.value = res.data;

        if (res.data.state === "ready") {
          clearPollTimer();
          fetchPreview();
          return;
        }

        if (res.data.state === "error") {
          clearPollTimer();
          downloading.value = false;
          ElMessage.error(res.data.message || "解析失败");
        }
      } catch {
        clearPollTimer();
        downloading.value = false;
      }
    }, 1000);
  };

  const handleSubmitImport = async () => {
    if (!importForm.url) return;

    downloading.value = true;
    downloadStep.value = 1;
    try {
      const payload: SubmitImportRequest = {
        url: importForm.url,
        password: importForm.password || undefined,
      };
      const res = await shareApi.submitImport(payload);
      stagingId.value = res.data.staging_id;
      startPolling();
    } catch (error) {
      ElMessage.error(error instanceof Error ? error.message : "提交失败");
      downloading.value = false;
      downloadStep.value = 0;
    }
  };

  const handleRetry = () => {
    handleSubmitImport();
  };

  const handleConfirmImport = async () => {
    if (!stagingId.value) return false;

    importing.value = true;
    try {
      const res = await shareApi.confirmImport(stagingId.value, {
        selected_tasks: selectedTasks.value,
        update_existing: isUpdateMode.value,
      });
      ElMessage.success(
        isUpdateMode.value
          ? `成功更新 ${res.data.success_count} 个任务`
          : `成功导入 ${res.data.success_count} 个任务`,
      );
      setTimeout(resetImport, 500);
      return true;
    } catch (error) {
      ElMessage.error(error instanceof Error ? error.message : "导入失败");
      return false;
    } finally {
      importing.value = false;
    }
  };

  const toggleTaskSelection = (name: string) => {
    if (selectedTasks.value.includes(name)) {
      selectedTasks.value = selectedTasks.value.filter((task) => task !== name);
      return;
    }
    selectedTasks.value.push(name);
  };

  const toggleSelectAll = () => {
    if (!previewPackage.value) return;

    if (selectedTasks.value.length === previewPackage.value.tasks.length) {
      selectedTasks.value = [];
      return;
    }

    selectedTasks.value = previewPackage.value.tasks.map(
      (task) => task.meta.name,
    );
  };

  const setImportUrl = (url: string, isUpdate = false) => {
    importForm.url = url;
    isUpdateMode.value = isUpdate;
    handleSubmitImport();
  };

  onUnmounted(clearPollTimer);

  return {
    downloadStep,
    downloadStatus,
    downloading,
    handleConfirmImport,
    handleRetry,
    handleSubmitImport,
    importForm,
    importing,
    previewPackage,
    resetImport,
    selectedTasks,
    setImportUrl,
    toggleSelectAll,
    toggleTaskSelection,
  };
}
