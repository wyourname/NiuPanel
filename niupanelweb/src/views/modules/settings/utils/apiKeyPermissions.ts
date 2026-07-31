export type ApiPermissionGroupId =
  | "task"
  | "var"
  | "file"
  | "env"
  | "job"
  | "sys"
  | "compiler"
  | "webhook"
  | "mcp";

export type ApiPermissionNavGroupId = "all" | ApiPermissionGroupId;

export type ApiPermission = {
  label: string;
  value: string;
};

export type ApiPermissionGroup = {
  title: string;
  icon: string;
  color: string;
  perms: ApiPermission[];
};

export type ApiKeyFormState = {
  name: string;
  expires_in_days: number;
  expires_at: string;
};

export const navGroups: Array<{
  id: ApiPermissionNavGroupId;
  label: string;
  icon: string;
}> = [
  { id: "all", label: "最高权限", icon: "i-ep-coordinate" },
  { id: "task", label: "任务管理", icon: "i-ep-list" },
  { id: "var", label: "环境变量", icon: "i-ep-key" },
  { id: "file", label: "文件中心", icon: "i-ep-document" },
  { id: "env", label: "运行环境", icon: "i-ep-cpu" },
  { id: "job", label: "作业记录", icon: "i-ep-monitor" },
  { id: "sys", label: "分享审计", icon: "i-ep-share" },
  { id: "compiler", label: "代码加密", icon: "i-ep-cpu" },
  { id: "webhook", label: "Webhook", icon: "i-ep-notification" },
  { id: "mcp", label: "MCP 工具预设", icon: "i-carbon-network-4" },
];

export const permissionGroups: Record<
  ApiPermissionGroupId,
  ApiPermissionGroup
> = {
  task: {
    title: "任务管理 (Task)",
    icon: "i-ep-list",
    color: "bg-blue-500",
    perms: [
      { label: "查看任务列表", value: "task:list" },
      { label: "读取任务详情", value: "task:read" },
      { label: "新建自动化任务", value: "task:create" },
      { label: "修改任务配置", value: "task:update" },
      { label: "物理删除任务", value: "task:delete" },
      { label: "执行/启动任务", value: "task:run" },
      { label: "强制中止运行", value: "task:stop" },
      { label: "管理所有任务", value: "task:*" },
    ],
  },
  var: {
    title: "环境变量 (Variable)",
    icon: "i-ep-key",
    color: "bg-amber-500",
    perms: [
      { label: "查看变量列表", value: "var:list" },
      { label: "读取变量明文", value: "var:read" },
      { label: "新增环境变量", value: "var:create" },
      { label: "修改变量/状态", value: "var:update" },
      { label: "物理删除变量", value: "var:delete" },
      { label: "管理所有变量", value: "var:*" },
    ],
  },
  file: {
    title: "文件中心 (File)",
    icon: "i-ep-document",
    color: "bg-indigo-500",
    perms: [
      { label: "浏览目录结构", value: "file:list" },
      { label: "读取文件内容", value: "file:read" },
      { label: "上传/写入文件", value: "file:write" },
      { label: "删除物理文件", value: "file:delete" },
      { label: "管理所有文件", value: "file:*" },
    ],
  },
  env: {
    title: "运行环境 (Env)",
    icon: "i-ep-cpu",
    color: "bg-emerald-500",
    perms: [
      { label: "查看环境列表", value: "env:list" },
      { label: "查询依赖版本", value: "env:read" },
      { label: "创建虚拟环境", value: "env:create" },
      { label: "安装/管理依赖", value: "env:update" },
      { label: "卸载运行环境", value: "env:delete" },
      { label: "环境完全控制", value: "env:*" },
    ],
  },
  job: {
    title: "作业记录 (Job)",
    icon: "i-ep-monitor",
    color: "bg-sky-500",
    perms: [
      { label: "查看作业历史", value: "job:list" },
      { label: "读取执行日志", value: "job:read" },
      { label: "删除作业记录", value: "job:delete" },
      { label: "作业全权管理", value: "job:*" },
    ],
  },
  sys: {
    title: "分享与审计 (System)",
    icon: "i-ep-share",
    color: "bg-rose-500",
    perms: [
      { label: "查看分享链接", value: "share:list" },
      { label: "系统概览", value: "overview:read" },
      { label: "查看审计日志", value: "audit:list" },
      { label: "审计全权", value: "audit:*" },
    ],
  },
  compiler: {
    title: "代码加密 (Compiler)",
    icon: "i-ep-cpu",
    color: "bg-orange-600",
    perms: [
      { label: "读取编译器版本", value: "compiler:read" },
      { label: "执行代码加密", value: "compiler:run" },
    ],
  },
  webhook: {
    title: "Webhook 推送 (Webhook)",
    icon: "i-ep-notification",
    color: "bg-purple-500",
    perms: [
      { label: "推送系统通知", value: "webhook:push" },
      { label: "管理所有推送", value: "webhook:*" },
    ],
  },
  mcp: {
    title: "MCP 工具权限预设",
    icon: "i-carbon-network-4",
    color: "bg-emerald-600",
    perms: [
      { label: "系统状态与版本", value: "overview:read" },
      { label: "全部任务工具", value: "task:*" },
      { label: "全部变量工具", value: "var:*" },
      { label: "全部文件工具", value: "file:*" },
      { label: "全部运行环境工具", value: "env:*" },
      { label: "全部作业工具", value: "job:*" },
      { label: "读取审计记录", value: "audit:list" },
      { label: "推送系统通知", value: "webhook:push" },
      { label: "读取分享数据", value: "share:list" },
      { label: "读取 Git 仓库", value: "git:read" },
      { label: "同步 Git 仓库", value: "git:sync" },
    ],
  },
};

export const getPermissionGroup = (id: ApiPermissionNavGroupId) =>
  id === "all" ? undefined : permissionGroups[id];

export const parsePerms = (permissions?: string) =>
  permissions
    ? permissions
        .split(",")
        .map((permission) => permission.trim())
        .filter(
          (permission) =>
            Boolean(permission) &&
            permission !== "mcp:connect" &&
            permission !== "mcp:*",
        )
    : [];

export const isExpired = (timestamp?: number) =>
  timestamp ? timestamp < Date.now() / 1000 : false;

export const getPermColor = (permission: string) => {
  if (permission.includes("*")) return "danger";
  if (
    ["write", "create", "update", "delete", "run", "stop"].some((keyword) =>
      permission.includes(keyword),
    )
  ) {
    return "warning";
  }
  return "primary";
};
