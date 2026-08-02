import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { useDark, useToggle, useWindowSize } from '@vueuse/core'
import { updateBaseUrl } from '@/utils/request'
import { storageKey as makeStorageKey } from '@/utils/storage'
import { MOBILE_MAX_WIDTH } from '@/constants/responsive'

const serverUrlKey = makeStorageKey('server_url')

export const useAppStore = defineStore('app', () => {
  // --- Theme (Dark Mode) ---
  const isDark = useDark()
  const toggleDark = useToggle(isDark)

  // --- Layout ---
  const sidebarCollapse = ref(false)
  const toggleSidebar = () => {
    sidebarCollapse.value = !sidebarCollapse.value
  }

  // --- Server Config ---
  const serverUrl = ref(localStorage.getItem(serverUrlKey) || '')
  const setServerUrl = (url: string) => {
    serverUrl.value = url
    if (url) {
      localStorage.setItem(serverUrlKey, url)
    } else {
      localStorage.removeItem(serverUrlKey)
    }
  }

  // Sync axios baseURL when serverUrl changes
  watch(serverUrl, (newUrl) => {
    updateBaseUrl(newUrl)
  }, { immediate: true })

  // --- Mobile Detection ---
  const { width } = useWindowSize()
  const isMobile = ref(width.value <= MOBILE_MAX_WIDTH)
  const showDrawerSidebar = ref(false)

  const toggleDrawerSidebar = () => {
    showDrawerSidebar.value = !showDrawerSidebar.value
  }

  watch(width, (newWidth) => {
    if (newWidth === 0) return
    const mobile = newWidth <= MOBILE_MAX_WIDTH
    isMobile.value = mobile
    // Auto-collapse sidebar on mobile
    if (mobile) {
      sidebarCollapse.value = true
    }
  }, { immediate: true })

  // --- Back Button Handling ---
  type BackActionOutcome =
    | boolean
    | {
        handled: boolean
        remove?: boolean
      }
  type BackActionResult = BackActionOutcome | Promise<BackActionOutcome>
  type BackAction = () => BackActionResult
  const backActionStack = ref<BackAction[]>([])

  const pushBackAction = (fn: BackAction) => {
    backActionStack.value.push(fn)
    return () => {
      const index = backActionStack.value.lastIndexOf(fn)
      if (index !== -1) {
        backActionStack.value.splice(index, 1)
      }
    }
  }

  const popBackAction = () => {
    backActionStack.value.pop()
  }

  const removeBackAction = (fn: BackAction) => {
    const index = backActionStack.value.lastIndexOf(fn)
    if (index !== -1) {
      backActionStack.value.splice(index, 1)
      return true
    }
    return false
  }

  const normalizeBackActionOutcome = (outcome: BackActionOutcome) => {
    if (typeof outcome === 'boolean') {
      return {
        handled: outcome,
        remove: outcome,
      }
    }

    return {
      handled: outcome.handled,
      remove: outcome.remove ?? outcome.handled,
    }
  }

  const handleBack = async () => {
    while (backActionStack.value.length > 0) {
      // 执行最近注册的返回操作
      const lastAction = backActionStack.value[backActionStack.value.length - 1]
      const result = normalizeBackActionOutcome(await lastAction())

      if (result.remove) {
        removeBackAction(lastAction)
      }

      if (result.handled) {
        return true
      }

      if (!result.remove) break
    }

    if (showDrawerSidebar.value) {
      showDrawerSidebar.value = false
      return true
    }

    return false
  }

  return {
    isDark,
    toggleDark,
    sidebarCollapse,
    toggleSidebar,
    isMobile,
    showDrawerSidebar,
    toggleDrawerSidebar,
    serverUrl,
    setServerUrl,
    pushBackAction,
    popBackAction,
    handleBack,
    backActionStack
  }
})
