import * as fileApi from "../api/file_manager";
import * as taskApi from "../api/tasks";
import type { Task } from "@/types";

export const isTaskScriptFileMode = (task: Task) => {
  return Boolean(task.path && task.path.length > 0);
};

export const readTaskScriptContent = async (task: Task) => {
  if (isTaskScriptFileMode(task) && task.path) {
    const res = await fileApi.readFileContent(task.path);
    return res.data;
  }

  return task.command || "";
};

export const writeTaskScriptContent = async (
  task: Task,
  content: string,
  isFileMode: boolean,
) => {
  if (isFileMode && task.path) {
    await fileApi.writeFileContent(task.path, content);
    return;
  }

  await taskApi.updateTask(task.id, { command: content });
};
