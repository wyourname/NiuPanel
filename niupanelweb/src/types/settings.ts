export interface SettingItem {
  key: string;
  value: string;
  category: string;
}

export interface GeneralSettings {
  name: string;
  logo: string;
  timezone: string;
  max_concurrency: number;
  log_retention_days: number;
  github_proxy_url: string;
  uv_python_mirror: string;
  uv_pypi_mirror: string;
  default_python_version: string;
  default_node_version: string;
  pnpm_node_dist_mirror: string;
  npm_registry_mirror: string;
}

export interface NotificationSettings {
  webhook_url: string;
  events: string;
  mail_host: string;
  mail_username: string;
  mail_password: string;
  mail_to: string;
}

export type NotificationTestType = "mail" | "webhook";

export interface NotificationTestRequest {
  notify_type: NotificationTestType;
  "notify.webhook_url": string;
  "mail.host": string;
  "mail.username": string;
  "mail.password": string;
  "mail.to": string;
}

export interface BackupOptions {
  tasks: boolean;
  variables: boolean;
  settings: boolean;
  environments: boolean;
  telegram: boolean;
}

export type MaintenanceTaskStatus =
  | "pending"
  | "processing"
  | "completed"
  | "error";

export interface MaintenanceStatus {
  progress: number;
  message: string;
  status: MaintenanceTaskStatus | string;
  filename: string | null;
}

export interface LogCleanupReport {
  dry_run: boolean;
  cutoff_at: string;
  files: number;
  task_runs: number;
  system_jobs: number;
  audit_logs: number;
  bytes: number;
  empty_directories: number;
  protected_files: number;
  warnings: string[];
}

export interface UpdateInfo {
  update_available: boolean;
  tag_name: string;
  body: string;
  channel?: UpdateChannel;
  prerelease?: boolean;
  size: number;
  [key: string]: unknown;
}

export type UpdateChannel = "stable" | "preview";

export interface UpdateStatus {
  state: string;
  message?: string;
  progress: number;
  error?: string;
}

export interface SecuritySettings {
  max_sessions: number;
}

export interface ProfileUpdateRequest {
  old_password?: string;
  new_username?: string;
  new_email?: string;
  new_password?: string;
}

export interface SessionInfo {
  id: string;
  expiry: number;
  is_current: boolean;
  ip_address?: string;
}
