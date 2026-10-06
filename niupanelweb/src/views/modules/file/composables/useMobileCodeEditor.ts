import { computed, onBeforeUnmount, onMounted, ref, watch, type Ref } from 'vue'
import { Compartment, EditorSelection, EditorState, type Extension } from '@codemirror/state'
import { Decoration, EditorView, MatchDecorator, ViewPlugin, drawSelection, highlightActiveLine, keymap, lineNumbers } from '@codemirror/view'
import { defaultKeymap, history, historyKeymap, indentLess, indentMore, indentWithTab, redo, redoDepth, undo, undoDepth } from '@codemirror/commands'
import { bracketMatching, defaultHighlightStyle, indentUnit, syntaxHighlighting } from '@codemirror/language'
import { SearchQuery, findNext, findPrevious, replaceAll, replaceNext, search, setSearchQuery } from '@codemirror/search'
import { oneDarkHighlightStyle } from '@codemirror/theme-one-dark'
import { getLanguageConfig, getLanguageFromFilename } from '@/utils/editor'
import { loadMobileEditorLanguage } from '../utils/mobileEditorLanguage'

type Options = {
  content: () => string
  fileName: () => string
  dark: () => boolean
  update: (content: string) => void
  save: () => void
  openSearch: () => void
}

export function useMobileCodeEditor(container: Ref<HTMLElement | null>, options: Options) {
  let view: EditorView | undefined
  let disposed = false
  const appearance = new Compartment()
  const wrapping = new Compartment()
  const highlighting = new Compartment()
  const language = new Compartment()
  const wrap = ref(true)
  const fontSize = ref(16)
  const line = ref(1), column = ref(1), lineCount = ref(1)
  const canUndo = ref(false), canRedo = ref(false)
  const query = ref(''), replacement = ref(''), caseSensitive = ref(false)
  const matchCount = ref(0)
  const languageName = computed(() => getLanguageFromFilename(options.fileName()))
  const lineEnding = options.content().includes('\r\n') ? '\r\n' : '\n'

  const theme = () => [syntaxHighlighting(options.dark() ? oneDarkHighlightStyle : defaultHighlightStyle), EditorView.theme({
    '&': { height: '100%', fontSize: `${fontSize.value}px`, color: 'var(--text-default)', backgroundColor: 'var(--editor-bg)' },
    '.cm-scroller': { fontFamily: '"JetBrains Mono", Consolas, monospace', lineHeight: '1.65', overflow: 'auto', overscrollBehavior: 'contain' },
    '.cm-content': { padding: '12px 0', caretColor: 'var(--el-color-primary)' },
    '.cm-line': { padding: '0 12px 0 8px' },
    '.cm-gutters': { backgroundColor: 'var(--editor-bg)', color: 'var(--text-muted)', border: 'none', minWidth: '36px' },
    '.cm-lineNumbers .cm-gutterElement': { padding: '0 8px 0 4px' },
    '.cm-activeLine': { backgroundColor: 'var(--bg-soft)' },
    '.cm-cursor': { borderLeftColor: 'var(--el-color-primary)' },
    '&.cm-focused': { outline: 'none' },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection': { backgroundColor: options.dark() ? '#3b526f' : '#c9ddfa' },
    '.cm-search-hit': { backgroundColor: options.dark() ? '#705a26' : '#ffe8a3', borderRadius: '2px' },
  }, { dark: options.dark() })]

  const updatePosition = () => {
    if (!view) return
    const selected = view.state.selection.main
    const currentLine = view.state.doc.lineAt(selected.head)
    line.value = currentLine.number
    column.value = selected.head - currentLine.from + 1
    lineCount.value = view.state.doc.lines
    canUndo.value = undoDepth(view.state) > 0
    canRedo.value = redoDepth(view.state) > 0
  }

  const searchQuery = () => new SearchQuery({ search: query.value, replace: replacement.value, caseSensitive: caseSensitive.value, literal: true })
  const countMatches = () => {
    matchCount.value = 0
    if (!view || !query.value) return
    // Bound work for very large files while keeping replacement independent of the display limit.
    const cursor = searchQuery().getCursor(view.state)
    while (!cursor.next().done) {
      if (++matchCount.value >= 10000) break
    }
  }
  const updateSearch = () => {
    if (!view) return
    let extension: Extension = []
    if (query.value) {
      const escaped = query.value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      const decorator = new MatchDecorator({ regexp: new RegExp(escaped, caseSensitive.value ? 'g' : 'gi'), decoration: Decoration.mark({ class: 'cm-search-hit' }) })
      extension = ViewPlugin.fromClass(class {
        decorations = decorator.createDeco(view!)
        update(update: Parameters<MatchDecorator['updateDeco']>[0]) { this.decorations = decorator.updateDeco(update, this.decorations) }
      }, { decorations: plugin => plugin.decorations })
    }
    view.dispatch({ effects: [setSearchQuery.of(searchQuery()), highlighting.reconfigure(extension)] })
    countMatches()
  }

  const run = (command: (editor: EditorView) => boolean) => {
    if (!view) return
    command(view)
    view.focus()
  }
  const insert = (text: string) => {
    if (!view) return
    view.dispatch(view.state.replaceSelection(text), { scrollIntoView: true, userEvent: 'input' })
    view.focus()
  }
  const goToLine = (value: number) => {
    if (!view) return
    const target = view.state.doc.line(Math.min(view.state.doc.lines, Math.max(1, Math.floor(value) || 1)))
    view.dispatch({ selection: EditorSelection.cursor(target.from), effects: EditorView.scrollIntoView(target.from, { y: 'center' }) })
    view.focus()
  }
  const moveCursor = (direction: -1 | 1) => {
    if (!view) return
    const position = view.moveByChar(view.state.selection.main, direction === 1)
    view.dispatch({ selection: position, scrollIntoView: true })
    view.focus()
  }

  onMounted(async () => {
    if (!container.value) return
    const config = getLanguageConfig(languageName.value)
    view = new EditorView({
      parent: container.value,
      state: EditorState.create({
        doc: options.content(),
        extensions: [
          history(), drawSelection(), lineNumbers(), highlightActiveLine(), bracketMatching(), search(),
          language.of([]),
          EditorState.lineSeparator.of(lineEnding), EditorState.tabSize.of(config.tabSize),
          indentUnit.of(config.insertSpaces ? ' '.repeat(config.tabSize) : '\t'),
          appearance.of(theme()), wrapping.of(EditorView.lineWrapping), highlighting.of([]),
          EditorView.contentAttributes.of({ 'aria-label': '文件内容', spellcheck: 'false', autocorrect: 'off', autocapitalize: 'off' }),
          keymap.of([
            { key: 'Mod-s', run: () => { options.save(); return true } },
            { key: 'Mod-f', run: () => { options.openSearch(); return true } },
            ...historyKeymap, indentWithTab, ...defaultKeymap,
          ]),
          EditorView.updateListener.of(update => {
            updatePosition()
            if (update.docChanged) { options.update(update.state.sliceDoc()); countMatches() }
          }),
        ],
      }),
    })
    updatePosition()
    const support = await loadMobileEditorLanguage(languageName.value).catch(() => [])
    if (!disposed) view?.dispatch({ effects: language.reconfigure(support) })
  })
  watch(() => options.content(), content => {
    if (view && view.state.sliceDoc() !== content) view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: content } })
  })
  watch([() => options.dark(), fontSize], () => view?.dispatch({ effects: appearance.reconfigure(theme()) }))
  watch(wrap, enabled => view?.dispatch({ effects: wrapping.reconfigure(enabled ? EditorView.lineWrapping : []) }))
  watch([query, replacement, caseSensitive], updateSearch)
  onBeforeUnmount(() => { disposed = true; view?.destroy(); view = undefined })

  return { wrap, fontSize, line, column, lineCount, canUndo, canRedo, languageName,
    query, replacement, caseSensitive, matchCount, lineEnding,
    insert, goToLine, moveCursor,
    undo: () => run(undo), redo: () => run(redo), indent: () => run(indentMore), outdent: () => run(indentLess),
    findNext: () => { if (view) findNext(view) }, findPrevious: () => { if (view) findPrevious(view) },
    replaceNext: () => { if (view) replaceNext(view) }, replaceAll: () => { if (view) replaceAll(view) },
    focus: () => view?.focus(),
  }
}
