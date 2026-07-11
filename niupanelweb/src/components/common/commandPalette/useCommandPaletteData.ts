import { ref } from "vue";
import { getEnvironments } from "@/api/environment";
import { getTasks } from "@/api/tasks";
import { getVariables } from "@/api/variable";
import type { Env, Task, Variable } from "@/types";
import type { PaletteItem } from "./types";

const toTaskPaletteItem = (task: Task): PaletteItem => ({
  title: task.name,
  desc: `任务 #${task.id} · ${task.status} · ${task.cron_schedule || "手动"}`,
  id: task.id,
  type: "task",
  path: "/tasks",
  query: { q: task.name },
});

const toVariablePaletteItem = (variable: Variable): PaletteItem => ({
  title: variable.key,
  desc: `变量 · ${variable.scope} · ${variable.remarks || "无备注"}`,
  id: variable.id,
  type: "variable",
  path: "/variables",
  query: { q: variable.key },
});

const toEnvironmentPaletteItem = (environment: Env): PaletteItem => ({
  title: environment.name,
  desc: `环境 · ${environment.env_type.toUpperCase()} · ${
    environment.version || "System"
  }`,
  id: environment.name,
  type: "env",
  path: "/environments",
  query: { q: environment.name },
});

export function useCommandPaletteData() {
  const tasks = ref<PaletteItem[]>([]);
  const variables = ref<PaletteItem[]>([]);
  const environments = ref<PaletteItem[]>([]);

  const loadPaletteData = async () => {
    try {
      const [taskRes, variableRes, envRes] = await Promise.all([
        getTasks(1, 100),
        getVariables({ page: 1, page_size: 100 }),
        getEnvironments(),
      ]);

      if (taskRes.data?.items) {
        tasks.value = taskRes.data.items.map(toTaskPaletteItem);
      }

      if (variableRes.data?.items) {
        variables.value = variableRes.data.items.map(toVariablePaletteItem);
      }

      if (envRes.data) {
        environments.value = envRes.data.map(toEnvironmentPaletteItem);
      }
    } catch (error) {
      console.error("Failed to load palette data", error);
    }
  };

  return {
    environments,
    loadPaletteData,
    tasks,
    variables,
  };
}
