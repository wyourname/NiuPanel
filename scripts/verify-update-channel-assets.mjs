import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { createReadStream, existsSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'

const [indexPath, artifactsRoot] = process.argv.slice(2)
if (!indexPath || !artifactsRoot) {
  console.error('Usage: node scripts/verify-update-channel-assets.mjs <index.json> <assets-dir>')
  process.exit(1)
}

const index = JSON.parse(readFileSync(indexPath, 'utf8'))
const hashFile = (path) =>
  new Promise((resolve, reject) => {
    const hash = createHash('sha256')
    const stream = createReadStream(path)
    stream.on('error', reject)
    stream.on('data', (chunk) => hash.update(chunk))
    stream.on('end', () => resolve(hash.digest('hex')))
  })

const archiveManifest = (path, filename) => {
  const entry = execFileSync('tar', ['-tzf', path], { encoding: 'utf8' })
    .split('\n')
    .map((candidate) => candidate.trim())
    .find((candidate) => {
      const normalized = candidate.replace(/^\.\//, '')
      return normalized === filename || normalized.endsWith(`/${filename}`)
    })
  if (!entry) throw new Error(`${path} does not contain ${filename}`)
  return JSON.parse(execFileSync('tar', ['-xOzf', path, entry], { encoding: 'utf8' }))
}

const verifyAsset = async (asset) => {
  const path = join(artifactsRoot, asset.name)
  if (!existsSync(path)) throw new Error(`Missing ${asset.name}`)
  if (statSync(path).size !== asset.size) throw new Error(`${asset.name} size mismatch`)
  if ((await hashFile(path)).toLowerCase() !== asset.sha256.toLowerCase()) {
    throw new Error(`${asset.name} checksum mismatch`)
  }
  return path
}

for (const [architecture, asset] of Object.entries(index.core.assets || {})) {
  const path = await verifyAsset(asset)
  const manifest = archiveManifest(path, 'core-release.json')
  if (
    manifest.component !== 'core' ||
    manifest.version !== index.core.version ||
    manifest.target !== asset.target ||
    manifest.launcher_protocol !== index.core.launcher_protocol ||
    manifest.api_contract !== index.core.api_contract ||
    manifest.schema_epoch !== index.core.schema_epoch ||
    manifest.schema_revision !== index.core.schema_revision
  ) {
    throw new Error(`Core ${architecture} archive does not match the update index`)
  }
}

const webPath = await verifyAsset(index.web.asset)
const webManifest = archiveManifest(webPath, 'release-manifest.json')
if (
  webManifest.component !== 'web' ||
  webManifest.version !== index.web.version ||
  webManifest.api_contract !== index.web.api_contract ||
  webManifest.core?.min !== index.web.core?.min ||
  (webManifest.core?.max || null) !== (index.web.core?.max || null)
) {
  throw new Error('Web archive does not match the update index')
}
