import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const [indexPath, expectedChannel] = process.argv.slice(2)
if (!indexPath || !expectedChannel) {
  console.error('Usage: node scripts/verify-update-channel-index.mjs <index-path> <preview|stable>')
  process.exit(1)
}

const index = JSON.parse(readFileSync(indexPath, 'utf8'))
if (index.schema_version !== 1 || index.channel !== expectedChannel) {
  throw new Error('Invalid update channel index schema or channel')
}

const componentPath = fileURLToPath(new URL('./update-channel-index.mjs', import.meta.url))
const { spawnSync } = await import('node:child_process')
const { mkdtempSync, writeFileSync, rmSync } = await import('node:fs')
const { tmpdir } = await import('node:os')
const { join } = await import('node:path')
const temp = mkdtempSync(join(tmpdir(), 'niupanel-update-index-'))
try {
  writeFileSync(join(temp, 'index.json'), JSON.stringify(index))
  writeFileSync(join(temp, 'core.json'), JSON.stringify(index.core))
  const result = spawnSync(
    process.execPath,
    [componentPath, join(temp, 'index.json'), expectedChannel, 'core', join(temp, 'core.json')],
    { encoding: 'utf8' }
  )
  if (result.status !== 0) throw new Error(result.stderr || result.stdout)
} finally {
  rmSync(temp, { recursive: true, force: true })
}
