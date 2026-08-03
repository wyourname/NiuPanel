import { computed, ref, type ComputedRef, type Ref } from "vue";
import * as envApi from "../api/environment";
import * as taskApi from "../api/tasks";
import * as variableApi from "../api/variable";
import type { Env, Task, Variable } from "@/types";
import {
  defaultRandomConfig,
  normalizeEnvVersion,
  type TaskVariableItem,
  type TaskWizardForm,
  type TaskWizardRandomConfig,
  type TaskWizardScriptSourceMode,
} from "./taskWizardHelpers";

export type TaskWizardInitialData = Partial<Task> & {
  id?: number;
  env_version?: string | null;
  random_config?: TaskWizardRandomConfig | null;
  scriptSourceMode?: TaskWizardScriptSourceMode;
  uploadedFile?: File;
};

type TaskWizardSimpleTask = {
  id: number;
  name: string;
};

type UseTaskWizardDataOptions = {
  form: TaskWizardForm;
  initialData: ComputedRef<TaskWizardInitialData | undefined>;
  isEdit: ComputedRef<boolean>;
  scriptSourceMode: Ref<TaskWizardScriptSourceMode>;
  setVariables: (items: TaskVariableItem[]) => void;
};

const toVariableItems = (data: Variable[]): TaskVariableItem[] =>
  data.map((variable) => ({
    key: variable.key,
    value: variable.value,
  }));

export function useTaskWizardData({
  form,
  initialData,
  isEdit,
  scriptSourceMode,
  setVariables,
}: UseTaskWizardDataOptions) {
  const environments = ref<Env[]>([]);
  const allTasks = ref<TaskWizardSimpleTask[]>([]);

  const pythonVersions = computed(() =>
    environments.value
      .filter((environment) => environment.env_type === "python")
      .map((environment) => environment.version)
      .filter((version): version is string => Boolean(version)),
  );

  const nodeVersions = computed(() =>
    environments.value
      .filter((environment) => environment.env_type === "node")
      .map((environment) => environment.version)
      .filter((version): version is string => Boolean(version)),
  );

  const loadWizardData = async () => {
    try {
      const [envRes, taskRes] = await Promise.all([
        envApi.getEnvironments(),
        variableApi.getAllTasksSimple(),
      ]);

      environments.value = envRes.data;
      allTasks.value = taskRes.data;
    } catch (error) {
      console.error("Failed to fetch wizard data:", error);
    }

    const data = initialData.value;

    if (data) {
      if (data.scriptSourceMode) {
        scriptSourceMode.value = data.scriptSourceMode;
      }

      if (data.env_type) {
        form.env_type = data.env_type;
      }

      if (data.env_version) {
        form.env_version = normalizeEnvVersion(data.env_version);
      }
    }

    if (isEdit.value && data?.id) {
      Object.assign(form, {
        ...data,
        env_version: normalizeEnvVersion(data.env_version),
        timeout_sec: data.timeout_sec ?? 0,
        cpu_limit: data.cpu_limit ?? 0,
      });

      if (data.random_config) {
        form.enableRandom = true;
        form.random_config = { ...data.random_config };
      } else {
        form.enableRandom = false;
        form.random_config = defaultRandomConfig();
      }

      if (form.command) {
        scriptSourceMode.value = "command";
      } else if (form.path) {
        scriptSourceMode.value = "file";
      }

      try {
        const res = await variableApi.getVariablesByTaskId(data.id);
        setVariables(toVariableItems(res.data));
      } catch {
        // Variable hydration is best-effort; task editing should still open.
      }
    }

  };

  return {
    allTasks,
    environments,
    loadWizardData,
    nodeVersions,
    pythonVersions,
  };
}
