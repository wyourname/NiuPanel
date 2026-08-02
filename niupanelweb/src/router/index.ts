import { createRouter, createWebHistory } from 'vue-router'
import { useUserStore } from '../stores/user'
import { usePluginAppsStore } from '../stores/pluginApps'
import MainLayout from '../layout/MainLayout.vue'
import Login from '../views/Login.vue'
import Onboarding from '../views/Onboarding.vue'
import request from '../utils/request'

// Modules
import Overview from '../views/modules/overview/index.vue'
import Tasks from '../views/modules/Tasks.vue'
import Variables from '../views/modules/Variables.vue'
import File from '../views/modules/File.vue'
import Environment from '../views/modules/Environment.vue'
import Share from '../views/modules/share/index.vue'
import Extensions from '../views/modules/extensions/index.vue'
import AgentsGateway from '../views/modules/AgentsGateway.vue'
import Settings from '../views/modules/settings/index.vue'
import Terminal from '../views/modules/terminal/index.vue'
import Git from '../views/modules/git/index.vue'
import More from '../views/modules/more/index.vue'
import Webhook from '../views/modules/webhook/index.vue'
import Telegram from '../views/modules/telegram/index.vue'
import PluginHostView from '../views/plugins/PluginHostView.vue'
import type { ApiResponse } from '@/types'

const routes = [
  {
    path: '/login',
    name: 'login',
    component: Login,
    meta: { title: '登录' }
  },
  {
    path: '/onboarding',
    name: 'onboarding',
    component: Onboarding,
    meta: { title: '首次配置' }
  },
  {
    path: '/',
    component: MainLayout,
    redirect: '/tasks',
    children: [
      { path: 'overview', name: 'overview', component: Overview, meta: { title: '系统概览' } },
      { path: 'tasks', name: 'tasks', component: Tasks, meta: { title: '任务列表' } },
      { path: 'variables', name: 'variables', component: Variables, meta: { title: '环境变量' } },
      { path: 'files', name: 'files', component: File, meta: { title: '文件管理' } },
      { path: 'environments', name: 'environments', component: Environment, meta: { title: '环境管理' } },
      { path: 'share', name: 'share', component: Share, meta: { title: '分享中心' } },
      { path: 'extensions', name: 'extensions', component: Extensions, meta: { title: '扩展中心' } },
      { path: 'settings', name: 'settings', component: Settings, meta: { title: '系统设置' } },
      { path: 'terminal', name: 'terminal', component: Terminal, meta: { title: '系统终端' } },
      { path: 'git', name: 'git', component: Git, meta: { title: 'Git 管理' } },
      { path: 'more', name: 'more', component: More, meta: { title: '更多功能' } },
      { path: 'webhook', name: 'webhook', component: Webhook, meta: { title: 'Webhook' } },
      { path: 'telegram', name: 'telegram', component: Telegram, meta: { title: '电报机器人' } },
      {
        path: 'plugins/agents/:pathMatch(.*)*',
        name: 'plugin-agents',
        component: AgentsGateway,
        meta: { title: '智能代理' }
      },
      {
        path: 'plugins/:pluginId/:pathMatch(.*)*',
        name: 'plugin-app',
        component: PluginHostView,
        meta: { title: '插件应用' }
      }
    ]
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

router.beforeEach(async (to, from) => {
  const userStore = useUserStore()
  const isAuthenticated = !!(userStore.userInfo && userStore.userInfo.username)

  // 1. If visiting login page
  if (to.name === 'login') {
    return isAuthenticated ? { name: 'tasks' } : true
  }

  // 2. Allow onboarding page if authenticated
  if (to.name === 'onboarding') {
    return isAuthenticated ? true : { name: 'login' }
  }

  // 3. If visiting protected routes
  if (!isAuthenticated) {
    return { name: 'login' }
  }

  try {
    await usePluginAppsStore().loadApps()
  } catch (_) {
    // Plugin apps are optional; navigation should not be blocked by a plugin registry error.
  }

  // 4. Check onboarding status (only once, first navigation after login)
  if (from.name === 'login' || from.name === 'onboarding') {
    try {
      const res = await request.get<ApiResponse<boolean>, ApiResponse<boolean>>('/settings/onboarding')
      const isDone = res.data === true
      if (!isDone) {
        return { name: 'onboarding' }
      }
    } catch (_) {
      // On error, don't block the user
    }
  }

  return true
})

export default router
