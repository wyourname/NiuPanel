import type { McpToolInfo } from "@/types";

export type McpAccessSummary = {
  accessible: number;
  destructive: number;
  total: number;
};

/**
 * Keep this in sync with `AuthenticatedUser::has_permission` on the server:
 * exact scope, global wildcard, and resource wildcard all grant MCP access.
 */
export const permissionGrants = (granted: string, required: string) => {
  if (granted === "*:*" || granted === required) return true;
  const [resource] = required.split(":", 1);
  return granted === `${resource}:*`;
};

export const canAccessMcpTool = (
  permissions: string[],
  tool: McpToolInfo,
) => permissions.some((permission) => permissionGrants(permission, tool.permission));

export const getMcpAccessSummary = (
  permissions: string[],
  tools: McpToolInfo[],
): McpAccessSummary => {
  const accessibleTools = tools.filter((tool) => canAccessMcpTool(permissions, tool));
  return {
    accessible: accessibleTools.length,
    destructive: accessibleTools.filter((tool) => tool.destructive).length,
    total: tools.length,
  };
};

export const getMcpToolCountForPermission = (
  permission: string,
  tools: McpToolInfo[],
) => tools.filter((tool) => permissionGrants(permission, tool.permission)).length;
