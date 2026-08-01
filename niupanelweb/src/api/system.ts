import type {
  ApiResponse,
  PanelReleaseList,
  PanelReleaseMutation,
  SystemVersionInfo
} from '@/types'
import request from '@/utils/request'

export const getSystemMeta = (): Promise<ApiResponse<SystemVersionInfo>> => {
  return request.get('/system/meta')
}

export const getPanelReleases = (): Promise<ApiResponse<PanelReleaseList>> => {
  return request.get('/system/releases')
}

export const rollbackPanelRelease = (
  version: string,
  confirmDataLoss: boolean
): Promise<ApiResponse<PanelReleaseMutation>> => {
  return request.post(`/system/releases/${encodeURIComponent(version)}/rollback`, {
    confirm_data_loss: confirmDataLoss
  })
}
