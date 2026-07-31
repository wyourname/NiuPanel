export interface McpToolInfo {
  name: string;
  category: string;
  description: string;
  permission: string;
  destructive: boolean;
}

export interface McpInfo {
  enabled: boolean;
  endpoint: string;
  transport: "streamable_http";
  auth_header: "X-API-Key";
  required_permission: string | null;
  tools: McpToolInfo[];
}
