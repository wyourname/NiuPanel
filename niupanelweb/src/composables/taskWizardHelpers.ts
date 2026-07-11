import {
  formatVariableText,
  parseVariableText,
  type VariableTextItem,
} from "../utils/variableText";

export type TaskVariableItem = VariableTextItem;

export type TaskWizardScriptSourceMode = "upload" | "file" | "command";

export type TaskWizardRandomConfig = {
  start: string;
  end: string;
  count: number;
};

export type TaskWizardForm = {
  name: string;
  description: string;
  path: string;
  command: string;
  cron_schedule: string;
  env_type: string;
  env_version: string;
  requirements: string;
  notify: boolean;
  cpu_limit: number;
  timeout_sec: number;
  memory_limit: number;
  trigger_next_tasks: number[];
  enableRandom: boolean;
  random_config: TaskWizardRandomConfig;
};

export const defaultRandomConfig = (): TaskWizardRandomConfig => ({
  start: "09:00",
  end: "18:00",
  count: 3,
});

export const createTaskWizardForm = (): TaskWizardForm => ({
  name: "",
  description: "",
  path: "",
  command: "",
  cron_schedule: "",
  env_type: "python",
  env_version: "",
  requirements: "",
  notify: false,
  cpu_limit: 0,
  timeout_sec: 0,
  memory_limit: 0,
  trigger_next_tasks: [],
  enableRandom: false,
  random_config: defaultRandomConfig(),
});

export const normalizeEnvVersion = (version?: string | null) => {
  return version?.replace(/^venv_/, "") ?? "";
};

export const parseVariableBulkText = (text: string): TaskVariableItem[] => {
  return parseVariableText(text);
};

export const formatVariableListText = (list: TaskVariableItem[]) => {
  return formatVariableText(list);
};

const getFileExtension = (fileName: string) => {
  const normalizedName = fileName.toLowerCase();
  const dotIndex = normalizedName.lastIndexOf(".");
  return dotIndex >= 0 ? normalizedName.slice(dotIndex) : "";
};

export const isSupportedScriptFileName = (fileName: string) =>
  [".py", ".js", ".mjs", ".cjs", ".ts", ".sh", ".bash"].includes(
    getFileExtension(fileName),
  );

export const inferScriptEnvironment = (
  fileName: string,
  pythonVersions: string[],
  nodeVersions: string[],
) => {
  const extension = getFileExtension(fileName);

  if (extension === ".py") {
    return {
      env_type: "python",
      env_version: pythonVersions[0] || "",
    };
  }

  if ([".js", ".mjs", ".cjs", ".ts"].includes(extension)) {
    return {
      env_type: "node",
      env_version: nodeVersions[0] || "",
    };
  }

  return {
    env_type: "sh",
    env_version: "",
  };
};

export const resolveUploadDirectory = (
  fileName: string,
  existingPath?: string,
) => {
  if (existingPath) {
    const parts = existingPath.split("/");
    if (parts.length > 1) {
      return parts.slice(0, -1).join("/");
    }
  }

  const extension = getFileExtension(fileName);
  if ([".js", ".mjs", ".cjs", ".ts"].includes(extension)) return "node";
  if ([".sh", ".bash"].includes(extension)) return "shell";
  return "python";
};
