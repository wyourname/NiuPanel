import request from '../utils/request'
import type { ApiResponse } from '@/types'

export interface TelegramChatBinding {
  chat_id: string;
  thread_id: number | null;
  user_id: number;
  label: string;
  events: string[];
  telegram_user_ids: string[];
  interaction_mode: "direct" | "mention";
}

export interface TelegramUserOption {
  id: number;
  username: string;
  role: string;
}

export interface TelegramBotConfig {
  enabled: boolean;
  token: string;
  token_present: boolean;
  clear_token: boolean;
  chat_bindings: TelegramChatBinding[];
  proxy_url?: string;
  api_base_url?: string;
  // niupanel-proxy integration
  cf_proxy_enabled: boolean;
  cf_host: string;
  cf_ip: string;
  cf_token: string;
  cf_token_present: boolean;
  clear_cf_token: boolean;
  agent_plugin_id: string;
}

export const getTelegramConfig = (): Promise<ApiResponse<TelegramBotConfig>> => {
  return request.get('/bot')
}

export const updateTelegramConfig = (data: TelegramBotConfig): Promise<ApiResponse<void>> => {
  return request.put('/bot', data)
}

export const testTelegram = (data: TelegramBotConfig): Promise<ApiResponse<void>> => {
  return request.post('/bot/test', data)
}

export const getTelegramUsers = (): Promise<ApiResponse<TelegramUserOption[]>> => {
  return request.get('/bot/users')
}
