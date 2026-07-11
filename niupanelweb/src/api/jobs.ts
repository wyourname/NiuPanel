import request from '../utils/request'
import type { ApiResponse, Job } from '@/types'

export const getJobs = (): Promise<ApiResponse<Job[]>> => {
  return request.get('/jobs')
}

export const getJob = (id: number): Promise<ApiResponse<Job>> => {
  return request.get(`/jobs/${id}`)
}

export const cancelJob = (id: number): Promise<ApiResponse<void>> => {
  return request.post(`/jobs/${id}/cancel`)
}
