import request from '../utils/request'
import type { ApiResponse } from '@/types'

export interface WebhookPushRequest {
  title: string;
  content: string;
  level?: string;
}

export const pushNotification = (data: WebhookPushRequest): Promise<ApiResponse<void>> => {
  return request.post('/webhook/push', data)
}
