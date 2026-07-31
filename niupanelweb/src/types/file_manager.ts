export interface FileItem {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  mtime?: number;
}

export type FileListQueryParams = {
  q?: string;
};
