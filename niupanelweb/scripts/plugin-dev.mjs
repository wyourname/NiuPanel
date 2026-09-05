import { readFileSync, realpathSync, statSync } from 'node:fs'
import { resolve, join } from 'node:path'

export function readPluginDirectories(raw = '[]') {
  const directories = JSON.parse(raw)
  if (!Array.isArray(directories) || directories.some(value => typeof value !== 'string')) {
    throw new Error('NIUPANEL_DEV_PLUGINS must be a JSON array of plugin directories')
  }
  const ids = new Set()
  return directories.map(directory => {
    const root = realpathSync(resolve(directory))
    const manifest = JSON.parse(readFileSync(join(root, 'plugin.json'), 'utf8'))
    if (!/^[a-zA-Z0-9][a-zA-Z0-9_-]*$/.test(manifest.id ?? '')) {
      throw new Error(`Invalid plugin id in ${root}`)
    }
    if (ids.has(manifest.id)) throw new Error(`Duplicate development plugin: ${manifest.id}`)
    ids.add(manifest.id)
    const ui = realpathSync(join(root, 'ui'))
    const entry = join(ui, 'src/plugin.ts')
    if (!statSync(entry).isFile()) throw new Error(`Missing UI entry: ${entry}`)
    return { id: manifest.id, ui, entry }
  })
}

export function pluginDevelopment() {
  const virtualId = 'virtual:niupanel-plugin-dev'
  const resolvedId = '\0' + virtualId
  let plugins = []
  return {
    name: 'niupanel-plugin-development',
    config(config, { command }) {
      // Never consume local source overrides in release builds.
      if (command !== 'serve') return
      plugins = readPluginDirectories(process.env.NIUPANEL_DEV_PLUGINS)
      if (!plugins.length) return
      return {
        resolve: { dedupe: ['vue', '@vue/runtime-core', '@vue/runtime-dom', '@vue/reactivity', '@vue/shared'] },
        server: { fs: { allow: [resolve(config.root || process.cwd()), resolve(config.root || process.cwd(), '../packages/plugin-sdk'), ...plugins.map(p => p.ui)] } },
      }
    },
    resolveId(id) { if (id === virtualId) return resolvedId },
    load(id) {
      if (id !== resolvedId) return
      const entries = plugins.map(p => `${JSON.stringify(p.id)}: () => import(${JSON.stringify(p.entry.replaceAll('\\', '/'))})`)
      return `export default {${entries.join(',')}}`
    },
    configureServer(server) {
      for (const plugin of plugins) {
        server.config.logger.info(`[plugin-dev] ${plugin.id}: ${plugin.entry}`)
      }
    },
  }
}
