export interface VariableSummary {
  id: number;
  key: string;
  scope: string;
  scope_id: number | null;
  task_ids?: number[];
  enabled: boolean;
  remarks: string | null;
  sort_order?: number;
  created_at?: string;
  updated_at?: string;
}

export interface Variable extends VariableSummary {
  value: string;
}

export interface VariableValue {
  id: number;
  key: string;
  value: string;
  updated_at: string;
}

export interface VariableListResponse {
  items: Variable[];
  total: number;
}

export interface VariableSummaryListResponse {
  items: VariableSummary[];
  total: number;
}

export interface VariableQueryParams {
  page?: number;
  page_size?: number;
  scope?: string;
  scope_id?: number;
  key?: string;
}

export interface VariableRequest {
  key: string;
  value: string;
  scope: string;
  scope_id?: number | null;
  scope_ids?: number[];
  remarks?: string | null;
  enabled?: boolean;
}

export type VariableUpdateRequest = VariableRequest;

export interface VariableReorderRequest {
  task_id?: number;
  scope?: string;
  ids: number[];
}

export interface TaskSimple {
  id: number;
  name: string;
}
