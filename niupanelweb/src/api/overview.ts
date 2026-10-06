import request from '../utils/request'
import type { ApiResponse, OverviewData, SystemMetrics } from '@/types'

export const getSystemOverview = (): Promise<ApiResponse<OverviewData>> => {
  return request.get('/overview')
}

export const getSystemMetrics = (signal?: AbortSignal): Promise<ApiResponse<SystemMetrics>> => {
  return request.get('/overview/metrics', { signal, skipMessage: true, timeout: 10000 })
}
