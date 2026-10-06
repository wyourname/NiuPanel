<template>
  <div class="mobile-code-editor flex h-full min-h-0 flex-1 flex-col overflow-hidden">
    <div class="mobile-editor-actions" role="toolbar" aria-label="编辑工具">
      <button type="button" aria-label="撤销" title="撤销" :disabled="!canUndo" @pointerdown.prevent @click="editor.undo"><span class="i-ep-refresh-left" /></button>
      <button type="button" aria-label="重做" title="重做" :disabled="!canRedo" @pointerdown.prevent @click="editor.redo"><span class="i-ep-refresh-right" /></button>
      <span class="action-divider" />
      <button type="button" aria-label="查找和替换" :aria-pressed="showSearch" @click="toggleSearch"><span class="i-ep-search" /></button>
      <button type="button" aria-label="跳转行号" :aria-pressed="showGoTo" @click="openGoTo"><span class="i-ep-position" /></button>
      <button type="button" aria-label="自动换行" :aria-pressed="wrap" @pointerdown.prevent @click="wrap = !wrap"><span class="i-carbon-text-wrap" /></button>
      <el-dropdown trigger="click" @command="handleCommand">
        <button type="button" aria-label="更多编辑选项"><span class="i-ep-more-filled" /></button>
        <template #dropdown>
          <el-dropdown-menu class="modern-dropdown">
            <el-dropdown-item command="symbols">{{ showSymbols ? '隐藏符号栏' : '显示符号栏' }}</el-dropdown-item>
            <el-dropdown-item command="larger" :disabled="fontSize >= 22">放大字号</el-dropdown-item>
            <el-dropdown-item command="smaller" :disabled="fontSize <= 12">缩小字号</el-dropdown-item>
            <el-dropdown-item command="outdent" divided>减少缩进</el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>

    <section v-if="showSearch" class="mobile-editor-search" aria-label="查找替换面板">
      <div class="search-row">
        <input ref="searchInput" v-model="query" aria-label="查找内容" placeholder="查找内容" autocomplete="off" @keydown.enter.prevent="($event.shiftKey ? editor.findPrevious : editor.findNext)()" />
        <button type="button" aria-label="区分大小写" :aria-pressed="caseSensitive" @click="caseSensitive = !caseSensitive">Aa</button>
        <button type="button" aria-label="关闭查找" @click="closeSearch"><span class="i-ep-close" /></button>
      </div>
      <div class="search-row">
        <input v-model="replacement" aria-label="替换为" placeholder="替换为" autocomplete="off" @keydown.enter.prevent="editor.replaceNext" />
        <button type="button" :disabled="!matchCount" @click="editor.replaceNext">替换</button>
        <button type="button" :disabled="!matchCount" @click="editor.replaceAll">全部</button>
      </div>
      <div class="search-results">
        <span role="status">{{ query ? (matchCount ? `${matchCount >= 10000 ? '10000+' : matchCount} 处匹配` : '没有匹配结果') : '输入文字开始查找' }}</span>
        <button type="button" aria-label="上一个匹配" :disabled="!matchCount" @pointerdown.prevent @click="editor.findPrevious"><span class="i-ep-arrow-up" /></button>
        <button type="button" aria-label="下一个匹配" :disabled="!matchCount" @pointerdown.prevent @click="editor.findNext"><span class="i-ep-arrow-down" /></button>
      </div>
    </section>

    <form v-if="showGoTo" class="mobile-editor-goto search-row" @submit.prevent="jumpToLine">
      <input ref="lineInput" v-model="targetLine" type="number" inputmode="numeric" min="1" :max="lineCount" aria-label="目标行号" :placeholder="`行号（1–${lineCount}）`" required />
      <button type="submit">跳转</button>
      <button type="button" aria-label="关闭行号跳转" @click="showGoTo = false"><span class="i-ep-close" /></button>
    </form>

    <div ref="container" class="mobile-editor-document min-h-0 flex-1 overflow-hidden" @keydown.esc.stop="closePanels" />

    <div class="mobile-editor-status">
      <button type="button" class="cursor-position" aria-label="当前位置，点击跳转行号" @click="openGoTo">{{ line }}:{{ column }} <span>/ {{ lineCount }} 行</span></button>
      <span class="editor-format">UTF-8 · {{ editor.lineEnding === '\r\n' ? 'CRLF' : 'LF' }} · {{ languageName }}</span>
    </div>
    <div v-if="showSymbols" class="mobile-editor-symbols" role="toolbar" aria-label="快捷输入">
      <div class="cursor-actions">
        <button type="button" aria-label="光标左移" @pointerdown.prevent @click="editor.moveCursor(-1)"><span class="i-ep-arrow-left" /></button>
        <button type="button" aria-label="光标右移" @pointerdown.prevent @click="editor.moveCursor(1)"><span class="i-ep-arrow-right" /></button>
      </div>
      <div class="symbol-scroll">
        <button type="button" aria-label="增加缩进" @pointerdown.prevent @click="editor.indent">Tab</button>
        <button v-for="symbol in symbols" :key="symbol" type="button" :aria-label="`输入 ${symbol}`" @pointerdown.prevent @click="editor.insert(symbol)">{{ symbol }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { nextTick, ref } from 'vue'
import { useMobileCodeEditor } from '../composables/useMobileCodeEditor'
import { useMobileBackCloseAction } from '@/composables/useMobileBackCloseAction'
import { useAppStore } from '@/stores/app'

const props = defineProps<{ content: string; fileName: string; isDark: boolean }>()
const emit = defineEmits<{ (event: 'update:content', content: string): void; (event: 'save'): void }>()
const container = ref<HTMLElement | null>(null)
const searchInput = ref<HTMLInputElement | null>(null), lineInput = ref<HTMLInputElement | null>(null)
const showSearch = ref(false), showGoTo = ref(false), showSymbols = ref(true)
const targetLine = ref<number | string>(1)
const symbols = ['{', '}', '(', ')', '[', ']', '=', ':', ';', '"', "'", '/', '\\', '$', '_', '-', '+', '#', '&', '|', '<', '>']
const openSearch = async () => { showSearch.value = true; showGoTo.value = false; await nextTick(); searchInput.value?.focus() }
const editor = useMobileCodeEditor(container, {
  content: () => props.content, fileName: () => props.fileName, dark: () => props.isDark,
  update: content => emit('update:content', content), save: () => emit('save'), openSearch,
})
const { wrap, fontSize, canUndo, canRedo, query, replacement, caseSensitive, matchCount, line, column, lineCount, languageName } = editor
const closeSearch = () => { showSearch.value = false; query.value = ''; editor.focus() }
const toggleSearch = () => { if (showSearch.value) closeSearch(); else void openSearch() }
const openGoTo = async () => {
  showGoTo.value = !showGoTo.value; showSearch.value = false; query.value = ''; targetLine.value = line.value
  await nextTick(); if (showGoTo.value) { lineInput.value?.focus(); lineInput.value?.select() }
}
const jumpToLine = () => { editor.goToLine(Number(targetLine.value)); showGoTo.value = false }
const closePanels = () => { showGoTo.value = false; closeSearch() }
const handleCommand = (command: string) => {
  if (command === 'symbols') showSymbols.value = !showSymbols.value
  if (command === 'larger') fontSize.value = Math.min(22, fontSize.value + 1)
  if (command === 'smaller') fontSize.value = Math.max(12, fontSize.value - 1)
  if (command === 'outdent') editor.outdent()
}
const appStore = useAppStore()
useMobileBackCloseAction({ appStore, visible: showSearch, close: closeSearch })
useMobileBackCloseAction({ appStore, visible: showGoTo, close: () => { showGoTo.value = false } })
</script>

<style scoped>
.mobile-code-editor { background: var(--editor-bg); color: var(--text-default); }
button { display: inline-flex; min-height: 44px; min-width: 44px; align-items: center; justify-content: center; flex-shrink: 0; gap: 4px; border: 0; border-radius: 6px; background: transparent; color: var(--text-secondary); cursor: pointer; font-size: 13px; }
button:disabled { opacity: .35; cursor: default; }
button:focus-visible { outline: 2px solid var(--el-color-primary); outline-offset: -2px; }
button:active:not(:disabled), button[aria-pressed="true"] { background: var(--bg-soft); color: var(--el-color-primary); }
button > span[class^="i-"] { width: 18px; height: 18px; }
.mobile-editor-actions { display: flex; flex-shrink: 0; align-items: center; justify-content: space-between; gap: 2px; padding: 0 6px; border-bottom: 1px solid var(--border-light); background: var(--bg-card); }
.action-divider { width: 1px; height: 18px; background: var(--border-light); }
.mobile-editor-search, .mobile-editor-goto { padding: 6px 8px; border-bottom: 1px solid var(--border-light); background: var(--bg-card); flex-shrink: 0; }
.search-row { display: flex; align-items: center; gap: 6px; }
.search-row + .search-row { margin-top: 6px; }
.search-row input { box-sizing: border-box; width: 0; min-width: 0; flex: 1; height: 40px; padding: 0 10px; border: 1px solid var(--border-light); border-radius: 6px; background: var(--bg-base); color: var(--text-default); font-size: 16px; outline-color: var(--el-color-primary); }
.search-results { display: flex; align-items: center; gap: 6px; color: var(--text-muted); font-size: 11px; }
.search-results > span { flex: 1; }
.mobile-editor-status { display: flex; flex-shrink: 0; align-items: center; gap: 12px; padding: 0 10px; border-top: 1px solid var(--border-light); background: var(--bg-card); font-size: 10px; color: var(--text-muted); }
.cursor-position { min-height: 36px; gap: 5px; color: var(--text-secondary); font-size: 11px; font-variant-numeric: tabular-nums; }
.cursor-position span { color: var(--text-muted); }
.editor-format { overflow: hidden; flex: 1; text-align: right; text-overflow: ellipsis; white-space: nowrap; }
.mobile-editor-symbols { display: flex; flex-shrink: 0; min-width: 0; border-top: 1px solid var(--border-light); background: var(--bg-card); padding-bottom: env(safe-area-inset-bottom, 0px); }
.cursor-actions { display: flex; flex-shrink: 0; border-right: 1px solid var(--border-light); }
.symbol-scroll { display: flex; min-width: 0; overflow-x: auto; overscroll-behavior-x: contain; scrollbar-width: none; }
.symbol-scroll button { font-family: monospace; font-size: 16px; }
</style>
