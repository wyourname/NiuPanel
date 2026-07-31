export interface ShareItem {
  id: string;
  name: string;
  expires_at: number | null;
  used_count: number;
  max_uses: number | null;
  burn_after_reading: boolean;
  [key: string]: unknown;
}

export interface FileNode {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  children?: FileNode[] | null;
  task_id?: number | null;
}

export interface TaskFileMapping {
  task_id: number;
  path: string;
  file: string;
  name: string;
}

export interface TaskFileTreeData {
  tree: FileNode[];
  tasks: TaskFileMapping[];
  suggestions: string[];
}

export interface FileAssociations {
  main_file: string;
  dependencies: string[];
}

export interface TaskExportSpec {
  task_id: number;
  files: FileAssociations[];
  include_envs: boolean;
}

export interface CreateShareRequest {
  tasks: TaskExportSpec[];
  password?: string | null;
  max_uses?: number | null;
  expires_in_hours?: number | null;
  burn_after_reading?: boolean;
  note?: string | null;
}

export interface CreateShareResponse {
  link: string;
}

export interface StationFile {
  token: string;
  file_key?: string;
  fileKey?: string;
  size: number;
  downloadsRemaining?: number;
  downloads_remaining?: number;
  deleteOnDownload?: boolean;
  delete_on_download?: boolean;
  expiresAt?: number | null;
  expires_at?: number | null;
  password?: string | null;
  note?: string | null;
  uploaded_at?: number | null;
}

export interface StationStats {
  currentUsageBytes: number;
  maxUsageBytes: number;
  usagePercent: string;
  maxFileSizeContent: number;
  isConfigured: boolean;
}

export interface StationConfigPayload {
  url: string;
  token: string;
}

export interface UpdateStationFileRequest {
  downloadsRemaining?: number;
  deleteOnDownload?: boolean;
  expiresAt?: number | null;
  password?: string;
  note?: string;
}

export interface MarketScriptItem {
  name: string;
  description?: string | null;
  version: string;
  author?: string | null;
  url: string;
  icon?: string | null;
  tags: string[];
  updated_at: number;
  is_encrypted: boolean;
}

export interface MarketScriptAggregated {
  source_id: number;
  source_name: string;
  script: MarketScriptItem;
}

export interface MarketSource {
  id: number;
  name: string;
  url: string;
  description?: string | null;
  enabled?: boolean;
  created_at?: string;
  updated_at?: string;
}

export interface CreateMarketSourceRequest {
  name: string;
  url: string;
  description?: string | null;
}

export type ImportState = "pending" | "downloading" | "ready" | "error";

export interface SubmitImportRequest {
  url: string;
  password?: string;
}

export interface SubmitImportResponse {
  staging_id: string;
}

export interface ImportStatus {
  state: ImportState;
  progress: number;
  message?: string | null;
}

export interface ConfirmImportRequest {
  selected_tasks?: string[];
  update_existing?: boolean;
}

export interface ImportFailure {
  task_name: string;
  error: string;
}

export interface ImportSummary {
  success_count: number;
  skip_count: number;
  failure_count: number;
  failures: ImportFailure[];
}

export interface ShareVariable {
  key: string;
  value: string;
}

export interface ShareTaskMeta {
  name: string;
  description?: string | null;
  env_type: string;
  env_version?: string | null;
  cron_schedule?: string | null;
  command?: string | null;
  requirements?: string | null;
  notify: boolean;
  variables?: ShareVariable[] | null;
  remote_task_id?: string | null;
}

export interface ShareFileEntry {
  path: string;
  mode: number;
  content?: unknown;
}

export interface ShareTaskData {
  meta: ShareTaskMeta;
  main_file?: string | null;
  files: ShareFileEntry[];
}

export interface NiuPackage {
  version: number;
  created_at: number;
  tasks: ShareTaskData[];
  note?: string | null;
}

export interface ImportHistoryItem {
  id: number;
  url: string;
  share_code: string | null;
  task_name: string;
  note: string | null;
  created_at: number;
  updated_at: number;
}

export interface ImportSourceGroup {
  share_code: string | null;
  url: string;
  tasks: ImportHistoryItem[];
  task_count: number;
  last_updated_at: number;
}

export interface DeleteImportedTasksParams {
  task_id?: number;
  share_code?: string;
  import_source?: string;
}

export type TransferState = "pending" | "uploading" | "success" | "error";

export interface TransferUploadRequest {
  expire_hours: number;
  password?: string | null;
  burn_after_reading?: boolean;
}

export interface TransferStatus {
  state: TransferState;
  progress: number;
  message?: string | null;
  download_url?: string | null;
}
