import {
  formatVariableText,
  parseVariableText,
} from "../utils/variableText";

export type TaskVariableRow = {
  id: number | null;
  key: string;
  value: string;
  remarks: string;
  enabled: boolean;
  scope: string;
  scope_id: number;
  task_ids?: number[];
  original_key?: string;
  original_value?: string;
  original_remarks?: string;
  statusLoading?: boolean;
  isNew?: boolean;
};

export type PersistedTaskVariableRow = TaskVariableRow & {
  id: number;
};

type RawTaskVariable = Omit<Partial<TaskVariableRow>, "scope_id" | "task_ids"> & {
  scope_id?: number | null;
  task_ids?: unknown;
};

const toFiniteNumber = (value: unknown) => {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
};

const normalizeTaskIds = (
  taskIds: unknown,
  scopeId: number | null | undefined,
  fallbackTaskId: number,
) => {
  if (Array.isArray(taskIds)) {
    const normalizedIds = taskIds.flatMap((item) => {
      const id = toFiniteNumber(item);
      return id === null ? [] : [id];
    });

    if (normalizedIds.length > 0) {
      return normalizedIds;
    }
  }

  const normalizedScopeId = toFiniteNumber(scopeId);
  return [normalizedScopeId ?? fallbackTaskId];
};

export const createTaskVariableRow = (
  taskId: number,
  values: Partial<TaskVariableRow> = {},
): TaskVariableRow => ({
  id: null,
  key: "",
  value: "",
  remarks: "",
  enabled: true,
  scope: "Script",
  scope_id: taskId,
  task_ids: [taskId],
  isNew: true,
  ...values,
});

export const hydrateTaskVariableRow = (
  variable: RawTaskVariable,
  taskId: number,
): TaskVariableRow => {
  const scopeId = variable.scope_id ?? taskId;
  const taskIds = normalizeTaskIds(variable.task_ids, variable.scope_id, taskId);

  return {
    id: variable.id ?? null,
    key: variable.key ?? "",
    value: variable.value ?? "",
    remarks: variable.remarks ?? "",
    enabled: variable.enabled ?? true,
    scope: variable.scope ?? "Script",
    scope_id: scopeId,
    task_ids: taskIds,
    original_key: variable.key ?? "",
    original_value: variable.value ?? "",
    original_remarks: variable.remarks ?? "",
    statusLoading: false,
    isNew: false,
  };
};

export const hasTaskVariableRowChanges = (variable: TaskVariableRow) => {
  return (
    variable.isNew ||
    variable.key !== variable.original_key ||
    variable.value !== variable.original_value ||
    variable.remarks !== variable.original_remarks
  );
};

export const isPersistedTaskVariableRow = (
  variable: TaskVariableRow,
): variable is PersistedTaskVariableRow => variable.id !== null;

export const collectPersistedTaskVariableIds = (
  variables: TaskVariableRow[],
) => {
  const ids: number[] = [];

  for (const variable of variables) {
    if (!isPersistedTaskVariableRow(variable)) {
      return null;
    }

    ids.push(variable.id);
  }

  return ids;
};

export const buildTaskVariableUpdatePayload = (variable: PersistedTaskVariableRow) => ({
  id: variable.id,
  key: variable.key,
  value: variable.value,
  remarks: variable.remarks,
  enabled: variable.enabled,
  scope: variable.scope,
  scope_id: variable.scope_id,
  scope_ids:
    Array.isArray(variable.task_ids) && variable.task_ids.length > 0
      ? [...variable.task_ids]
      : [variable.scope_id],
});

export const formatTaskVariablesSource = (variables: TaskVariableRow[]) => {
  return formatVariableText(variables);
};

export const applyTaskVariablesSource = (
  currentVariables: TaskVariableRow[],
  source: string,
  taskId: number,
) => {
  return parseVariableText(source).map((item) => {
    const existing = currentVariables.find((variable) => variable.key === item.key);

    if (existing) {
      return { ...existing, value: item.value || "" };
    }

    return createTaskVariableRow(taskId, {
      key: item.key,
      value: item.value || "",
    });
  });
};
