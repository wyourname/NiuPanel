import { StreamLanguage, type LanguageSupport } from '@codemirror/language'
import type { Extension } from '@codemirror/state'

export async function loadMobileEditorLanguage(language: string): Promise<Extension | LanguageSupport> {
  switch (language) {
    case 'javascript':
    case 'typescript': return (await import('@codemirror/lang-javascript')).javascript({ typescript: language === 'typescript', jsx: true })
    case 'json': return (await import('@codemirror/lang-json')).json()
    case 'python': return (await import('@codemirror/lang-python')).python()
    case 'shell': return StreamLanguage.define((await import('@codemirror/legacy-modes/mode/shell')).shell)
    case 'yaml': return StreamLanguage.define((await import('@codemirror/legacy-modes/mode/yaml')).yaml)
    case 'toml': return StreamLanguage.define((await import('@codemirror/legacy-modes/mode/toml')).toml)
    default: return []
  }
}
