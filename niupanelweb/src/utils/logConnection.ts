export type LogConnectionState = 'idle' | 'connecting' | 'syncing' | 'live' | 'reconnecting' | 'offline' | 'ended'

export function createLogConnection(options: {
  open: () => EventSource
  write: (content: string, reset: boolean) => void
  status: (state: LogConnectionState) => void
  readMissing?: (offset: number, limit: number) => Promise<{ content: string; offset: number; length: number }>
}) {
  let source: EventSource | null = null
  let timer: ReturnType<typeof setTimeout> | undefined
  let stopped = false
  let attempt = 0
  let connectedOnce = false
  let offset = -1
  const disconnect = () => { source?.close(); source = null; clearTimeout(timer) }
  const connect = () => {
    if (stopped) return
    disconnect()
    if (!navigator.onLine) { options.status('offline'); return }
    options.status(connectedOnce ? 'reconnecting' : 'connecting')
    const current = options.open()
    source = current
    let queue = Promise.resolve()
    const fail = () => {
      if (source !== current) return
      disconnect()
      options.status(navigator.onLine ? 'reconnecting' : 'offline')
      if (navigator.onLine) timer = setTimeout(connect, Math.min(30000, 1000 * 2 ** Math.min(attempt++, 5)))
    }
    const enqueue = (action: () => void | Promise<void>) => { queue = queue.then(action).catch(fail) }
    current.onopen = () => { if (source === current) options.status('syncing') }
    current.addEventListener('history', (event: MessageEvent) => enqueue(async () => {
      if (source !== current) return
      const parsedEnd = event.lastEventId ? Number(event.lastEventId) : -1
      const end = Number.isSafeInteger(parsedEnd) && parsedEnd >= 0 ? parsedEnd : -1
      if (connectedOnce && offset >= 0 && end >= offset && options.readMissing) {
        options.status('syncing')
        while (offset < end) {
          const chunk = await options.readMissing(offset, Math.min(64 * 1024, end - offset))
          if (source !== current) return
          if (chunk.offset !== offset || chunk.length <= 0) {
            offset = -1
            throw new Error('日志位置变化，重新同步')
          }
          options.write(chunk.content, false)
          offset += chunk.length
        }
      } else {
        // Keep the old view during reconnect; replace only with a fresh snapshot.
        options.write(event.data, true)
        offset = end
      }
      connectedOnce = true
      attempt = 0
      options.status('live')
    }))
    const write = (event: MessageEvent) => {
      if (source !== current) return
      let content = event.data
      try {
        const payload = JSON.parse(event.data)
        if (typeof payload === 'string') content = payload
        else if (typeof payload?.content === 'string') {
          content = payload.content
          if (typeof payload.offset === 'number') {
            if (payload.offset <= offset) return
            const bytes = new TextEncoder().encode(content)
            const overlap = offset - (payload.offset - bytes.length)
            if (overlap > 0) content = new TextDecoder().decode(bytes.subarray(overlap))
            offset = payload.offset
          }
        }
      } catch { /* Older servers send plain text. */ }
      options.write(content, false)
      connectedOnce = true
      attempt = 0
      options.status('live')
    }
    current.addEventListener('log', (event: MessageEvent) => enqueue(() => write(event)))
    current.addEventListener('resync', () => enqueue(fail))
    current.onmessage = event => enqueue(() => write(event))
    current.addEventListener('end', () => enqueue(() => {
      if (source !== current) return
      disconnect()
      stopped = true
      options.status('ended')
    }))
    current.onerror = () => enqueue(fail)
  }
  const online = () => { if (!source && !stopped) connect() }
  const offline = () => { if (!stopped) { disconnect(); options.status('offline') } }
  window.addEventListener('online', online)
  window.addEventListener('offline', offline)
  connect()
  return {
    retry: () => { stopped = false; attempt = 0; connect() },
    close: () => {
      stopped = true
      disconnect()
      window.removeEventListener('online', online)
      window.removeEventListener('offline', offline)
    },
  }
}
