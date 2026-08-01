import request from '../utils/request'
import { uploadMultipart, type UploadRequestOptions } from './upload'
import type {
  ApiResponse,
  BackupOptions,
  GeneralSettings,
  LogCleanupReport,
  MaintenanceStatus,
  NotificationSettings,
  NotificationTestRequest,
  ProfileUpdateRequest,
  SecuritySettings,
  SessionInfo,
  SettingItem,
  UpdateChannel,
  UpdateInfo,
  UpdateStatus,
} from '@/types'

/**
 * Basic Settings
 */
export const getSettings = (): Promise<ApiResponse<SettingItem[]>> => {
  return request.get('/settings')
}

export const updateGeneralSettings = (data: GeneralSettings): Promise<ApiResponse<void>> => {
  return request.put('/settings/general', data)
}

export const updateSecuritySettings = (data: SecuritySettings): Promise<ApiResponse<void>> => {
  return request.put('/settings/security', data)
}

export const updateProfile = (data: ProfileUpdateRequest): Promise<ApiResponse<void>> => {
  return request.put('/settings/profile', data)
}

/**
 * Session Management
 */
export const getActiveSessions = (): Promise<ApiResponse<SessionInfo[]>> => {
  return request.get('/settings/sessions')
}

export const revokeSession = (id: string): Promise<ApiResponse<void>> => {
  return request.delete(`/settings/sessions/${id}`)
}

/**
 * Notification Settings
 */
export const updateNotificationSettings = (data: NotificationSettings): Promise<ApiResponse<void>> => {
  return request.put('/settings/notification', data)
}

export const testNotification = (data: NotificationTestRequest): Promise<ApiResponse<void>> => {
  return request.post('/settings/test_notify', data)
}

/**
 * Maintenance & System Info
 */
export const getVersion = (): Promise<ApiResponse<string>> => {
  return request.get('/settings/version')
}

export const backupSystem = (params?: BackupOptions): Promise<ApiResponse<void>> => {
  return request.get('/settings/backup', { params })
}

export const getMaintenanceStatus = (): Promise<ApiResponse<MaintenanceStatus>> => {
  return request.get('/settings/maintenance/status')
}

export const downloadBackup = (filename: string): Promise<Blob> => {
  return request.get<Blob, Blob>(`/settings/backup/download/${filename}`, {
    responseType: 'blob',
    timeout: 0
  })
}

export const restoreSystem = (
  formData: FormData,
  options: UploadRequestOptions = {}
): Promise<ApiResponse<void>> => {
  return uploadMultipart('/settings/restore', formData, options)
}

export const cleanupLogs = (
  days = 15,
  dryRun = false,
): Promise<ApiResponse<LogCleanupReport>> => {
  return request.post('/settings/cleanup_logs', { days, dry_run: dryRun }, { timeout: 0 })
}

/**
 * System Updates
 */
export const checkUpdate = (): Promise<ApiResponse<UpdateInfo>> => {
  return request.get('/settings/update/check')
}

export const updateUpdateChannel = (channel: UpdateChannel): Promise<ApiResponse<void>> => {
  return request.put('/settings/update/channel', { channel })
}

export const executeUpdate = (): Promise<ApiResponse<void>> => {
  return request.post('/settings/update/execute', {}, { timeout: 300000 })
}

export const getUpdateStatus = (): Promise<ApiResponse<UpdateStatus>> => {
  return request.get('/settings/update/stats')
}

export const cancelUpdate = (): Promise<ApiResponse<void>> => {
  return request.post('/settings/update/cancel')
}
