import request from '../utils/request';
import type {
  ApiResponse,
  ConfirmImportRequest,
  CreateMarketSourceRequest,
  CreateShareRequest,
  CreateShareResponse,
  DeleteImportedTasksParams,
  ImportSourceGroup,
  ImportStatus,
  ImportSummary,
  MarketScriptAggregated,
  MarketSource,
  NiuPackage,
  StationConfigPayload,
  StationFile,
  StationStats,
  SubmitImportRequest,
  SubmitImportResponse,
  TaskFileTreeData,
  TransferStatus,
  TransferUploadRequest,
  UpdateStationFileRequest,
} from '@/types'

// --- Share Creation ---

export function getTaskFileTree(taskIds: number[]): Promise<ApiResponse<TaskFileTreeData>> {
  return request({
    url: '/share/tasks/files',
    method: 'post',
    data: taskIds
  });
}

export function createShare(data: CreateShareRequest): Promise<ApiResponse<CreateShareResponse>> {
  return request({
    url: '/share/create',
    method: 'post',
    data
  });
}

// --- Import Flow ---

export function submitImport(data: SubmitImportRequest): Promise<ApiResponse<SubmitImportResponse>> {
  return request({
    url: '/share/import/submit',
    method: 'post',
    data // { url, password }
  });
}

export function retryImport(stagingId: string, data: SubmitImportRequest): Promise<ApiResponse<void>> {
  return request({
    url: `/share/import/${stagingId}/retry`,
    method: 'post',
    data // { url, password }
  });
}

export function getImportStatus(stagingId: string): Promise<ApiResponse<ImportStatus>> {
  return request({
    url: `/share/import/${stagingId}/status`,
    method: 'get'
  });
}

export function getImportPreview(stagingId: string): Promise<ApiResponse<NiuPackage>> {
  return request({
    url: `/share/import/${stagingId}/preview`,
    method: 'get'
  });
}

export function confirmImport(stagingId: string, data: ConfirmImportRequest): Promise<ApiResponse<ImportSummary>> {
  return request({
    url: `/share/import/${stagingId}/confirm`,
    method: 'post',
    data // { selected_tasks, update_existing }
  });
}

export function getImportHistory(): Promise<ApiResponse<ImportSourceGroup[]>> {
  return request({
    url: '/share/import/history',
    method: 'get'
  });
}

export function deleteImportedTasks(params: DeleteImportedTasksParams): Promise<ApiResponse<number>> {
  return request({
    url: '/share/import/tasks',
    method: 'delete',
    params
  });
}

// --- Station Management ---

export function listStationFiles(): Promise<ApiResponse<StationFile[]>> {
  return request({
    url: '/share/station/list',
    method: 'get'
  })
}

export function getStationStats(): Promise<ApiResponse<StationStats>> {
  return request({
    url: '/share/station/stats',
    method: 'get'
  })
}

export function saveStationConfig(data: StationConfigPayload): Promise<ApiResponse<void>> {
  return request({
    url: '/share/station/config',
    method: 'post',
    data
  })
}

export function updateStationFile(token: string, data: UpdateStationFileRequest): Promise<ApiResponse<void>> {
  return request({
    url: `/share/station/${token}`,
    method: 'patch',
    data
  })
}

export function updateStationContent(token: string): Promise<ApiResponse<void>> {
  return request({
    url: `/share/station/${token}/content`,
    method: 'post'
  })
}

export function deleteStationFile(token: string): Promise<ApiResponse<void>> {
  return request({
    url: `/share/station/${token}`,
    method: 'delete'
  })
}

// --- Market Management ---

export function listMarketSources(): Promise<ApiResponse<MarketSource[]>> {
  return request({
    url: '/share/market/sources',
    method: 'get'
  })
}

export function addMarketSource(data: CreateMarketSourceRequest): Promise<ApiResponse<MarketSource>> {
  return request({
    url: '/share/market/sources',
    method: 'post',
    data
  })
}

export function deleteMarketSource(id: number): Promise<ApiResponse<number>> {
  return request({
    url: `/share/market/sources/${id}`,
    method: 'delete'
  })
}

export function syncMarketSource(id: number): Promise<ApiResponse<void>> {
  return request({
    url: `/share/market/sources/${id}/sync`,
    method: 'post'
  })
}

export function listMarketScripts(): Promise<ApiResponse<MarketScriptAggregated[]>> {
  return request({
    url: '/share/market/scripts',
    method: 'get'
  })
}

export function uploadToTransferStation(shareId: string, data: TransferUploadRequest): Promise<ApiResponse<void>> {
  return request({
    url: `/share/transfer/${shareId}/upload`,
    method: 'post',
    data
  })
}

export function getTransferStatus(shareId: string): Promise<ApiResponse<TransferStatus>> {
  return request({
    url: `/share/transfer/${shareId}/status`,
    method: 'get'
  })
}
