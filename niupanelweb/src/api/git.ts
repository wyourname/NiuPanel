import request from '../utils/request'
import type { ApiResponse } from '@/types'

export interface GitRepo {
  id: number;
  name: string;
  repo_url: string;
  branch: string;
  auth_token?: string;
  proxy_url?: string;
  auto_sync: boolean;
  last_sync_at: string | null;
  last_sync_status: string | null;
  last_sync_message: string | null;
  current_commit: string | null;
  created_at: string;
  updated_at: string;
}

export interface GitRepoRequest {
  name: string;
  repo_url: string;
  branch?: string;
  auth_token?: string;
  proxy_url?: string;
  auto_sync?: boolean;
}

export interface SyncResult {
  success: boolean;
  message: string;
  changes: string[];
}

export interface FileEntry {
  name: string;
  path: string;
  full_path?: string;
  is_dir: boolean;
  size: number;
}

export interface DiscoveredTask {
  name: string;
  command: string;
  cron: string | null;
  file_path: string;
  description: string | null;
}

export const listGitRepos = (): Promise<ApiResponse<GitRepo[]>> => {
  return request.get('/git')
}

export const createGitRepo = (data: GitRepoRequest): Promise<ApiResponse<GitRepo>> => {
  return request.post('/git', data)
}

export const updateGitRepo = (id: number, data: GitRepoRequest): Promise<ApiResponse<GitRepo>> => {
  return request.put(`/git/${id}`, data)
}

export const deleteGitRepo = (id: number): Promise<ApiResponse<void>> => {
  return request.delete(`/git/${id}`)
}

export const syncGitRepo = (id: number): Promise<ApiResponse<SyncResult>> => {
  return request.post(`/git/${id}/sync`)
}

export const getRepoFiles = (id: number, path?: string): Promise<ApiResponse<FileEntry[]>> => {
  return request.get(`/git/${id}/files`, { params: { path } })
}

export const scanRepoTasks = (id: number): Promise<ApiResponse<DiscoveredTask[]>> => {
  return request.get(`/git/${id}/scan`)
}

export const importRepoTasks = (id: number, tasks: DiscoveredTask[]): Promise<ApiResponse<void>> => {
  return request.post(`/git/${id}/import`, { tasks })
}
