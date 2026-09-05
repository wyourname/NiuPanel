#!/usr/bin/env node
import { spawn } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { readPluginDirectories } from '../niupanelweb/scripts/plugin-dev.mjs'

const args = process.argv.slice(2)
const directories = []
let port = '7787'
let api = 'http://127.0.0.1:7788'
let host = '127.0.0.1'
for (let i = 0; i < args.length; i++) {
  const flag = args[i]
  if (flag === '--help') {
    console.log('node scripts/dev-workspace.mjs --plugin <directory> [--plugin <directory>] [--port 7787] [--api http://127.0.0.1:7788] [--host 127.0.0.1]')
    process.exit(0)
  }
  const value = args[++i]
  if (!value || value.startsWith('--')) throw new Error(`Missing value for ${flag}`)
  if (flag === '--plugin') directories.push(resolve(value))
  else if (flag === '--port') port = value
  else if (flag === '--api') api = value
  else if (flag === '--host') host = value
  else throw new Error(`Unknown option: ${flag}`)
}
if (!/^\d+$/.test(port) || Number(port) < 1 || Number(port) > 65535) throw new Error('Invalid port')
if (!['http:', 'https:'].includes(new URL(api).protocol)) throw new Error('Invalid API URL')
const plugins = readPluginDirectories(JSON.stringify(directories))
const web = fileURLToPath(new URL('../niupanelweb/', import.meta.url))
console.log(`Panel API: ${api}`)
console.log(`UI overrides: ${plugins.map(plugin => plugin.id).join(', ') || '(none)'}`)
console.log('Backend and permissions use installed plugins. This server exposes development source; use a trusted network only.')
const child = spawn(process.execPath, [resolve(web, 'node_modules/vite/bin/vite.js'), '--host', host, '--port', port, '--strictPort'], {
  cwd: web,
  stdio: 'inherit',
  env: { ...process.env, NIUPANEL_DEV_PLUGINS: JSON.stringify(directories), VITE_API_PROXY_TARGET: api },
})
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => child.kill(signal))
child.on('error', error => { console.error(error.message); process.exitCode = 1 })
child.on('exit', (code, signal) => { process.exitCode = code ?? (signal === 'SIGINT' ? 130 : 1) })
