import type { Variable, VariableListResponse } from "./variable";

export type TaskStatus =
  | "Pending"
  | "Running"
  | "Finished"
  | "Failed"
  | "Cancelled"
  | "Stopped"
  | "Paused"
  | "Idle";

export interface TaskRandomConfig {
  start: string;
  end: string;
  count: number;
}

export interface TaskVariablePayload {
  key: string;
  value: string;
}

export interface Task {
  id: number;
  name: string;
  path: string;
  command?: string | null;
  description?: string | null;
  cron_schedule?: string | null;
  enabled: boolean;
  status: TaskStatus;
  pid?: number | null;
  run_id?: number | null;
  notify?: boolean;
  tags?: unknown;
  env_type: string;
  env_version?: string | null;
  user_id?: number;
  created_at?: string;
  updated_at?: string;
  requirements?: string | null;
  last_finished_at?: string | null;
  next_run_at?: string | null;
  cpu_limit?: number | null;
  timeout_sec?: number | null;
  memory_limit?: number | null;
  is_pinned?: boolean;
  trigger_next_tasks?: number[] | null;
  import_source?: string | null;
  remote_task_id?: string | null;
  share_code?: string | null;
  random_config?: TaskRandomConfig | null;
  cpu_usage?: number;
  memory_usage?: number;
  [key: string]: unknown;
}

export interface TaskListResponse {
  items: Task[];
  total: number;
}

export interface CreateTaskRequest {
  name: string;
  path?: string | null;
  command?: string | null;
  description?: string | null;
  env_type: string;
  env_version?: string | null;
  tags?: unknown;
  cron_schedule?: string | null;
  requirements?: string | null;
  variables?: TaskVariablePayload[];
  notify?: boolean;
  cpu_limit?: number | null;
  timeout_sec?: number | null;
  memory_limit?: number | null;
  trigger_next_tasks?: number[];
  random_config?: TaskRandomConfig | null;
}

export type UpdateTaskRequest = Partial<
  Omit<CreateTaskRequest, "random_config">
> & {
  enabled?: boolean;
  random_config?: TaskRandomConfig | null;
};

export interface TaskRunResult {
  task_id: number;
  status: "success" | "error";
  message: string;
  run_id?: number;
}

export interface TaskLogResponse {
  content: string;
  total_size: number;
  offset: number;
  length: number;
}

export interface TaskRunHistoryItem {
  id: number;
  task_id: number;
  status: TaskStatus;
  started_at: string;
  ended_at?: string | null;
  log_path?: string | null;
  pid?: number | null;
}

export interface TaskRunHistoryResponse {
  items: TaskRunHistoryItem[];
  total: number;
}

export type TaskVariableListResponse = VariableListResponse;

export type TaskVariable = Variable;
