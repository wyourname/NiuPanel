import request from '../utils/request'
import type {
  ApiResponse,
  TaskSimple,
  Variable,
  VariableListResponse,
  VariableQueryParams,
  VariableReorderRequest,
  VariableRequest,
  VariableSummaryListResponse,
  VariableUpdateRequest,
  VariableValue,
} from '@/types'

/**
 * 获取变量列表
 */
export const getVariables = (params: VariableQueryParams): Promise<ApiResponse<VariableSummaryListResponse>> => {
  return request.get('/variables', { params })
}

export const getVariablesWithValues = (params: VariableQueryParams): Promise<ApiResponse<VariableListResponse>> => {
  return request.get('/variables/with-values', { params })
}

export const getVariableValue = (id: number): Promise<ApiResponse<VariableValue>> => {
  return request.get(`/variables/${id}/value`)
}

/**
 * 获取特定任务的变量
 */
export const getVariablesByTaskId = (taskId: number): Promise<ApiResponse<VariableListResponse>> => {
  return request.get('/variables/with-values', {
    params: {
      scope: 'Script',
      scope_id: taskId,
      page_size: 1000
    }
  })
}

/**
 * 获取所有任务 (简化版，仅包含 ID 和名称)
 */
export const getAllTasksSimple = (): Promise<ApiResponse<TaskSimple[]>> => {
  return request.get('/variables/tasks/all')
}

/**
 * 创建变量
 */
export const createVariable = (data: VariableRequest): Promise<ApiResponse<Variable>> => {
  return request.post('/variables', data)
}

/**
 * 更新变量
 */
export const updateVariable = (id: number, data: VariableUpdateRequest): Promise<ApiResponse<Variable>> => {
  return request.patch(`/variables/${id}`, data)
}

/**
 * 批量更新 (由于后端不直接支持，前端仍保留循环调用逻辑)
 */
export const updateVariables = (variables: Array<VariableUpdateRequest & { id: number }>): Promise<Array<ApiResponse<Variable>>> => {
  return Promise.all(variables.map(v => updateVariable(v.id, v)))
}

export const saveTaskVariables = (data: {
  task_id: number;
  upserts: Array<{
    id?: number;
    variable: VariableRequest;
  }>;
  delete_ids: number[];
}): Promise<ApiResponse<Variable[]>> => {
  return request.post('/variables/batch-save', data)
}

/**
 * 切换启用状态 (支持单个或多个)
 */
export const toggleVariables = (ids: number[], enabled: boolean): Promise<ApiResponse<void>> => {
  return request.post('/variables/toggle', { ids, enabled })
}

export const reorderVariables = (data: VariableReorderRequest): Promise<ApiResponse<void>> => {
  return request.post('/variables/reorder', data)
}

/**
 * 导入变量
 */
export const importVariables = (data: VariableRequest[]): Promise<ApiResponse<number>> => {
  return request.post('/variables/import', data)
}

/**
 * 删除变量 (支持单个或多个)
 */
export const deleteVariables = (ids: number[], taskId?: number): Promise<ApiResponse<void>> => {
  return request.delete('/variables', {
    data: { ids, task_id: taskId }
  })
}
