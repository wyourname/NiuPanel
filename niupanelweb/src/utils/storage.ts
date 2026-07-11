const port = window.location.port || '80'

export function storageKey(base: string): string {
  return `${base}_${port}`
}
