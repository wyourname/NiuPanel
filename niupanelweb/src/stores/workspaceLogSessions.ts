import { onScopeDispose, shallowReactive } from 'vue'
import { defineStore } from 'pinia'
import * as taskApi from '@/api/tasks'
import { createLogConnection, type LogConnectionState } from '@/utils/logConnection'

type LogSubscriber = (content: string, reset?: boolean) => void

type WorkspaceLogSession = {
  runId: number | null
  buffer: string[]
  connection: ReturnType<typeof createLogConnection> | null
  subscribers: Set<LogSubscriber>
  status: LogConnectionState
}

export const useWorkspaceLogSessionStore = defineStore('workspace-log-sessions', () => {
  const sessions = shallowReactive(new Map<string, WorkspaceLogSession>())
  const ensureSession = (key: string, runId: number | null) => {
    let session = sessions.get(key)
    if (!session || session.runId !== runId) {
      session?.connection?.close()
      session = shallowReactive({ runId, buffer: [], connection: null, subscribers: new Set<LogSubscriber>(), status: 'idle' as LogConnectionState })
      sessions.set(key, session)
    }
    return session
  }
  const connectLive = (key: string, taskId: number, runId: number | null) => {
    const session = ensureSession(key, runId)
    if (!session.connection) session.connection = createLogConnection({
      open: () => runId ? taskApi.streamTaskRunLogs(taskId, runId) : taskApi.streamTaskLogs(taskId),
      readMissing: async (offset, limit) => (await (runId ? taskApi.getTaskRunLog(taskId, runId, offset, limit) : taskApi.getLatestLog(taskId, offset, limit))).data,
      status: state => { session.status = state },
      write: (content, reset) => {
        if (reset) session.buffer = [content]
        else session.buffer.push(content)
        if (session.buffer.length > 3000) session.buffer.splice(0, session.buffer.length - 3000)
        session.subscribers.forEach(subscriber => subscriber(content, reset))
      },
    })
    return session
  }
  const subscribe = (key: string, _taskId: number, runId: number | null, subscriber: LogSubscriber) => {
    const session = ensureSession(key, runId)
    session.subscribers.add(subscriber)
    session.buffer.forEach((content, index) => subscriber(content, index === 0))
    return () => {
      session.subscribers.delete(subscriber)
      if (!session.subscribers.size) {
        session.connection?.close()
        session.connection = null
        sessions.delete(key)
      }
    }
  }
  const closeSession = (key: string) => { sessions.get(key)?.connection?.close(); sessions.delete(key) }
  const retry = (key: string) => sessions.get(key)?.connection?.retry()
  onScopeDispose(() => { for (const key of sessions.keys()) closeSession(key) })
  return { sessions, connectLive, subscribe, closeSession, retry }
})
