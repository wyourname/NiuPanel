export type JobStatus =
  | "Pending"
  | "Running"
  | "Success"
  | "Failed"
  | "Cancelled"
  | "Finished";

export interface Job {
  id: number;
  name: string;
  status: JobStatus | string;
  created_at?: string;
  updated_at?: string;
  [key: string]: unknown;
}
