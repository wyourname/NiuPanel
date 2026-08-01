import type { AxiosProgressEvent, AxiosRequestConfig } from "axios";
import request from "@/utils/request";

export type UploadRequestOptions = {
  onUploadProgress?: (event: AxiosProgressEvent) => void;
  params?: AxiosRequestConfig["params"];
  signal?: AbortSignal;
  skipMessage?: boolean;
};

export type UploadFormEntry = readonly [
  name: string,
  value: string | Blob,
  fileName?: string,
];

export function createUploadFormData(
  entries: Iterable<UploadFormEntry>,
): FormData {
  const formData = new FormData();
  for (const [name, value, fileName] of entries) {
    if (typeof value === "string") {
      formData.append(name, value);
    } else if (fileName) {
      formData.append(name, value, fileName);
    } else {
      formData.append(name, value);
    }
  }
  return formData;
}

/**
 * 所有浏览器到 NiuPanel 的 multipart 上传统一走这里。
 *
 * Axios 会自动为 FormData 生成包含 boundary 的 Content-Type；不要在各业务
 * API 中手动设置该请求头。上传不继承全局 30 秒超时，取消和进度则由调用方
 * 通过同一组选项传入。
 */
export function uploadMultipart<T>(
  url: string,
  formData: FormData,
  options: UploadRequestOptions = {},
): Promise<T> {
  // request 的响应拦截器会返回 response.data，Axios 自带类型无法表达该行为。
  return request.post(url, formData, {
    onUploadProgress: options.onUploadProgress,
    params: options.params,
    signal: options.signal,
    skipMessage: options.skipMessage,
    timeout: 0,
  }) as unknown as Promise<T>;
}
