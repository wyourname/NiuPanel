import { ref } from "vue";
import { defineStore } from "pinia";

export type WorkspaceTaskCommandType =
  | "create"
  | "quick_create"
  | "create_upload"
  | "edit"
  | "script"
  | "variables"
  | "cron"
  | "select";

export type WorkspaceTaskCommandOptions = {
  uploadedFile?: File;
};

export type WorkspaceTaskCommand = {
  id: number;
  type: WorkspaceTaskCommandType;
  taskId?: number;
  issuedAt: number;
} & WorkspaceTaskCommandOptions;

export const useWorkspaceTaskCommandStore = defineStore(
  "workspace-task-commands",
  () => {
    const command = ref<WorkspaceTaskCommand | null>(null);
    let sequence = 0;

    const dispatch = (
      type: WorkspaceTaskCommandType,
      taskId?: number,
      options: WorkspaceTaskCommandOptions = {},
    ) => {
      sequence += 1;
      command.value = {
        id: sequence,
        type,
        taskId,
        issuedAt: Date.now(),
        ...options,
      };
    };

    const clear = (id?: number) => {
      if (id && command.value?.id !== id) return;
      command.value = null;
    };

    return {
      clear,
      command,
      dispatch,
    };
  },
);
