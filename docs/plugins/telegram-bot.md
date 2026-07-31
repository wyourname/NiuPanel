# Telegram Bot 插件化计划

## 目标

将当前内置于 `niupanel-bot`、Core Telegram API 和 Web Telegram 页面中的能力迁移为可独立安装、更新、停用和回滚的 `telegram-bot` 插件。Core 只提供通用插件生命周期、权限、事件和宿主工具，不继续承载 Telegram 专用业务逻辑。

## 当前状态

- `niupanel-bot/` 实现 Telegram Long Polling、任务交互、文件接收和通知。
- Core 负责启动 Bot，并提供 `/api/v1/bot/telegram/*` 配置、命令和工作流接口。
- Web UI 包含固定 Telegram 页面。

这些代码在插件迁移完成前继续工作，但属于过渡实现。

## 目标插件结构

```text
telegram-bot/
  plugin.json
  backend/telegram-bot
  ui/dist/niupanel-plugin.js
```

建议 manifest：

```json
{
  "schema_version": 1,
  "id": "telegram-bot",
  "name": "Telegram Bot",
  "version": "1.0.0",
  "description": "Telegram notifications and remote task operations for NiuPanel.",
  "license": "Apache-2.0",
  "runtime": "process",
  "protocol": "json_lines",
  "entry": "backend/telegram-bot",
  "runtime_permissions": ["network_outbound"],
  "capabilities": ["telegram.bot", "telegram.notifications"],
  "ui": {
    "enabled": true,
    "mode": "vue_app",
    "entry": "ui/dist/niupanel-plugin.js",
    "sdk_version": "^0.1.0",
    "display": {
      "sidebar": true,
      "workspace": true,
      "mobile": true,
      "category": "integrations",
      "order": 40,
      "layout": "panel"
    },
    "routes": [
      {
        "id": "home",
        "path": "/plugins/telegram-bot",
        "title": "Telegram",
        "icon": "i-carbon-logo-telegram"
      }
    ],
    "permissions": [],
    "api": { "allow": [] }
  }
}
```

## 宿主能力前置条件

当前 `json_lines` 进程只在调用时启动并处理请求，尚不足以承载持续 Long Polling。迁移前需要补齐通用插件能力：

1. **服务生命周期**：插件启用时启动，停用、更新和回滚时停止，异常退出可按策略重启。
2. **事件订阅**：允许插件按 manifest 声明订阅任务运行、失败、登录告警和系统状态事件。
3. **宿主工具**：通过权限受控工具读取任务、运行记录和日志，以及执行任务、停止任务和导入文件。
4. **Secret 配置**：Bot Token 等敏感配置由宿主加密保存，不写入 `plugin.json`、普通环境变量或插件日志。
5. **入站通道**：默认使用 `network_outbound` Long Polling；Webhook 模式由 Core 提供受控转发入口，插件不能自行监听宿主端口。

这些能力必须是通用插件机制，不能在 Core 中重新加入 Telegram 特例。

## 功能范围

第一阶段保留当前已有能力：

- Bot 连接、代理、自定义 Telegram API 地址和延迟测试；
- 任务搜索、运行、停止、启停调度和最近日志；
- 任务失败、系统上线和安全事件通知；
- 自定义命令与工作流；
- 脚本文件和分享包接收；
- Telegram 二次确认及操作审计。

后续能力包括代码片段创建任务、日志 Tail/Grep、通知静默规则和多 Chat 角色。

## 数据与安全边界

- 插件持久化数据只能写入 `NIUPANEL_PLUGIN_DATA_DIR`。
- 插件不得读取或迁移面板 SQLite 数据库。
- 所有面板业务操作必须经过宿主工具或显式允许的 API，并复用当前用户/插件权限检查。
- Telegram Chat ID 需要映射为插件角色，危险操作必须二次确认。
- Bot Token、代理凭据、文件内容和变量值不得写入审计详情。
- 网络权限仅用于 Telegram API；如果未来支持域名级网络规则，应限制到配置的 Telegram API 主机。

## 迁移阶段

1. 为插件宿主增加服务生命周期、事件订阅和 Secret 配置。
2. 将 `niupanel-bot` 逻辑抽取为独立插件后端，先保持现有协议和行为。
3. 将固定 Telegram Web 页面迁入插件原生 Vue UI。
4. 提供一次性迁移，将 `plugin.telegram.config`、命令和工作流导入插件数据目录。
5. 在兼容版本中保留旧入口跳转和迁移提示。
6. 删除 Core Telegram 专用路由、后台启动逻辑以及 `niupanel-bot` workspace crate。

迁移完成的判定标准是：未安装 Telegram 插件时，Core 和 Web 不包含 Telegram 专用后台任务、API 或固定页面，安装插件后可以恢复完整功能。
