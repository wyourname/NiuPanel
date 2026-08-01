import type {
  ApiResponse,
  CoreReleaseList,
  CoreReleaseMutation,
  SystemVersionInfo,
  WebReleaseList,
  WebReleaseMutation,
  WebUpdateInfo
} from '@/types'
import request from '@/utils/request'
import { uploadMultipart, type UploadRequestOptions } from './upload'

export const getSystemMeta = (): Promise<ApiResponse<SystemVersionInfo>> => {
  return request.get('/system/meta')
}

export const getCoreReleases = (): Promise<ApiResponse<CoreReleaseList>> => {
  return request.get('/system/core/releases')
}

export const activateCoreRelease = (
  version: string,
  confirmDataLoss = false
): Promise<ApiResponse<CoreReleaseMutation>> => {
  return request.post(`/system/core/releases/${encodeURIComponent(version)}/activate`, {
    confirm_data_loss: confirmDataLoss
  })
}

export const getWebReleases = (): Promise<ApiResponse<WebReleaseList>> => {
  return request.get('/system/web/releases')
}

export const uploadWebRelease = (
  formData: FormData,
  activate = true,
  options: UploadRequestOptions = {}
): Promise<ApiResponse<WebReleaseMutation>> => {
  return uploadMultipart('/system/web/releases/upload', formData, {
    ...options,
    params: { activate }
  })
}

export const activateWebRelease = (version: string): Promise<ApiResponse<WebReleaseMutation>> => {
  return request.post(`/system/web/releases/${encodeURIComponent(version)}/activate`)
}

export const rollbackWebRelease = (): Promise<ApiResponse<WebReleaseMutation>> => {
  return request.post('/system/web/rollback')
}

export const checkWebUpdate = (): Promise<ApiResponse<WebUpdateInfo>> => {
  return request.get('/system/web/update/check')
}

export const installWebUpdate = (): Promise<ApiResponse<WebReleaseMutation>> => {
  return request.post('/system/web/update/install', undefined, { timeout: 1800000 })
}
