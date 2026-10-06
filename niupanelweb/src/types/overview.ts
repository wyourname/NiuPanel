import type { TaskStatus } from "./tasks";

export interface OverviewTaskStats {
  total: number;
  running: number;
  failed_today: number;
  next_run: number | null;
}

export interface OverviewActivityItem {
  id: number;
  task_name: string;
  status: TaskStatus;
  time: number;
  duration: string | null;
}

export interface OverviewChartData {
  hours: string[];
  success: number[];
  failed: number[];
}

export interface SystemMetrics {
  cpu_usage: number;
  memory_total: number;
  memory_used: number;
  network_upload_speed?: number | null;
  network_download_speed?: number | null;
  uptime: number;
  os_info: string;
}

export interface OverviewSystemInfo extends SystemMetrics {
  disk_total: number;
  disk_used: number;
  public_ip: string | null;
}

export interface OverviewData extends OverviewSystemInfo {
  task_stats: OverviewTaskStats;
  recent_activity: OverviewActivityItem[];
  chart_data: OverviewChartData;
}
