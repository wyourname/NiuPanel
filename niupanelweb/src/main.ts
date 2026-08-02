import { createApp } from 'vue'
import { createPinia } from 'pinia'
import 'element-plus/dist/index.css'
import 'element-plus/theme-chalk/dark/css-vars.css'
import './assets/styles/index.css'
import 'virtual:uno.css'

import App from './App.vue'
import router from './router'
import i18n from './locales' // Import i18n
import { hasPermission } from './utils/permission'

const app = createApp(App)

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

app.mount('#app')
