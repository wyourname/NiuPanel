import request from '../utils/request'
import type { ApiResponse, EncryptPayload, FileItem } from '@/types'

export const getSupportedVersions = (): Promise<ApiResponse<string[]>> => {
  return request.get('/compiler/versions')
}

export const getRecursiveScripts = (): Promise<ApiResponse<FileItem[]>> => {
  return request.get('/compiler/scripts')
}

export const encryptCode = (payload: EncryptPayload): Promise<ApiResponse<string>> => {
  return request.post('/compiler/encrypt', payload)
}
