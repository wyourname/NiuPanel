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
    component: Login
  },
  {
    path: '/onboarding',
    name: 'onboarding',
    component: Onboarding
  },
  {
    path: '/',
    component: MainLayout,
    redirect: '/tasks',
    children: [
      { path: 'overview', name: 'overview', component: Overview },
      { path: 'tasks', name: 'tasks', component: Tasks },
      { path: 'variables', name: 'variables', component: Variables },
      { path: 'files', name: 'files', component: File },
      { path: 'environments', name: 'environments', component: Environment },
      { path: 'share', name: 'share', component: Share },
      { path: 'extensions', name: 'extensions', component: Extensions },
      { path: 'settings', name: 'settings', component: Settings },
      { path: 'terminal', name: 'terminal', component: Terminal },
      { path: 'git', name: 'git', component: Git },
      { path: 'more', name: 'more', component: More },
      { path: 'webhook', name: 'webhook', component: Webhook },
      { path: 'telegram', name: 'telegram', component: Telegram },
      {
        path: 'plugins/agents/:pathMatch(.*)*',
        name: 'plugin-agents',
        component: AgentsGateway
      },
      {
        path: 'plugins/:pluginId/:pathMatch(.*)*',
        name: 'plugin-app',
        component: PluginHostView
      }
    ]
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

router.beforeEach(async (to, from, next) => {
  const userStore = useUserStore()
  const isAuthenticated = !!(userStore.userInfo && userStore.userInfo.username)

  // 1. If visiting login page
  if (to.name === 'login') {
    if (isAuthenticated) {
      next({ name: 'tasks' })
    } else {
      next()
    }
    return
  }

  // 2. Allow onboarding page if authenticated
  if (to.name === 'onboarding') {
    if (!isAuthenticated) {
      next({ name: 'login' })
    } else {
      next()
    }
    return
  }

  // 3. If visiting protected routes
  if (!isAuthenticated) {
    next({ name: 'login' })
    return
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
        next({ name: 'onboarding' })
        return
      }
    } catch (_) {
      // On error, don't block the user
    }
  }

  next()
})

export default router
