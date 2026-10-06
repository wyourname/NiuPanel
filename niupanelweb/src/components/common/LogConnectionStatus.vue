<template>
  <div v-if="state && state !== 'idle'" class="flex shrink-0 items-center gap-2 border-b border-light bg-card px-3 text-xs text-secondary" role="status">
    <span :class="state === 'live' ? 'i-ep-circle-check text-emerald-600' : state === 'offline' ? 'i-ep-warning text-amber-600' : state === 'ended' ? 'i-ep-check' : 'i-ep-loading animate-spin'" aria-hidden="true" />
    <span class="min-w-0 flex-1 py-2">{{ labels[state] }}</span>
    <button v-if="['offline', 'reconnecting'].includes(state)" type="button" class="min-h-11 shrink-0 px-2 text-primary" @click="$emit('retry')">重新连接</button>
  </div>
</template>
<script setup lang="ts">
import type { LogConnectionState } from '@/utils/logConnection'
defineProps<{ state?: LogConnectionState }>()
defineEmits<{ (event: 'retry'): void }>()
const labels: Record<LogConnectionState, string> = { idle: '', connecting: '正在连接日志…', syncing: '正在同步历史日志…', live: '日志实时更新中', reconnecting: '日志连接中断，正在重连…', offline: '网络已断开，恢复后自动同步日志', ended: '本次日志输出已结束' }
</script>
