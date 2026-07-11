import request from '../utils/request'
import type { ApiResponse } from '@/types'

export interface ApiKey {
  id: number;
  name: string;
  prefix: string;
  permissions?: string;
  created_at: number;
  expires_at?: number;
  last_used_at?: number;
  last_used_ip?: string;
}

export interface CreateApiKeyRequest {
  name: string;
  permissions?: string;
  expires_in_days?: number;
}

export interface UpdateApiKeyRequest {
  name?: string;
  permissions?: string;
  expires_at?: number;
}

export interface CreateApiKeyResponse {
  id: number;
  name: string;
  token: string;
}

export const listApiKeys = (): Promise<ApiResponse<ApiKey[]>> => {
  return request.get('/keys')
}

export const createApiKey = (data: CreateApiKeyRequest): Promise<ApiResponse<CreateApiKeyResponse>> => {
  return request.post('/keys', data)
}

export const updateApiKey = (id: number, data: UpdateApiKeyRequest): Promise<ApiResponse<void>> => {
  return request.patch(`/keys/${id}`, data)
}

export const deleteApiKey = (id: number): Promise<ApiResponse<void>> => {
  return request.delete(`/keys/${id}`)
}