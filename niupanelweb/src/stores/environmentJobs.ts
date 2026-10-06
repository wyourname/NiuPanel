import { computed, onScopeDispose, ref, watch } from 'vue'
import { defineStore } from 'pinia'
import { getJobs } from '@/api/jobs'
import { useAppStore } from './app'
import { useUserStore } from './user'
import type { Env, Job } from '@/types'

export const environmentJobKey = (env: Pick<Env, 'env_type' | 'name'>) => `${env.env_type}:${env.name}`
export const isJobActive = (job?: Job) => !!job && ['Pending', 'Running'].includes(job.status)

export const useEnvironmentJobsStore = defineStore('environment-jobs', () => {
  const jobs = ref<Record<string, Job>>({})
  const restoring = ref(false)
  const error = ref('')
  let generation = 0
  let pending: Promise<void> | null = null
  let timer: ReturnType<typeof setTimeout> | undefined
  const app = useAppStore()
  const user = useUserStore()
  const scope = computed(() => `${app.serverUrl}:${user.userInfo.username}`)

  const restore = (background = false): Promise<void> => {
    if (pending) return pending
    const version = generation
    const knownIds = new Set(Object.values(jobs.value).map(job => job.id))
    restoring.value = !background
    clearTimeout(timer)
    const request = (async () => {
      try {
        const response = await getJobs({ skipMessage: true })
        if (version !== generation) return
        const next: Record<string, Job> = {}
        for (const job of response.data) {
          const meta = job.metadata as Record<string, unknown> | null
          if (meta?.kind !== 'environment-packages' || typeof meta.env_type !== 'string' || typeof meta.env_name !== 'string') continue
          const key = `${meta.env_type}:${meta.env_name}`
          const previous = next[key]
          if (!previous || (isJobActive(job) && !isJobActive(previous)) || (isJobActive(job) === isJobActive(previous) && job.id > previous.id)) next[key] = job
        }
        // A submission that finished while this snapshot was loading must survive it.
        for (const [key, job] of Object.entries(jobs.value)) {
          if (!knownIds.has(job.id) && (!next[key] || next[key]!.id < job.id)) next[key] = job
          if (isJobActive(job) && next[key]?.id === job.id && !isJobActive(next[key])) window.dispatchEvent(new CustomEvent('niu:job-finished'))
        }
        jobs.value = next
        error.value = ''
      } catch {
        if (version === generation) error.value = '安装状态暂时无法获取，正在重试…'
      } finally {
        if (version === generation) {
          restoring.value = false
          pending = null
          if (error.value || Object.values(jobs.value).some(isJobActive)) timer = setTimeout(() => void restore(true), error.value ? 5000 : 2000)
        }
      }
    })()
    pending = request
    return request
  }

  const track = (env: Env, id: number, name: string) => {
    jobs.value[environmentJobKey(env)] = { id, name, status: 'Pending' }
    void restore(true)
  }
  const clear = () => {
    generation++
    clearTimeout(timer)
    pending = null
    jobs.value = {}
    restoring.value = false
    error.value = ''
  }
  watch(scope, clear)
  onScopeDispose(clear)
  return { jobs, restoring, error, restore: () => restore(), track }
})
