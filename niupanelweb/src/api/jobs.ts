import request from '../utils/request'
import type { ApiResponse, Job } from '@/types'
import type { TaskLogResponse } from '@/types/tasks'

export const getJobLogContent = (id: number | string, offset: number, limit: number): Promise<ApiResponse<TaskLogResponse>> =>
  request.get(`/jobs/${encodeURIComponent(id)}/logs/content`, { params: { offset, limit }, skipMessage: true })

export const getJobs = (options: { skipMessage?: boolean } = {}): Promise<ApiResponse<Job[]>> => {
  return request.get('/jobs', options)
}

export const getJob = (id: number): Promise<ApiResponse<Job>> => {
  return request.get(`/jobs/${id}`, { skipMessage: true })
}

export const cancelJob = (id: number): Promise<ApiResponse<void>> => {
  return request.post(`/jobs/${id}/cancel`)
}
