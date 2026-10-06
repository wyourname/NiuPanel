/**
 * Format bytes to a human readable string
 */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

export function formatTransferRate(bytesPerSecond: number | null | undefined): string {
  if (bytesPerSecond == null || !Number.isFinite(bytesPerSecond) || bytesPerSecond < 0) return '--'
  const units = ['B/s', 'KB/s', 'MB/s', 'GB/s', 'TB/s']
  const unit = bytesPerSecond < 1
    ? 0
    : Math.min(Math.floor(Math.log(bytesPerSecond) / Math.log(1024)), units.length - 1)
  const value = bytesPerSecond / 1024 ** unit
  return `${parseFloat(value.toFixed(unit === 0 ? 0 : 1))} ${units[unit]}`
}

/**
 * Format uptime seconds to a human readable string
 */
export function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const mins = Math.floor((seconds % 3600) / 60)
  if (days > 0) return `${days}天 ${hours}小时`
  if (hours > 0) return `${hours}小时 ${mins}分`
  return `${mins}分钟`
}

/**
 * Format date string or timestamp to a locale string
 */
export function formatDate(date: string | number | undefined, emptyText = '-'): string {
  if (!date) return emptyText
  const d = new Date(typeof date === 'number' && date < 10000000000 ? date * 1000 : date)
  return d.toLocaleString()
}
