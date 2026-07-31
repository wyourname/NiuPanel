import {
  Box,
  DataAnalysis,
  Document,
  Key,
  List,
  Setting,
  Share,
} from "@element-plus/icons-vue";
import type { PaletteItem } from "./types";

export const navItems: PaletteItem[] = [
  {
    title: "系统概览",
    desc: "查看系统状态和统计",
    path: "/overview",
    icon: DataAnalysis,
    type: "nav",
  },
  {
    title: "任务列表",
    desc: "管理和运行定时任务",
    path: "/tasks",
    icon: List,
    type: "nav",
  },
  {
    title: "环境变量",
    desc: "配置全局和脚本变量",
    path: "/variables",
    icon: Key,
    type: "nav",
  },
  {
    title: "文件管理",
    desc: "浏览和编辑文件",
    path: "/files",
    icon: Document,
    type: "nav",
  },
  {
    title: "环境管理",
    desc: "Python/Node/Shell 环境配置",
    path: "/environments",
    icon: Box,
    type: "nav",
  },
  {
    title: "分享中心",
    desc: "查看分享的脚本",
    path: "/share",
    icon: Share,
    type: "nav",
  },
  {
    title: "系统设置",
    desc: "系统偏好设置",
    path: "/settings",
    icon: Setting,
    type: "nav",
  },
];

export const commandItems: PaletteItem[] = [
  {
    title: "刷新数据",
    desc: "重新加载当前页面数据",
    action: "refresh",
    type: "command",
  },
  {
    title: "切换主题",
    desc: "在深色/浅色模式间切换",
    action: "toggle_theme",
    type: "command",
  },
  {
    title: "退出登录",
    desc: "注销当前账户",
    action: "logout",
    type: "command",
  },
];
