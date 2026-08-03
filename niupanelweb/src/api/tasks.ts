import request from '../utils/request'
import type {
  ApiResponse,
  CreateTaskRequest,
  Task,
  TaskListResponse,
  TaskLogResponse,
  TaskRunHistoryResponse,
  TaskRunResult,
  UpdateTaskRequest,
} from '@/types'

/**
 * 获取任务列表
 */
export const getTasks = (page?: number, pageSize?: number, q?: string, status?: string): Promise<ApiResponse<TaskListResponse>> => {
  return request.get('/tasks', {
    params: {
      page,
      page_size: pageSize,
      q,
      status
    }
  })
}

/**
 * 创建任务
 */
export const createTask = (data: CreateTaskRequest): Promise<ApiResponse<Task>> => {
  return request.post('/tasks', data)
}

/**
 * 更新任务详情
 */
export const updateTask = (id: number, data: UpdateTaskRequest): Promise<ApiResponse<Task>> => {
  return request.patch(`/tasks/${id}`, data)
}

/**
 * 删除任务 (支持单个或多个)
 */
export const deleteTasks = (ids: number[], deleteVar = false, deleteScript = false): Promise<ApiResponse<void>> => {
  return request.delete('/tasks', {
    data: { ids },
    params: {
        delete_var: deleteVar,
        delete_script: deleteScript
    }
  })
}

/**
 * 运行任务 (支持单个或多个)
 */
export const runTasks = (ids: number[]): Promise<ApiResponse<TaskRunResult[]>> => {
  return request.post('/tasks/run', { ids })
}

/**
 * 停止任务 (支持单个或多个)
 */
export const stopTasks = (ids: number[]): Promise<ApiResponse<TaskRunResult[]>> => {
  return request.post('/tasks/stop', { ids })
}

/**
 * 暂停任务 (支持单个或多个)
 */
export const pauseTasks = (ids: number[]): Promise<ApiResponse<TaskRunResult[]>> => {
  return request.post('/tasks/pause', { ids })
}

/**
 * 恢复任务 (支持单个或多个)
 */
export const resumeTasks = (ids: number[]): Promise<ApiResponse<TaskRunResult[]>> => {
  return request.post('/tasks/resume', { ids })
}

/**
 * 启用任务 (支持单个或多个)
 */
export const enableTasks = (ids: number[]): Promise<ApiResponse<void>> => {
  return request.post('/tasks/enable', { ids })
}

/**
 * 禁用任务 (支持单个或多个)
 */
export const disableTasks = (ids: number[]): Promise<ApiResponse<void>> => {
  return request.post('/tasks/disable', { ids })
}

/**
 * 置顶任务 (支持单个或多个)
 */
export const pinTasks = (ids: number[]): Promise<ApiResponse<void>> => {
  return request.post('/tasks/pin', { ids })
}

/**
 * 取消置顶 (支持单个或多个)
 */
export const unpinTasks = (ids: number[]): Promise<ApiResponse<void>> => {
  return request.post('/tasks/unpin', { ids })
}

// --- 监控与日志 (保持 ID 路径，因为这是获取特定资源的详情) ---

export const getLatestLog = (id: number, offset: number | null = null, limit: number | null = null): Promise<ApiResponse<TaskLogResponse>> => {
  return request.get(`/tasks/${id}/logs/latest`, {
      params: { offset, limit },
      skipMessage: true
  })
}

export const getTaskHistory = (id: number, page = 1, pageSize = 10): Promise<ApiResponse<TaskRunHistoryResponse>> => {
  return request.get(`/tasks/${id}/history`, {
      params: {
          page,
          page_size: pageSize
      }
  })
}

export const getTaskRunLog = (id: number, runId: number, offset: number | null = null, limit: number | null = null): Promise<ApiResponse<TaskLogResponse>> => {
  return request.get(`/tasks/${id}/runs/${runId}/log`, {
      params: { offset, limit },
      skipMessage: true
  })
}

export const streamTaskLogs = (id: number): EventSource => {
  return new EventSource(`${request.defaults.baseURL}/tasks/${id}/logs`)
}

export const streamTaskRunLogs = (id: number, runId: number): EventSource => {
  return new EventSource(`${request.defaults.baseURL}/tasks/${id}/runs/${runId}/logs`)
}

export const streamTaskStatus = (): EventSource => {
  return new EventSource(`${request.defaults.baseURL}/tasks/status`)
}

// Pagination Settings
export const getPaginationSetting = (): Promise<ApiResponse<number | null>> => {
  return request.get('/tasks/settings/pagination')
}

export const savePaginationSetting = (pageSize: number): Promise<ApiResponse<void>> => {
  return request.post('/tasks/settings/pagination', { page_size: pageSize })
}

export const quickCreateFromUrl = (url: string): Promise<ApiResponse<Task>> => {
  return request.post('/tasks/quick_create', { url })
}
