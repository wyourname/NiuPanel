import { defineStore } from 'pinia'
import { ref } from 'vue'
import { login as apiLogin, logout as apiLogout, getSetupStatus as apiGetSetupStatus } from '../api/auth'
import * as userApi from '../api/user'
import type { LoginRequest, LoginResponse, UserInfo } from '@/types'
import { storageKey as makeStorageKey } from '@/utils/storage'

const userStorageKey = makeStorageKey('user_info')

export const useUserStore = defineStore('user', () => {
  const userInfo = ref<UserInfo>(JSON.parse(localStorage.getItem(userStorageKey) || '{}'))
  const isInitialized = ref<boolean | null>(null)

  const isLoginSuccess = (data: LoginResponse): data is UserInfo => {
    return !('ticket' in data)
  }

  const getHttpStatus = (error: unknown) => {
    if (typeof error !== 'object' || error === null || !('response' in error)) return undefined
    const response = (error as { response?: { status?: number } }).response
    return response?.status
  }

  const setUserInfo = (info: UserInfo) => {
    userInfo.value = info
    localStorage.setItem(userStorageKey, JSON.stringify(info))
  }

  const clearUser = () => {
    userInfo.value = {}
    localStorage.removeItem(userStorageKey)
  }

  const login = async (form: LoginRequest) => {
    const res = await apiLogin(form)
    if (res.data && isLoginSuccess(res.data)) {
      setUserInfo(res.data)
      await fetchUserProfile()
    }
    return res
  }

  const fetchUserProfile = async () => {
    try {
      const res = await userApi.getMyProfile()
      setUserInfo(res.data)
      return res.data
    } catch {
      return null
    }
  }

  const logout = async () => {
    try {
      await apiLogout()
    } finally {
      clearUser()
    }
  }

  const checkSystemStatus = async () => {
    try {
      const res = await apiGetSetupStatus()
      isInitialized.value = res.data.initialized
      return res.data.initialized
    } catch (e: unknown) {
      console.error('Check status error:', e)
      const status = getHttpStatus(e)
      if (status === 401 || status === 403) {
          isInitialized.value = true
          return true
      }
      isInitialized.value = true
      return true
    }
  }

  return {
    userInfo,
    isInitialized,
    login,
    logout,
    checkSystemStatus,
    setUserInfo,
    fetchUserProfile
  }
})
