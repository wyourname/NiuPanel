import type { Env, Task } from "@/types";

const resolveDefaultEnvironment = (environments: Env[]) => {
  const pythonEnv = environments.find((env) => env.env_type === "python");
  if (pythonEnv) {
    return {
      env_type: "python",
      env_version: pythonEnv.name,
    };
  }

  const nodeEnv = environments.find((env) => env.env_type === "node");
  if (nodeEnv) {
    return {
      env_type: "node",
      env_version: nodeEnv.name,
    };
  }

  return {
    env_type: "python",
    env_version: "",
  };
};

export const createTaskDraft = (environments: Env[] = []): Partial<Task> => ({
  scriptSourceMode: "file",
  ...resolveDefaultEnvironment(environments),
});
