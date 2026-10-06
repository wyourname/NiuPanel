<template>
  <div v-if="count" class="file-clipboard-bar" aria-label="文件剪贴板">
    <span :class="action === 'cut' ? 'i-ep-scissor' : 'i-ep-copy-document'" class="shrink-0 text-base text-primary" aria-hidden="true" />
    <div class="min-w-0 flex-1">
      <div class="text-xs font-semibold text-default">{{ action === 'cut' ? '待移动' : '待复制' }} {{ count }} 项</div>
      <div class="mt-0.5 truncate text-[11px] text-muted">目标：{{ path === '/' ? '根目录' : path }}</div>
    </div>
    <button type="button" class="clipboard-paste" :disabled="pasting" @click="emit('paste')">
      <span v-if="pasting" class="i-ep-loading animate-spin" aria-hidden="true" />{{ pasting ? '处理中…' : '粘贴到此处' }}
    </button>
    <button type="button" class="clipboard-clear" :disabled="pasting" aria-label="清空文件剪贴板" @click="emit('clear')"><span class="i-ep-close" /></button>
  </div>
</template>
<script setup lang="ts">
defineProps<{ count: number; action: 'copy' | 'cut' | null; path: string; pasting: boolean }>()
const emit = defineEmits<{ (event: 'paste'): void; (event: 'clear'): void }>()
</script>
<style scoped>
.file-clipboard-bar { display: flex; flex-shrink: 0; align-items: center; gap: 10px; padding: 4px 10px; border-bottom: 1px solid var(--border-light); background: var(--bg-soft); }
.clipboard-paste, .clipboard-clear { display: inline-flex; flex-shrink: 0; align-items: center; justify-content: center; min-height: 44px; gap: 5px; border: 0; border-radius: 6px; cursor: pointer; }
.clipboard-paste { padding: 0 10px; color: var(--el-color-primary); font-size: 12px; font-weight: 600; background: transparent; }
.clipboard-clear { width: 44px; color: var(--text-muted); background: transparent; }
button:disabled { opacity: .5; cursor: default; }
button:hover:not(:disabled) { background: var(--bg-card); }
</style>
