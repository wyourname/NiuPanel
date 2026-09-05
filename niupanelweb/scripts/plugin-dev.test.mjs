import { test } from 'node:test'
import assert from 'node:assert/strict'
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { pluginDevelopment, readPluginDirectories } from './plugin-dev.mjs'

test('development overrides validate directories and reject duplicate ids', () => {
  const root = mkdtempSync(join(tmpdir(), 'niupanel-dev-'))
  try {
    mkdirSync(join(root, 'ui/src'), { recursive: true })
    writeFileSync(join(root, 'plugin.json'), JSON.stringify({ id: 'test-plugin' }))
    writeFileSync(join(root, 'ui/src/plugin.ts'), 'export default {}')
    assert.equal(readPluginDirectories(JSON.stringify([root]))[0].id, 'test-plugin')
    assert.throws(() => readPluginDirectories(JSON.stringify([root, root])), /Duplicate/)
    assert.throws(() => readPluginDirectories('{}'), /JSON array/)
    const previous = process.env.NIUPANEL_DEV_PLUGINS
    try {
      process.env.NIUPANEL_DEV_PLUGINS = JSON.stringify([root])
      const dev = pluginDevelopment()
      const config = dev.config({}, { command: 'serve' })
      assert.ok(config.server.fs.allow.includes(join(root, 'ui')))
      const source = dev.load(dev.resolveId('virtual:niupanel-plugin-dev'))
      assert.match(source, /test-plugin/)
      assert.match(source, /import\(/)
      process.env.NIUPANEL_DEV_PLUGINS = 'invalid-json'
      const production = pluginDevelopment()
      production.config({}, { command: 'build' })
      assert.equal(production.load(production.resolveId('virtual:niupanel-plugin-dev')), 'export default {}')
    } finally {
      if (previous === undefined) delete process.env.NIUPANEL_DEV_PLUGINS
      else process.env.NIUPANEL_DEV_PLUGINS = previous
    }
  } finally { rmSync(root, { recursive: true, force: true }) }
})
