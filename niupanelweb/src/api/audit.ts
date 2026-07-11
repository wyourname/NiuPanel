import request from '../utils/request'
import type { ApiResponse, PaginatedData } from '@/types'

export interface AuditLog {
  id: number;
  user_id?: number;
  actor_type: 'User' | 'Key';
  action: string;
  resource: string;
  resource_id?: string;
  details?: string;
  ip_address?: string;
  created_at: string;
}

export interface AuditQueryParams {
  page?: number;
  page_size?: number;
}

export const listAuditLogs = (params: AuditQueryParams): Promise<ApiResponse<PaginatedData<AuditLog>>> => {
  return request.get('/audit', { params })
}
