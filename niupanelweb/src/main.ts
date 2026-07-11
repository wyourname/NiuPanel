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

import 'monaco-editor/esm/vs/basic-languages/javascript/javascript.contribution'
import 'monaco-editor/esm/vs/basic-languages/typescript/typescript.contribution'
import 'monaco-editor/esm/vs/basic-languages/python/python.contribution'
import 'monaco-editor/esm/vs/basic-languages/shell/shell.contribution'
import 'monaco-editor/esm/vs/basic-languages/html/html.contribution'
import 'monaco-editor/esm/vs/basic-languages/css/css.contribution'
import 'monaco-editor/esm/vs/basic-languages/yaml/yaml.contribution'
import 'monaco-editor/esm/vs/basic-languages/sql/sql.contribution'
import 'monaco-editor/esm/vs/basic-languages/xml/xml.contribution'
import 'monaco-editor/esm/vs/basic-languages/markdown/markdown.contribution'
import 'monaco-editor/esm/vs/basic-languages/ini/ini.contribution'
import 'monaco-editor/esm/vs/basic-languages/dockerfile/dockerfile.contribution'
import 'monaco-editor/esm/vs/basic-languages/rust/rust.contribution'
import 'monaco-editor/esm/vs/basic-languages/go/go.contribution'

// 使用本地 npm 包，不依赖 CDN（支持国内网络）
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
