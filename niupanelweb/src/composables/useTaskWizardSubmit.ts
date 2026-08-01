import { ref, type ComputedRef, type Ref } from "vue";
import { ElMessage } from "element-plus";
import * as fileApi from "../api/file_manager";
import * as taskApi from "../api/tasks";
import { createUploadFormData } from "../api/upload";
import {
  resolveUploadDirectory,
  type TaskVariableItem,
  type TaskWizardForm,
  type TaskWizardRandomConfig,
  type TaskWizardScriptSourceMode,
} from "./taskWizardHelpers";
import type { TaskWizardInitialData } from "./useTaskWizardData";

type TaskWizardSubmitPayload = Omit<TaskWizardForm, "random_config"> & {
  variables: TaskVariableItem[];
  random_config: TaskWizardRandomConfig | null;
};

type UseTaskWizardSubmitOptions = {
  form: TaskWizardForm;
  getSubmitVariables: () => TaskVariableItem[];
  initialData: ComputedRef<TaskWizardInitialData | undefined>;
  isEdit: ComputedRef<boolean>;
  onSuccess: () => void;
  scriptSourceMode: Ref<TaskWizardScriptSourceMode>;
  uploadedFile: Ref<File | null>;
};

export function useTaskWizardSubmit({
  form,
  getSubmitVariables,
  initialData,
  isEdit,
  onSuccess,
  scriptSourceMode,
  uploadedFile,
}: UseTaskWizardSubmitOptions) {
  const submitting = ref(false);

  const uploadScriptIfNeeded = async () => {
    if (scriptSourceMode.value !== "upload" || !uploadedFile.value) {
      return;
    }

    const formData = createUploadFormData([["file", uploadedFile.value]]);

    const dir = resolveUploadDirectory(
      uploadedFile.value.name,
      isEdit.value ? initialData.value?.path : undefined,
    );

    await fileApi.uploadFile(dir, formData);
    form.path = `${dir}/${uploadedFile.value.name}`;
    form.command = "";
  };

  const buildPayload = (): TaskWizardSubmitPayload => {
    const payload: TaskWizardSubmitPayload = {
      ...form,
      variables: getSubmitVariables(),
      cron_schedule: form.enableRandom ? "" : form.cron_schedule,
      random_config: form.enableRandom ? form.random_config : null,
    };

    if (scriptSourceMode.value === "command") {
      payload.path = "";
    } else {
      payload.command = "";
    }

    return payload;
  };

  const submit = async () => {
    submitting.value = true;

    try {
      await uploadScriptIfNeeded();

      const payload = buildPayload();
      const data = initialData.value;

      if (isEdit.value && data?.id) {
        await taskApi.updateTask(data.id, payload);
      } else {
        await taskApi.createTask(payload);
      }

      ElMessage.success("操作成功");
      onSuccess();
    } catch (error) {
      console.error(error);
      ElMessage.error(
        "失败: " + (error instanceof Error ? error.message : String(error)),
      );
    } finally {
      submitting.value = false;
    }
  };

  return {
    submit,
    submitting,
  };
}
