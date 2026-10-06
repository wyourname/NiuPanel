<template>
  <section v-if="transfers.items.length" class="max-h-[28vh] shrink-0 overflow-y-auto border-b border-light bg-card" aria-label="文件传输">
    <article v-for="item in transfers.items" :key="item.id" class="flex items-center gap-2 border-b border-light px-3 py-2 last:border-b-0">
      <span :class="item.direction === 'upload' ? 'i-ep-upload' : 'i-ep-download'" class="shrink-0 text-primary" aria-hidden="true" />
      <div class="min-w-0 flex-1">
        <div class="truncate text-xs font-semibold text-default" :title="item.name">{{ item.name }}</div>
        <div class="mt-1 text-[11px] text-secondary" :class="{ 'text-red-600': item.state === 'error' }">
          {{ item.error || labels[item.state] }}
          <span v-if="item.state === 'running'"> · {{ formatFileSize(item.loaded) }}<template v-if="item.total"> / {{ formatFileSize(item.total) }}</template> · {{ formatFileSize(item.speed) }}/s</span>
        </div>
        <progress v-if="item.state === 'running'" class="mt-1 h-1 w-full accent-primary" :max="item.total || undefined" :value="item.total ? item.loaded : undefined" aria-label="传输进度" />
      </div>
      <button v-if="['running', 'preparing'].includes(item.state)" type="button" class="min-h-11 min-w-11 px-2 text-xs text-primary" @click="transfers.cancel(item.id)">取消</button>
      <button v-if="['error', 'cancelled'].includes(item.state)" type="button" class="min-h-11 min-w-11 px-2 text-xs text-primary" @click="transfers.retry(item.id)">重试</button>
      <button v-if="!['running', 'preparing'].includes(item.state)" type="button" class="min-h-11 min-w-11 text-muted" aria-label="移除传输记录" @click="transfers.dismiss(item.id)"><span class="i-ep-close" /></button>
    </article>
  </section>
</template>
<script setup lang="ts">
import { useFileTransfersStore } from '@/stores/fileTransfers'
import { formatFileSize } from '@/utils/format'
const transfers = useFileTransfersStore()
const labels = { preparing: '准备中…', running: '传输中', success: '已完成', cancelled: '已取消', error: '传输失败', browser: '已交给浏览器，请在下载列表查看进度或取消' }
</script>
