import request from '../utils/request'
import type { ApiResponse, FileItem, FileListQueryParams } from '@/types'
import { uploadMultipart, type UploadRequestOptions } from './upload'

const BASE_URL = '/files'

// Helper to construct path for /scripts or /scripts/{*path}
const getScriptsPath = (path: string) => {
  if (path === '' || path === '/') {
    return `${BASE_URL}/scripts`
  }
  // Remove leading slash if exists
  const cleanPath = path.startsWith('/') ? path.substring(1) : path
  return `${BASE_URL}/scripts/${cleanPath}`
}

export const listDirectoryContents = (path = '/', params?: FileListQueryParams): Promise<ApiResponse<FileItem[]>> => {
  return request.get(getScriptsPath(path), { params })
}

export const readFileContent = (path: string): Promise<ApiResponse<string>> => {
  return request.get(`${BASE_URL}/file/${path}`)
}

export const writeFileContent = (path: string, content: string): Promise<ApiResponse<void>> => {
  return request.put(`${BASE_URL}/file`, { path, content })
}

export const createFile = (path: string, content = ''): Promise<ApiResponse<void>> => {
  return request.post(`${BASE_URL}/file`, { path, content })
}

export const createDirectory = (path: string): Promise<ApiResponse<void>> => {
  return request.post(`${BASE_URL}/directory`, { path })
}

export const deleteItem = (path: string): Promise<ApiResponse<void>> => {
  return request.delete(getScriptsPath(path))
}

export const renameItem = (old_path: string, new_path: string): Promise<ApiResponse<void>> => {
  return request.post(`${BASE_URL}/rename`, { old_path, new_path })
}

export const copyItem = (from_path: string, to_path: string): Promise<ApiResponse<void>> => {
  return request.post(`${BASE_URL}/copy`, { from_path, to_path })
}

export const extractArchive = (path: string, destination_path?: string): Promise<ApiResponse<number>> => {
  return request.post(`${BASE_URL}/extract`, { path, destination_path })
}

export const uploadFile = (
  path: string,
  formData: FormData,
  options: UploadRequestOptions = {}
): Promise<ApiResponse<void>> => {
  const url = path && path !== '/' ? `${BASE_URL}/upload/${path}` : `${BASE_URL}/upload`;
  return uploadMultipart(url, formData, { ...options, skipMessage: true })
}

export const downloadFile = (path: string): Promise<Blob> => {
  return request.get<Blob, Blob>(`${BASE_URL}/download/${path}`, {
    responseType: 'blob'
  })
}

export const downloadFromUrl = (url: string, path: string, filename?: string): Promise<ApiResponse<void>> => {
  return request.post(`${BASE_URL}/download_url`, { url, path, filename }, { timeout: 0 })
}

export const downloadBatch = (paths: string[]): Promise<Blob> => {
  return request.post<Blob, Blob>(`${BASE_URL}/download_batch`, { paths }, {
    responseType: 'blob'
  })
}
