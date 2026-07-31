import request from '../utils/request'
import type { ApiResponse, PaginatedData } from '@/types'

export interface TelegramBotConfig {
  enabled: boolean;
  token: string;
  admin_chat_id: string;
  proxy_url?: string;
  api_base_url?: string;
  events: string[];
  // niupanel-proxy integration
  cf_proxy_enabled: boolean;
  cf_host: string;
  cf_ip: string;
  cf_token: string;
  login_2fa: boolean;
}

export interface TelegramBotMetrics {
  cpu_usage?: number;
  memory_usage?: number;
  disk_usage?: number;
  [key: string]: unknown;
}

export interface TelegramQueryParams {
  page?: number;
  limit?: number;
}

export interface TelegramCommand {
  id: number;
  name: string;
  script: string;
  created_at: number;
  updated_at: number;
}

export type TelegramCommandRequest = Pick<TelegramCommand, 'name' | 'script'>

export type TelegramWorkflowEventType =
  | 'alert'
  | 'cron'
  | 'failed'
  | 'success'
  | string

export type TelegramWorkflowActionType =
  | 'approval'
  | 'notify'
  | 'shell'
  | string

export interface TelegramWorkflow {
  id: number;
  event_type: TelegramWorkflowEventType;
  action_type: TelegramWorkflowActionType;
  config_json: string;
  created_at: number;
  updated_at: number;
}

export type TelegramWorkflowRequest = Pick<
  TelegramWorkflow,
  'action_type' | 'config_json' | 'event_type'
>

export const getTelegramConfig = (): Promise<ApiResponse<TelegramBotConfig>> => {
  return request.get('/bot')
}

export const updateTelegramConfig = (data: TelegramBotConfig): Promise<ApiResponse<void>> => {
  return request.put('/bot', data)
}

export const testTelegram = (data: TelegramBotConfig): Promise<ApiResponse<void>> => {
  return request.post('/bot/test', data)
}

export const getLatency = (): Promise<ApiResponse<number>> => {
  return request.get('/bot/latency')
}

export const sendMessage = (chat_id: string, text: string): Promise<ApiResponse<void>> => {
  return request.post('/bot/message', { chat_id, text })
}

export const sendFile = (chat_id: string, file: File): Promise<ApiResponse<void>> => {
  const formData = new FormData()
  formData.append('chat_id', chat_id)
  formData.append('file', file)
  return request.post('/bot/file', formData, {
    headers: {
      'Content-Type': 'multipart/form-data'
    }
  })
}

export const getBotMetrics = (): Promise<ApiResponse<TelegramBotMetrics>> => {
  return request.get('/bot/metrics')
}

export const sendServerFile = (chat_id: string, path: string): Promise<ApiResponse<void>> => {
  return request.post('/bot/server-file', { chat_id, path })
}

export const approveBind = (chat_id: string): Promise<ApiResponse<void>> => {
  return request.post('/bot/approve-bind', { chat_id })
}

// Custom Commands API
export const getCommands = (params: TelegramQueryParams): Promise<ApiResponse<PaginatedData<TelegramCommand>>> => {
  return request.get('/bot/commands', { params })
}

export const createCommand = (data: TelegramCommandRequest): Promise<ApiResponse<TelegramCommand>> => {
  return request.post('/bot/commands', data)
}

export const updateCommand = (id: number, data: TelegramCommandRequest): Promise<ApiResponse<TelegramCommand>> => {
  return request.put(`/bot/commands/${id}`, data)
}

export const deleteCommand = (id: number): Promise<ApiResponse<void>> => {
  return request.delete(`/bot/commands/${id}`)
}

// Workflows API
export const getWorkflows = (params: TelegramQueryParams): Promise<ApiResponse<PaginatedData<TelegramWorkflow>>> => {
  return request.get('/bot/workflows', { params })
}

export const createWorkflow = (data: TelegramWorkflowRequest): Promise<ApiResponse<TelegramWorkflow>> => {
  return request.post('/bot/workflows', data)
}

export const updateWorkflow = (id: number, data: TelegramWorkflowRequest): Promise<ApiResponse<TelegramWorkflow>> => {
  return request.put(`/bot/workflows/${id}`, data)
}

export const deleteWorkflow = (id: number): Promise<ApiResponse<void>> => {
  return request.delete(`/bot/workflows/${id}`)
}
