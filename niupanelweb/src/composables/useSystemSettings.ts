import { reactive, toRefs } from 'vue'
import * as settingsApi from '../api/settings'
import type { SettingItem } from '@/types'

interface SystemSettingsState {
  systemName: string;
  logoUrl: string;
  loading: boolean;
}

const state = reactive<SystemSettingsState>({
  systemName: 'NiuPanel',
  logoUrl: '',
  loading: false
})

export const useSystemSettings = () => {
  const findSetting = (items: SettingItem[], key: string) =>
    items.find((item) => item.key === key)

  const fetchSystemSettings = async () => {
    state.loading = true
    try {
      const res = await settingsApi.getSettings()
      const data = res.data || []

      const nameSetting = findSetting(data, 'system.name')
      if (nameSetting) state.systemName = nameSetting.value

      const logoSetting = findSetting(data, 'system.logo')
      if (logoSetting) state.logoUrl = logoSetting.value

      updateDocumentTitle()
    } catch (e) {
      console.error('Failed to fetch system settings', e)
    } finally {
      state.loading = false
    }
  }

  const updateSystemSettingsState = (name?: string, logo?: string) => {
    if (name !== undefined) state.systemName = name
    if (logo !== undefined) state.logoUrl = logo
    updateDocumentTitle()
  }

  const updateDocumentTitle = () => {
      document.title = `${state.systemName} - 脚本管理面板`
  }

  return {
    ...toRefs(state),
    fetchSystemSettings,
    updateSystemSettingsState
  }
}
