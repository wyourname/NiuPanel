import axios, { type InternalAxiosRequestConfig, type AxiosResponse } from 'axios'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useAppStore } from '@/stores/app'
import { storageKey as makeStorageKey } from '@/utils/storage'

declare module 'axios' {
  export interface AxiosRequestConfig {
    skipMessage?: boolean
  }

  export interface InternalAxiosRequestConfig {
    skipMessage?: boolean
  }
}

const serverUrlKey = makeStorageKey('server_url')
const userStorageKey = makeStorageKey('user_info')

const getServerUrl = () => {
  const storedUrl = localStorage.getItem(serverUrlKey)
  if (storedUrl) {
    return storedUrl.replace(/\/$/, '') + '/api/v1'
  }
  return '/api/v1'
}

const request = axios.create({
  baseURL: getServerUrl(),
  timeout: 30000,
  // 允许跨域携带 Cookie
  withCredentials: true
})

// 或者提供一个辅助函数来更新
export const updateBaseUrl = (url?: string) => {
  const targetUrl = url || localStorage.getItem(serverUrlKey) || ''
  request.defaults.baseURL = targetUrl ? targetUrl.replace(/\/$/, '') + '/api/v1' : '/api/v1'
}

request.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    // 动态从 appStore 获取最新的服务器地址
    try {
      const appStore = useAppStore()
      const baseUrl = appStore.serverUrl ? appStore.serverUrl.replace(/\/$/, '') + '/api/v1' : '/api/v1'
      config.baseURL = baseUrl
    } catch (e) {
      // Pinia might not be initialized yet in some contexts
      if (!config.baseURL || config.baseURL === '/api/v1') {
        config.baseURL = getServerUrl()
      }
    }
    return config
  },
  error => {
    return Promise.reject(error)
  }
)

request.interceptors.response.use(
  (response: AxiosResponse) => {
    if (response.config.responseType === 'blob' || response.data instanceof Blob) {
      return response.data
    }
    return response.data
  },
  error => {
    const status = error.response?.status
    const bizCode = error.response?.data?.code
    const message = error.response?.data?.message || error.message || '请求失败'

    if (status === 401) {
      if (window.location.pathname !== '/login') {
        localStorage.removeItem(userStorageKey)
        ElMessage.error('登录已过期，请重新登录')
        // 使用原生跳转替代动态导入路由，消除构建警告并彻底打破循环依赖
        window.location.replace('/login')
      }
    } else if (bizCode === 5017) {
      ElMessageBox.alert(
        `<div>${message}</div><div class="mt-4 p-3 bg-base/50 rounded-xl border border-base"><p class="text-xs text-secondary mb-2">您可以前往以下地址免费部署自己的中转站：</p><a href="https://github.com/wyourname/cf-r2-transit-station" target="_blank" class="text-primary font-bold break-all hover:underline">https://github.com/wyourname/cf-r2-transit-station</a></div>`,
        '中转站未配置',
        {
          dangerouslyUseHTMLString: true,
          confirmButtonText: '确定',
          type: 'warning',
          customClass: 'station-error-box'
        }
      )
    } else {
      if (!error.config?.skipMessage) {
        ElMessage.error(message)
      }
    }

    return Promise.reject(error)
  }
)

export default request
