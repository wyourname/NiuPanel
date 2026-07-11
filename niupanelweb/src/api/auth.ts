import request from '../utils/request'
import type {
  ApiResponse,
  LoginRequest,
  LoginResponse,
  RegisterRequest,
  ResetPasswordRequest,
  SetupStatus,
  UserInfo,
  VerifyLogin2FARequest,
} from '@/types'

export const getSetupStatus = (): Promise<ApiResponse<SetupStatus>> => {
  return request.get('/auth/status')
}

export const register = (data: RegisterRequest): Promise<ApiResponse<void>> => {
  return request.post('/auth/register', data)
}

export const login = (data: LoginRequest): Promise<ApiResponse<LoginResponse>> => {
  return request.post('/auth/login', data, { timeout: 60000 })
}

export const verifyLogin2FA = (data: VerifyLogin2FARequest): Promise<ApiResponse<UserInfo>> => {
  return request.post('/auth/login/verify-2fa', data)
}

export const logout = (): Promise<ApiResponse<void>> => {
  return request.get('/auth/logout')
}

export const identifyReset = (username: string): Promise<ApiResponse<{ suffix: string }>> => {
  return request.post('/auth/identify-reset', { username })
}

export const forgotPassword = (username: string, email: string): Promise<ApiResponse<void>> => {
  return request.post('/auth/forgot-password', { username, email })
}

export const verifyResetCode = (email: string, code: string): Promise<ApiResponse<string>> => {
  return request.post('/auth/verify-reset-code', { email, code })
}

export const resetPassword = (data: ResetPasswordRequest): Promise<ApiResponse<void>> => {
  return request.post('/auth/reset-password', data)
}
