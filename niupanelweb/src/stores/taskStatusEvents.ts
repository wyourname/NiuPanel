import type { Task, TaskStatus } from "@/types";

type JsonObject = Record<string, unknown>;

export type TaskStatusUpdate = {
  raw: JsonObject;
  taskId: number | null;
  jobId: number | null;
  status?: TaskStatus;
  runId?: number | null;
  pid?: number | null;
  cpuUsage?: number;
  memoryUsage?: number;
  isSystem: boolean;
};

const taskStatuses: readonly TaskStatus[] = [
  "Pending",
  "Running",
  "Finished",
  "Failed",
  "Cancelled",
  "Stopped",
  "Paused",
  "Idle",
];

const terminalTaskStatuses: readonly TaskStatus[] = [
  "Finished",
  "Failed",
  "Stopped",
];

const finishedSystemJobStatuses: readonly TaskStatus[] = [
  "Finished",
  "Failed",
  "Cancelled",
];

const isObject = (value: unknown): value is JsonObject =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const toNumber = (value: unknown) => {
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value === "string" && value.trim()) {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) return parsed;
  }
  return null;
};

const toTaskStatus = (value: unknown) =>
  typeof value === "string" && taskStatuses.includes(value as TaskStatus)
    ? (value as TaskStatus)
    : undefined;

const unwrapStatusPayload = (payload: JsonObject) => {
  const taskEnvelope = payload.Task;
  if (isObject(taskEnvelope) && isObject(taskEnvelope.StatusChanged)) {
    return taskEnvelope.StatusChanged;
  }

  if (isObject(payload.TaskStatusChanged)) {
    return payload.TaskStatusChanged;
  }

  if (typeof payload.status === "string" && (payload.task_id || payload.job_id)) {
    return payload;
  }

  return null;
};

export const parseTaskStatusPayload = (payload: string) => {
  const parsed: unknown = JSON.parse(payload);
  if (!isObject(parsed)) return null;

  const data = unwrapStatusPayload(parsed);
  if (!data || !(data.task_id || data.job_id)) return null;

  return {
    raw: data,
    taskId: toNumber(data.task_id),
    jobId: toNumber(data.job_id),
    status: toTaskStatus(data.status),
    runId: data.run_id === null ? null : toNumber(data.run_id) ?? undefined,
    pid: data.pid === null ? null : toNumber(data.pid) ?? undefined,
    cpuUsage: toNumber(data.cpu_usage) ?? undefined,
    memoryUsage: toNumber(data.memory_usage) ?? undefined,
    isSystem: Boolean(data.is_system),
  } satisfies TaskStatusUpdate;
};

export const isFinishedSystemJob = (update: TaskStatusUpdate) =>
  update.isSystem &&
  update.status !== undefined &&
  finishedSystemJobStatuses.includes(update.status);

export const applyTaskStatusUpdate = (
  tasks: Task[],
  update: TaskStatusUpdate,
  now = new Date(),
) => {
  if (update.taskId === null || update.taskId === 0) return;

  const task = tasks.find((item) => Number(item.id) === update.taskId);
  if (!task) return;

  if (update.status && task.status !== update.status) {
    task.status = update.status;
  }

  if (update.pid !== undefined && task.pid !== update.pid) {
    task.pid = update.pid;
  }

  if (update.runId !== undefined && task.run_id !== update.runId) {
    task.run_id = update.runId;
  }

  if (update.cpuUsage !== undefined) {
    task.cpu_usage = update.cpuUsage;
  }

  if (update.memoryUsage !== undefined) {
    task.memory_usage = update.memoryUsage;
  }

  if (update.status && terminalTaskStatuses.includes(update.status)) {
    task.last_finished_at = now.toISOString();
    task.pid = null;
    task.cpu_usage = undefined;
    task.memory_usage = undefined;
  }
};
