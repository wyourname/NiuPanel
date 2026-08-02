import { createRouter, createWebHistory } from 'vue-router'
import { useUserStore } from '../stores/user'
import { usePluginAppsStore } from '../stores/pluginApps'
import request from '../utils/request'
import type { ApiResponse } from '@/types'
import { workspaceAppLoaders } from '@/workspace/components'

const MainLayout = () => import('../layout/MainLayout.vue')
const Login = () => import('../views/Login.vue')
const Onboarding = () => import('../views/Onboarding.vue')

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
      { path: 'overview', name: 'overview', component: workspaceAppLoaders.overview, meta: { title: '系统概览' } },
      { path: 'tasks', name: 'tasks', component: workspaceAppLoaders.tasks, meta: { title: '任务列表' } },
      { path: 'variables', name: 'variables', component: workspaceAppLoaders.variables, meta: { title: '环境变量' } },
      { path: 'files', name: 'files', component: workspaceAppLoaders.files, meta: { title: '文件管理' } },
      { path: 'environments', name: 'environments', component: workspaceAppLoaders.environments, meta: { title: '环境管理' } },
      { path: 'share', name: 'share', component: workspaceAppLoaders.share, meta: { title: '分享中心' } },
      { path: 'extensions', name: 'extensions', component: workspaceAppLoaders.extensions, meta: { title: '扩展中心' } },
      { path: 'settings', name: 'settings', component: workspaceAppLoaders.settings, meta: { title: '系统设置' } },
      { path: 'terminal', name: 'terminal', component: workspaceAppLoaders.terminal, meta: { title: '系统终端' } },
      { path: 'git', name: 'git', component: workspaceAppLoaders.git, meta: { title: 'Git 管理' } },
      { path: 'more', name: 'more', component: workspaceAppLoaders.more, meta: { title: '更多功能' } },
      { path: 'webhook', name: 'webhook', component: () => import('../views/modules/webhook/index.vue'), meta: { title: 'Webhook' } },
      { path: 'telegram', name: 'telegram', component: workspaceAppLoaders.telegram, meta: { title: '电报机器人' } },
      {
        path: 'plugins/agents/:pathMatch(.*)*',
        name: 'plugin-agents',
        component: () => import('../views/modules/AgentsGateway.vue'),
        meta: { title: '智能代理' }
      },
      {
        path: 'plugins/:pluginId/:pathMatch(.*)*',
        name: 'plugin-app',
        component: () => import('../views/plugins/PluginHostView.vue'),
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
