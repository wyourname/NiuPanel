<template>
  <ResponsiveDialog
    :visible="visible"
    :title="currentFile?.name || '文件编辑器'"
    :show-header="false"
    :show-close="false"
    :close-on-click-modal="false"
    :before-close="beforeClose"
    :style="viewportStyle"
    desktop-size="fluid"
    desktop-height="min(760px, calc(var(--app-viewport-height) - 48px))"
    content-preset="workspace"
    size="100%"
    destroy-on-close
    append-to-body
    custom-class="editor-overlay file-editor-overlay"
    @update:visible="emit('update:visible', $event)"
  >
    <div class="flex h-full min-h-0 flex-1 flex-col overflow-hidden bg-[var(--editor-bg)]">
      <header class="file-editor-header">
        <button type="button" class="editor-back" aria-label="关闭文件编辑器" @click="requestClose">
          <span class="i-ep-arrow-left" aria-hidden="true" />
        </button>
        <div class="editor-file-title">
          <div class="editor-file-name">
            <span v-if="dirty" class="unsaved-dot" aria-hidden="true" />
            <strong>{{ currentFile?.name || '文件编辑器' }}</strong>
          </div>
          <div class="editor-file-subtitle">
            <span :class="{ 'text-amber-600 dark:text-amber-300': dirty }">{{ loading ? '读取中…' : loadError ? '读取失败' : dirty ? '未保存' : '已保存' }}</span>
            <span class="file-location">{{ parentPath }}</span>
          </div>
        </div>
        <button type="button" class="editor-save" :disabled="saving || loading || !!loadError || !dirty" aria-label="保存文件" @click="emit('save')">
          <span :class="saving ? 'i-ep-loading animate-spin' : 'i-ep-document-checked'" aria-hidden="true" />
          {{ saving ? '保存中' : '保存' }}
        </button>
      </header>

      <div v-if="loading" class="min-h-0 flex-1 flex-center gap-2 text-sm text-muted" role="status">
        <span class="i-ep-loading animate-spin" />正在读取文件…
      </div>
      <div v-else-if="loadError" class="min-h-0 flex-1 flex flex-col items-center justify-center gap-3 px-5 text-center" role="alert">
        <span class="i-ep-warning text-2xl text-amber-500" />
        <p class="text-sm text-secondary">{{ loadError }}</p>
        <button type="button" class="h-11 rounded-md bg-soft px-5 text-sm font-semibold text-primary" @click="emit('retry')">重新读取</button>
      </div>
      <FileMobileCodeEditor
        v-else-if="visible"
        :key="currentFile?.path"
        v-model:content="contentValue"
        :file-name="currentFile?.name || ''"
        :is-dark="isDark"
        @save="emit('save')"
      />
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent, onBeforeUnmount, onMounted, ref, type CSSProperties } from 'vue'
import type { FileItem } from '@/composables/useFileOperations'
import ResponsiveDialog from '@/components/common/ResponsiveDialog.vue'

const FileMobileCodeEditor = defineAsyncComponent(() => import('./FileMobileCodeEditor.vue'))
const props = defineProps<{
  content: string; currentFile: FileItem | null; isDark: boolean; isMobile: boolean
  saving: boolean; visible: boolean; dirty: boolean; loading: boolean; loadError: string
  confirmClose: () => Promise<boolean>
}>()
const emit = defineEmits<{
  (event: 'save'): void
  (event: 'retry'): void
  (event: 'update:content', content: string): void
  (event: 'update:visible', visible: boolean): void
}>()
const contentValue = computed({ get: () => props.content, set: (content: string) => emit('update:content', content) })
const parentPath = computed(() => props.currentFile?.path.split('/').slice(0, -1).join('/') || '根目录')
const viewportHeight = ref(0), viewportTop = ref(0)
const updateViewport = () => {
  viewportHeight.value = window.visualViewport?.height ?? window.innerHeight
  viewportTop.value = window.visualViewport?.offsetTop ?? 0
}
const viewportStyle = computed<CSSProperties | undefined>(() => props.isMobile && viewportHeight.value ? {
  '--file-editor-height': `${viewportHeight.value}px`,
  '--file-editor-top': `${viewportTop.value}px`,
} : undefined)
const requestClose = async () => { if (await props.confirmClose()) emit('update:visible', false) }
const beforeClose = async (done: () => void) => { if (await props.confirmClose()) done() }
onMounted(() => {
  updateViewport()
  window.visualViewport?.addEventListener('resize', updateViewport)
  window.visualViewport?.addEventListener('scroll', updateViewport)
  window.addEventListener('resize', updateViewport)
})
onBeforeUnmount(() => {
  window.visualViewport?.removeEventListener('resize', updateViewport)
  window.visualViewport?.removeEventListener('scroll', updateViewport)
  window.removeEventListener('resize', updateViewport)
})
</script>

<style scoped>
.file-editor-header { display: flex; align-items: center; flex-shrink: 0; gap: 6px; min-height: 58px; padding: env(safe-area-inset-top, 0px) 8px 0 2px; border-bottom: 1px solid var(--border-light); background: var(--bg-card); }
.editor-back, .editor-save { display: inline-flex; flex-shrink: 0; align-items: center; justify-content: center; gap: 5px; min-height: 44px; min-width: 44px; border: 0; border-radius: 6px; cursor: pointer; }
.editor-back { color: var(--text-secondary); background: transparent; font-size: 19px; }
.editor-back:active { background: var(--bg-soft); }
.editor-save { padding: 0 10px; color: var(--el-color-primary); background: var(--bg-soft); font-size: 13px; font-weight: 600; }
.editor-save:disabled { color: var(--text-muted); background: transparent; cursor: default; opacity: .65; }
.editor-file-title { flex: 1; min-width: 0; }
.editor-file-name { display: flex; align-items: center; gap: 5px; }
.editor-file-name strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-default); font-size: 14px; }
.unsaved-dot { width: 6px; height: 6px; flex-shrink: 0; border-radius: 50%; background: #d97706; }
.editor-file-subtitle { display: flex; gap: 8px; margin-top: 3px; font-size: 10px; color: var(--text-muted); }
.editor-file-subtitle > span:first-child { flex-shrink: 0; }
.file-location { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
<style>
@media (max-width: 768px) {
  .file-editor-overlay.overlay-drawer--workspace.el-drawer {
    top: var(--file-editor-top, 0px) !important;
    bottom: auto !important;
    height: var(--file-editor-height, var(--app-viewport-height)) !important;
    max-height: var(--file-editor-height, var(--app-viewport-height)) !important;
  }
}
</style>
