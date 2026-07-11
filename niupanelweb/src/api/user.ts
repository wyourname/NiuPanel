import request from '../utils/request'
import type {
  ApiResponse,
  ChangePasswordRequest,
  EmailChangeRequest,
  UserInfo,
  UserPreferences,
} from '@/types'

/**
 * 需要认证的个人资料接口
 */
export const getMyProfile = (): Promise<ApiResponse<UserInfo>> => {
  return request.get('/users/me')
}

export const updateProfile = (data: { username: string; password_confirm: string }): Promise<ApiResponse<void>> => {
  return request.put('/users/profile', data)
}

export const changePassword = (data: ChangePasswordRequest): Promise<ApiResponse<void>> => {
  return request.put('/users/password', data)
}

export const updatePreferences = (prefs: UserPreferences): Promise<ApiResponse<void>> => {
  return request.put('/users/preferences', { preferences: prefs })
}

export const requestEmailChange = (data: EmailChangeRequest): Promise<ApiResponse<void>> => {
  return request.post('/users/email/verify-request', data)
}
