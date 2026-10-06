import { ref, watch, onScopeDispose } from 'vue'
import { defineStore } from 'pinia'
import request from '@/utils/request'
import { uploadFile } from '@/api/file_manager'
import { useAppStore } from './app'
import { useUserStore } from './user'

export type Transfer = {
  id: number; name: string; direction: 'upload' | 'download'; destination?: string
  state: 'preparing' | 'running' | 'success' | 'error' | 'cancelled' | 'browser'
  loaded: number; total: number; speed: number; error: string
}
type SaveHandle = { createWritable: () => Promise<WritableStream<Uint8Array>> }
type PickerWindow = Window & { showSaveFilePicker?: (options: { suggestedName: string }) => Promise<SaveHandle> }

export const useFileTransfersStore = defineStore('file-transfers', () => {
  const items = ref<Transfer[]>([])
  const actions = new Map<number, () => Promise<void>>()
  const controllers = new Map<number, AbortController>()
  const frames = new Map<number, HTMLIFrameElement>()
  let nextId = 0
  const app = useAppStore(), user = useUserStore()
  const apiUrl = (path: string) => `${app.serverUrl.replace(/\/$/, '')}/api/v1${path}`
  const create = (name: string, direction: Transfer['direction'], total = 0) => {
    const item: Transfer = { id: ++nextId, name, direction, state: 'preparing', loaded: 0, total, speed: 0, error: '' }
    items.value.unshift(item)
    return items.value[0]!
  }
  const run = async (item: Transfer, action: (signal: AbortSignal, progress: (loaded: number, total?: number) => void) => Promise<void | 'browser'>) => {
    if (controllers.has(item.id)) return
    const controller = new AbortController()
    controllers.set(item.id, controller)
    item.state = 'preparing'; item.loaded = 0; item.speed = 0; item.error = ''
    let lastTime = performance.now(), lastBytes = 0
    const progress = (loaded: number, total?: number) => {
      const now = performance.now(), elapsed = now - lastTime
      if (elapsed >= 250) { item.speed = Math.max(0, (loaded - lastBytes) * 1000 / elapsed); lastTime = now; lastBytes = loaded }
      item.loaded = loaded
      if (total && total > 0) item.total = total
    }
    try {
      const result = await action(controller.signal, progress)
      if (controller.signal.aborted) item.state = 'cancelled'
      else {
        item.state = result === 'browser' ? 'browser' : 'success'
        if (item.state === 'success') {
          actions.delete(item.id)
          if (item.direction === 'upload') window.dispatchEvent(new CustomEvent('niu:files-changed', { detail: item.destination }))
        }
      }
    } catch (error) {
      const cancelled = controller.signal.aborted || (error instanceof DOMException && error.name === 'AbortError')
      item.state = cancelled ? 'cancelled' : 'error'
      item.error = cancelled ? '' : error instanceof Error ? error.message : '传输失败，请重试'
      controller.abort()
    } finally { item.speed = 0; controllers.delete(item.id) }
  }
  const upload = (files: File[], destination: string) => {
    const item = create(files.length === 1 ? files[0]!.name : `${files.length} 个文件`, 'upload', files.reduce((sum, file) => sum + file.size, 0))
    item.destination = destination
    const action = () => run(item, async (signal, progress) => {
      const form = new FormData()
      files.forEach(file => form.append('files', file))
      item.state = 'running'
      await uploadFile(destination, form, { signal, onUploadProgress: event => progress(event.loaded, event.total) })
    })
    actions.set(item.id, action)
    return action().then(() => item.state === 'success')
  }
  const download = (name: string, paths: string[], batch: boolean) => {
    const item = create(name, 'download')
    const url = apiUrl(batch ? '/files/download_batch' : `/files/download/${paths[0]!.split('/').filter(Boolean).map(encodeURIComponent).join('/')}`)
    const picker = (window as PickerWindow).showSaveFilePicker
    let handle: SaveHandle | undefined
    const action = () => run(item, async (signal, progress) => {
      if (picker && window.isSecureContext) {
        handle ??= await picker.call(window, { suggestedName: name })
        if (signal.aborted) throw new DOMException('已取消', 'AbortError')
        const response = await fetch(url, {
          method: batch ? 'POST' : 'GET', credentials: 'include', signal,
          ...(batch ? { headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ paths }) } : {}),
        })
        if (!response.ok) throw new Error(`下载失败（HTTP ${response.status}）`)
        if (!response.body) throw new Error('浏览器未提供下载数据流')
        item.state = 'running'
        const total = Number(response.headers.get('content-length')) || 0
        let loaded = 0
        const writable = await handle.createWritable()
        await response.body.pipeThrough(new TransformStream<Uint8Array, Uint8Array>({
          transform(chunk, output) { loaded += chunk.byteLength; progress(loaded, total); output.enqueue(chunk) },
        })).pipeTo(writable, { signal })
        return
      }
      // Native downloads stream to disk and expose progress/cancel in the browser's download list.
      if (batch) await request.post('/files/download_batch/check', { paths }, { signal, skipMessage: true, timeout: 0 })
      else {
        const response = await fetch(url, { method: 'HEAD', credentials: 'include', signal })
        if (!response.ok) throw new Error(`下载失败（HTTP ${response.status}）`)
      }
      if (signal.aborted) throw new DOMException('已取消', 'AbortError')
      frames.get(item.id)?.remove()
      const frame = document.createElement('iframe')
      frame.name = `file-download-${item.id}`; frame.hidden = true
      document.body.append(frame); frames.set(item.id, frame)
      if (batch) {
        const form = document.createElement('form')
        form.method = 'POST'; form.action = `${url}/form`; form.target = frame.name; form.acceptCharset = 'UTF-8'
        const input = document.createElement('input')
        input.type = 'hidden'; input.name = 'paths'; input.value = JSON.stringify(paths)
        form.append(input); document.body.append(form); form.submit(); form.remove()
      } else frame.src = url
      return 'browser'
    })
    actions.set(item.id, action)
    return action()
  }
  const cancel = (id: number) => controllers.get(id)?.abort()
  const retry = (id: number) => actions.get(id)?.()
  const dismiss = (id: number) => {
    if (controllers.has(id)) return
    items.value = items.value.filter(item => item.id !== id)
    actions.delete(id); frames.get(id)?.remove(); frames.delete(id)
  }
  const reset = () => {
    controllers.forEach(controller => controller.abort())
    actions.clear(); frames.forEach(frame => frame.remove()); frames.clear(); items.value = []
  }
  watch(() => `${app.serverUrl}:${user.userInfo.username}`, reset)
  const beforeUnload = (event: BeforeUnloadEvent) => {
    if (!controllers.size) return
    event.preventDefault()
    event.returnValue = ''
  }
  window.addEventListener('beforeunload', beforeUnload)
  onScopeDispose(() => { reset(); window.removeEventListener('beforeunload', beforeUnload) })
  return { items, upload, download, cancel, retry, dismiss }
})
