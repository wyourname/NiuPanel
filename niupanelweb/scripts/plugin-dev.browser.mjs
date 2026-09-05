import assert from 'node:assert/strict'
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createServer } from 'vite'

// Playwright is an optional local verification dependency, not a shipped runtime dependency.
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright')
const root = await mkdtemp(join(tmpdir(), 'niupanel-hmr-'))
const oldEnv = process.env.NIUPANEL_DEV_PLUGINS
let server
let browser
try {
  await mkdir(join(root, 'ui/src'), { recursive: true })
  await writeFile(join(root, 'plugin.json'), JSON.stringify({ id: 'hmr-test' }))
  await writeFile(join(root, 'ui/src/plugin.ts'), `import {createApp} from 'vue'; import App from './App.vue';
export default { mount(el) { const app = createApp(App); app.mount(el); return app; }, unmount(app) { app.unmount(); } }`)
  const source = label => `<script setup>import {ref} from 'vue'; const value=ref('')</script>
<template><label>${label}<input v-model="value" /></label><p>{{ value }}</p></template>`
  await writeFile(join(root, 'ui/src/App.vue'), source('Before'))
  const additional = process.argv.slice(2)
  process.env.NIUPANEL_DEV_PLUGINS = JSON.stringify([root, ...additional])
  server = await createServer({
    cacheDir: join(root, 'vite-cache'),
    root: fileURLToPath(new URL('../', import.meta.url)),
    configFile: fileURLToPath(new URL('../vite.config.ts', import.meta.url)),
    server: { host: '127.0.0.1', port: 0, open: false },
    plugins: [{ name: 'hmr-test-page', configureServer(vite) {
      vite.middlewares.use('/__hmr_test', (_req, res) => {
        res.setHeader('Content-Type', 'text/html')
        res.end(`<div id="plugin"></div><script type="module">
import '/@vite/client';
import entries from '/@id/__x00__virtual:niupanel-plugin-dev';
window.pageIdentity = Math.random();
for (const load of Object.values(entries)) { const module = await load(); if (typeof module.default.mount !== 'function') throw Error('Invalid entry'); }
window.instance = (await entries['hmr-test']()).default.mount(document.querySelector('#plugin'));
</script>`)
      })
    } }],
  })
  await server.listen()
  browser = await chromium.launch({ headless: true, ...(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {}) })
  const page = await browser.newPage()
  const errors = []
  page.on('pageerror', error => errors.push(error.message))
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__hmr_test`)
  await page.getByLabel('Before').fill('preserved draft')
  const identity = await page.evaluate(() => window.pageIdentity)
  await writeFile(join(root, 'ui/src/App.vue'), source('After'))
  await page.getByLabel('After').waitFor()
  assert.equal(await page.getByLabel('After').inputValue(), 'preserved draft')
  assert.equal(await page.evaluate(() => window.pageIdentity), identity)
  assert.deepEqual(errors, [])
  console.log('PASS: source modules load; Vue HMR updates without reload and preserves input state')
} finally {
  await browser?.close()
  await server?.close()
  if (oldEnv === undefined) delete process.env.NIUPANEL_DEV_PLUGINS
  else process.env.NIUPANEL_DEV_PLUGINS = oldEnv
  await rm(root, { recursive: true, force: true })
}
