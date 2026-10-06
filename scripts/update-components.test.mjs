import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'

const scripts = dirname(fileURLToPath(import.meta.url))
const run = (script, ...args) => execFileSync(process.execPath, [join(scripts, script), ...args], { stdio: 'pipe' })

for (const legacyArm of [false, true]) {
  test(`release pipeline accepts ${legacyArm ? 'legacy ARMv7' : 'pnpm 12 two-architecture'} assets`, () => {
    const root = mkdtempSync(join(tmpdir(), 'niupanel-components-'))
    try {
      const stage = join(root, 'stage')
      mkdirSync(stage)
      const architectures = ['x86_64', 'aarch64', ...(legacyArm ? ['armv7'] : [])]
      for (const architecture of architectures) {
        writeFileSync(join(stage, 'core-release.json'), JSON.stringify({
          component: 'core', version: '0.8.5', launcher_protocol: 1, api_contract: 1,
          schema_epoch: 1, schema_revision: 1,
          target: architecture === 'armv7' ? 'armv7-unknown-linux-musleabihf' : `${architecture}-unknown-linux-musl`,
        }))
        execFileSync('tar', ['-czf', join(root, `niupanel_linux_${architecture}.tar.gz`), '-C', stage, 'core-release.json'])
      }
      writeFileSync(join(stage, 'release-manifest.json'), JSON.stringify({
        component: 'web', version: '2.0.5', api_contract: 1, core: { min: '0.8.0' },
      }))
      execFileSync('tar', ['-czf', join(root, 'niupanel_web_2.0.5.tar.gz'), '-C', stage, 'release-manifest.json'])
      const core = join(root, 'core.json'), web = join(root, 'web.json'), index = join(root, 'index.json')
      run('generate-update-component.mjs', 'core', root, 'example/panel', 'Core-v0.8.5', core)
      run('generate-update-component.mjs', 'web', root, 'example/panel', 'web-v2.0.5', web)
      assert.deepEqual(Object.keys(JSON.parse(readFileSync(core)).assets), architectures)
      run('compose-update-channel-index.mjs', 'preview', '1.0.0', core, web, 'example/panel', index)
      run('verify-update-channel-index.mjs', index, 'preview')
      run('verify-update-channel-assets.mjs', index, root)

      // Optional legacy support must still validate any ARMv7 descriptor that is present.
      if (legacyArm) {
        const invalid = JSON.parse(readFileSync(index))
        invalid.release.core.assets.armv7.target = 'incorrect-target'
        writeFileSync(index, JSON.stringify(invalid))
        assert.throws(() => run('verify-update-channel-index.mjs', index, 'preview'))
      }
      rmSync(join(root, 'niupanel_linux_aarch64.tar.gz'))
      assert.throws(() => run('generate-update-component.mjs', 'core', root, 'example/panel', 'Core-v0.8.5', core))
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  })
}
