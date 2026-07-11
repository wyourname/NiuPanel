import request from "@/utils/request";
import type { ApiResponse, McpInfo } from "@/types";

export const getMcpInfo = (): Promise<ApiResponse<McpInfo>> =>
  request.get("/mcp/info");
