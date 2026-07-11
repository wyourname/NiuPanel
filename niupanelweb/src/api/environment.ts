import request from '../utils/request'
import type {
  ApiResponse,
  Env,
  EnvType,
  InstallableEnvType,
  InstallPackagesRequest,
  PackageListPayload,
} from '@/types'

export const getEnvironments = (): Promise<ApiResponse<Env[]>> => {
  return request.get('/environments')
}

export const createEnvironment = (data: { version: string }, envType: InstallableEnvType = 'python'): Promise<ApiResponse<string>> => {
  return request.post(`/environments/${envType}`, data)
}

export const deleteEnvironment = (env: Env): Promise<ApiResponse<void>> => {
  if (env.env_type === 'python') {
    return request.delete(`/environments/python/${env.name}`)
  } else if (env.env_type === 'node') {
    return request.delete(`/environments/node/${env.name}`)
  }
  throw new Error('Cannot delete this environment type')
}

export const getPackages = (env: Env): Promise<ApiResponse<PackageListPayload>> => {
  if (env.env_type === 'python') {
    return request.get(`/environments/python/${env.name}/packages`)
  } else if (env.env_type === 'node') {
    return request.get(`/environments/node/${env.name}/packages`)
  } else if (env.env_type === 'sh') {
    return request.get(`/environments/shell/packages`)
  }
  return Promise.reject('Unknown env type')
}

export const installPackages = (env: Env, data: InstallPackagesRequest): Promise<ApiResponse<string>> => {
  if (env.env_type === 'python') {
    return request.post(`/environments/python/${env.name}/packages`, data)
  } else if (env.env_type === 'node') {
    return request.post(`/environments/node/${env.name}/packages`, data)
  } else if (env.env_type === 'sh') {
    return request.post(`/environments/shell/packages`, data)
  }
  return Promise.reject('Unknown env type')
}

export const uninstallPackage = (env: Env, pkg: string): Promise<ApiResponse<string | void>> => {
  if (env.env_type === 'python') {
    return request.delete(`/environments/python/${env.name}/packages/${pkg}`)
  } else if (env.env_type === 'node') {
    return request.delete(`/environments/node/${env.name}/packages/${pkg}`)
  } else if (env.env_type === 'sh') {
    return request.delete(`/environments/shell/packages/${pkg}`)
  }
  return Promise.reject('Unknown env type')
}

export const getAvailableVersions = (): Promise<ApiResponse<string>> => {
  return request.get('/environments/versions')
}

export const setMirrorSource = (envType: EnvType, mirrorUrl: string): Promise<ApiResponse<void>> => {
  return request.post(`/environments/mirror/${envType}`, { mirror_url: mirrorUrl })
}

export const setNodeDefault = (version: string): Promise<ApiResponse<void>> => {
  return request.post(`/environments/node/${encodeURIComponent(version)}/set-default`, {})
}
