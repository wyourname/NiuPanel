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

export interface OverviewSystemInfo {
  cpu_usage: number;
  memory_total: number;
  memory_used: number;
  disk_total: number;
  disk_used: number;
  uptime: number;
  os_info: string;
  public_ip: string | null;
}

export interface OverviewData {
  task_stats: OverviewTaskStats;
  cpu_usage: number;
  memory_total: number;
  memory_used: number;
  disk_total: number;
  disk_used: number;
  uptime: number;
  os_info: string;
  public_ip: string | null;
  recent_activity: OverviewActivityItem[];
  chart_data: OverviewChartData;
}
