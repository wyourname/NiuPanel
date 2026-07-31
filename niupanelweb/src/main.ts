import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import zhCn from 'element-plus/dist/locale/zh-cn.mjs'
import 'element-plus/dist/index.css'
import 'element-plus/theme-chalk/dark/css-vars.css'
import './assets/styles/index.css'
import 'virtual:uno.css'

import App from './App.vue'
import router from './router'
import i18n from './locales' // Import i18n
import { hasPermission } from './utils/permission'
import PullToRefresh from './components/common/PullToRefresh.vue'
import { loader } from "@guolao/vue-monaco-editor"
import * as monaco from 'monaco-editor'

import 'monaco-editor/basic-languages/monaco.contribution.js'

// 使用本地 pnpm 依赖，不依赖 CDN（支持国内网络）
// 由 vite-plugin-monaco-editor 处理 worker 注入
loader.config({ monaco })

const app = createApp(App)

// Register icons
for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
  app.component(key, component)
}

// Register global components
app.component('PullToRefresh', PullToRefresh)

// Register v-permission directive
app.directive('permission', {
  mounted(el, binding) {
    const { value } = binding
    if (value && typeof value === 'string') {
      if (!hasPermission(value)) {
        el.parentNode?.removeChild(el)
      }
    }
  }
})

app.use(createPinia())
app.use(router)
app.use(i18n) // Install i18n
app.use(ElementPlus, {
  locale: zhCn,
})

app.mount('#app')
