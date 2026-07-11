import { useUserStore } from '@/stores/user'

/**
 * 检查当前用户是否拥有指定权限
 * @param permission 权限标识符, 如 'task:run'
 */
export const hasPermission = (permission: string): boolean => {
  const userStore = useUserStore()
  const perms = userStore.userInfo?.permissions || []

  // 管理员拥有所有权限
  if (userStore.userInfo?.role === 'admin') {
    return true
  }

  // 支持通配符匹配 (如 'task:*' 匹配 'task:run')
  const [resource, action] = permission.split(':')

  return perms.some(p => {
    if (p === '*' || p === permission) return true

    const [pResource, pAction] = p.split(':')
    if (pResource === resource && (pAction === '*' || pAction === action)) {
      return true
    }

    return false
  })
}
