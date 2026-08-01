import request from "@/utils/request";
import { uploadMultipart, type UploadRequestOptions } from "./upload";
import { storageKey as makeStorageKey } from "@/utils/storage";
import type {
  ApiResponse,
  PluginAppRecord,
  PluginHealthReport,
  PluginInstallRequest,
  PluginImpactPreview,
  PluginMarketIndex,
  PluginMarketInstallRequest,
  PluginMarketSource,
  PluginMarketSourcesUpdateRequest,
  PluginMarketUpdateRecord,
  PluginRecord,
  PluginThemeRecord,
  PluginUpdateRequest,
  PluginVersionRecord,
} from "@/types";

const serverUrlKey = makeStorageKey("server_url");
export const listPluginApps = (): Promise<ApiResponse<PluginAppRecord[]>> => {
  return request.get("/plugins/apps", { skipMessage: true });
};

export const listPluginThemes = (): Promise<ApiResponse<PluginThemeRecord[]>> => {
  return request.get("/plugins/themes", { skipMessage: true });
};

export const listPluginHealth = (): Promise<ApiResponse<PluginHealthReport[]>> => {
  return request.get("/plugins/health", { skipMessage: true });
};

export const listPlugins = (): Promise<ApiResponse<PluginRecord[]>> => {
  return request.get("/plugins");
};

export const getPluginMarket = (
  indexUrl: string,
): Promise<ApiResponse<PluginMarketIndex>> => {
  return request.get("/plugins/market", {
    params: { index_url: indexUrl },
  });
};

export const listPluginMarketSources = (): Promise<ApiResponse<PluginMarketSource[]>> => {
  return request.get("/plugins/market/sources", { skipMessage: true });
};

export const updatePluginMarketSources = (
  data: PluginMarketSourcesUpdateRequest,
): Promise<ApiResponse<PluginMarketSource[]>> => {
  return request.put("/plugins/market/sources", data);
};

export const checkPluginMarketUpdates = (): Promise<ApiResponse<PluginMarketUpdateRecord[]>> => {
  return request.get("/plugins/market/updates");
};

export const installMarketPlugin = (
  data: PluginMarketInstallRequest,
): Promise<ApiResponse<PluginRecord>> => {
  return request.post("/plugins/market/install", data);
};

export const previewMarketPlugin = (
  data: PluginMarketInstallRequest,
): Promise<ApiResponse<PluginImpactPreview>> => {
  return request.post("/plugins/market/preview", data);
};

export const installPlugin = (
  data: PluginInstallRequest,
): Promise<ApiResponse<PluginRecord>> => {
  return request.post("/plugins/install", data);
};

export const previewInstallPlugin = (
  data: PluginInstallRequest,
): Promise<ApiResponse<PluginImpactPreview>> => {
  return request.post("/plugins/preview", data);
};

export const uploadInstallPlugin = (
  data: FormData,
  options: UploadRequestOptions = {},
): Promise<ApiResponse<PluginRecord>> => {
  return uploadMultipart("/plugins/upload", data, options);
};

export const previewUploadInstallPlugin = (
  data: FormData,
  options: UploadRequestOptions = {},
): Promise<ApiResponse<PluginImpactPreview>> => {
  return uploadMultipart("/plugins/preview-upload", data, options);
};

export const updatePlugin = (
  id: string,
  data: PluginUpdateRequest,
): Promise<ApiResponse<PluginRecord>> => {
  return request.post(`/plugins/${encodeURIComponent(id)}/update`, data);
};

export const previewUpdatePlugin = (
  id: string,
  data: PluginUpdateRequest,
): Promise<ApiResponse<PluginImpactPreview>> => {
  return request.post(`/plugins/${encodeURIComponent(id)}/preview-update`, data);
};

export const uploadUpdatePlugin = (
  id: string,
  data: FormData,
  options: UploadRequestOptions = {},
): Promise<ApiResponse<PluginRecord>> => {
  return uploadMultipart(
    `/plugins/${encodeURIComponent(id)}/upload-update`,
    data,
    options,
  );
};

export const previewUploadUpdatePlugin = (
  id: string,
  data: FormData,
  options: UploadRequestOptions = {},
): Promise<ApiResponse<PluginImpactPreview>> => {
  return uploadMultipart(
    `/plugins/${encodeURIComponent(id)}/preview-upload-update`,
    data,
    options,
  );
};

export const listPluginVersions = (
  id: string,
): Promise<ApiResponse<PluginVersionRecord[]>> => {
  return request.get(`/plugins/${encodeURIComponent(id)}/versions`);
};

export const rollbackPlugin = (
  id: string,
  versionId: string,
): Promise<ApiResponse<PluginRecord>> => {
  return request.post(
    `/plugins/${encodeURIComponent(id)}/rollback/${encodeURIComponent(versionId)}`,
  );
};

export const enablePlugin = (
  id: string,
): Promise<ApiResponse<PluginRecord>> => {
  return request.post(`/plugins/${encodeURIComponent(id)}/enable`);
};

export const disablePlugin = (
  id: string,
): Promise<ApiResponse<PluginRecord>> => {
  return request.post(`/plugins/${encodeURIComponent(id)}/disable`);
};

export const uninstallPlugin = (id: string): Promise<ApiResponse<void>> => {
  return request.delete(`/plugins/${encodeURIComponent(id)}`);
};

export const resolvePluginAssetUrl = (url: string) => {
  if (/^https?:\/\//i.test(url)) return url;

  const base = localStorage.getItem(serverUrlKey)?.replace(/\/$/, "") ?? "";
  if (url.startsWith("/")) return `${base}${url}`;
  return `${base}/api/v1/${url.replace(/^\/+/, "")}`;
};
