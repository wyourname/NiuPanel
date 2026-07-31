import request from '../utils/request'
import type { ApiResponse, OverviewData } from '@/types'

export const getSystemOverview = (): Promise<ApiResponse<OverviewData>> => {
  return request.get('/overview')
}
