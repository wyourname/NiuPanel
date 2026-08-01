import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { createReadStream, existsSync, readFileSync, statSync } from 'node:fs'
import { basename, join } from 'node:path'

const args = process.argv.slice(2)
const manifestPath = args.shift()
const assetsRoot = args.shift()
let expectedVersion
let expectedGitSha
let requireAllCoreAssets = false

while (args.length > 0) {
  const option = args.shift()
  if (option === '--expected-version') {
    expectedVersion = args.shift()?.replace(/^v/, '')
  } else if (option === '--expected-git-sha') {
    expectedGitSha = args.shift()
  } else if (option === '--require-all-core-assets') {
    requireAllCoreAssets = true
  } else {
    throw new Error(`Unknown option: ${option}`)
  }
}

if (!manifestPath || !assetsRoot) {
  console.error(
    'Usage: node scripts/verify-release-bundle.mjs <manifest> <assets-root> [--expected-version <version>] [--expected-git-sha <sha>] [--require-all-core-assets]'
  )
  process.exit(1)
}

const hashFile = (path) =>
  new Promise((resolve, reject) => {
    const hash = createHash('sha256')
    const stream = createReadStream(path)
    stream.on('error', reject)
    stream.on('data', (chunk) => hash.update(chunk))
    stream.on('end', () => resolve(hash.digest('hex')))
  })

const archiveEntries = (archivePath) =>
  execFileSync('tar', ['-tzf', archivePath], { encoding: 'utf8' })
    .split('\n')
    .map((entry) => entry.trim())
    .filter(Boolean)

const archiveManifest = (archivePath, filename) => {
  const entry = archiveEntries(archivePath).find((candidate) => {
    const normalized = candidate.replace(/^\.\//, '')
    return normalized === filename || normalized.endsWith(`/${filename}`)
  })
  if (!entry) throw new Error(`${basename(archivePath)} does not contain ${filename}`)
  return JSON.parse(
    execFileSync('tar', ['-xOzf', archivePath, entry], { encoding: 'utf8' })
  )
}

const assertPlainVersion = (version, label) => {
  if (!/^\d+\.\d+\.\d+$/.test(version || '')) {
    throw new Error(`${label} must be a plain numeric version, got ${version}`)
  }
}

const assertHash = (hash, label) => {
  if (!/^[a-f0-9]{64}$/i.test(hash || '')) {
    throw new Error(`${label} has an invalid SHA-256`)
  }
}

const versionParts = (version) => {
  const match = version?.match(/^(\d+)\.(\d+)\.(\d+)$/)
  if (!match) throw new Error(`Invalid semantic version: ${version}`)
  return match.slice(1).map(Number)
}

const compareVersions = (left, right) => {
  const leftParts = versionParts(left)
  const rightParts = versionParts(right)
  for (let index = 0; index < leftParts.length; index += 1) {
    if (leftParts[index] !== rightParts[index]) return leftParts[index] - rightParts[index]
  }
  return 0
}

const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'))
if (manifest.schema_version !== 2) {
  throw new Error(`Unsupported release manifest schema ${manifest.schema_version}`)
}
if (Object.hasOwn(manifest, 'channel')) {
  throw new Error('Release manifest must not persist a channel; GitHub Release state is authoritative')
}
assertPlainVersion(manifest.release_version, 'release_version')
if (expectedVersion && manifest.release_version !== expectedVersion) {
  throw new Error(
    `Expected release ${expectedVersion}, got ${manifest.release_version}`
  )
}
if (expectedGitSha && manifest.git_sha !== expectedGitSha) {
  throw new Error(`Expected git SHA ${expectedGitSha}, got ${manifest.git_sha}`)
}
if (!/^[a-f0-9]{40}$/i.test(manifest.git_sha || '')) {
  throw new Error(`Invalid release git SHA: ${manifest.git_sha}`)
}

const core = manifest.components?.core
const web = manifest.components?.web
if (!core || !web) throw new Error('Release manifest must contain Core and Web components')
if (core.version !== manifest.release_version) {
  throw new Error(`Core ${core.version} does not match release ${manifest.release_version}`)
}
if (manifest.api_contract !== core.api_contract || web.api_contract !== core.api_contract) {
  throw new Error('Core, Web and release API contracts do not match')
}

const requiredArchitectures = ['x86_64', 'aarch64', 'armv7']
const coreAssets = core.assets || {}
if (Object.keys(coreAssets).length === 0) throw new Error('Release has no Core assets')
if (
  requireAllCoreAssets &&
  requiredArchitectures.some((architecture) => !coreAssets[architecture])
) {
  throw new Error('Release must contain x86_64, aarch64 and armv7 Core assets')
}

const verifyAsset = async (asset, label) => {
  if (!asset?.name || basename(asset.name) !== asset.name) {
    throw new Error(`${label} has an unsafe asset name`)
  }
  assertHash(asset.sha256, label)
  if (!Number.isSafeInteger(asset.size) || asset.size <= 0) {
    throw new Error(`${label} has an invalid size`)
  }
  const path = join(assetsRoot, asset.name)
  if (!existsSync(path)) throw new Error(`Missing ${asset.name}`)
  const size = statSync(path).size
  if (size !== asset.size) {
    throw new Error(`${asset.name} size mismatch: expected ${asset.size}, got ${size}`)
  }
  const digest = await hashFile(path)
  if (digest.toLowerCase() !== asset.sha256.toLowerCase()) {
    throw new Error(`${asset.name} checksum does not match niupanel-release.json`)
  }
  return path
}

for (const [architecture, asset] of Object.entries(coreAssets)) {
  const path = await verifyAsset(asset, `Core ${architecture}`)
  const component = archiveManifest(path, 'core-release.json')
  if (
    component.component !== 'core' ||
    component.version !== core.version ||
    component.target !== asset.target ||
    component.launcher_protocol !== core.launcher_protocol ||
    component.api_contract !== core.api_contract ||
    component.schema_epoch !== core.schema_epoch ||
    component.schema_revision !== core.schema_revision
  ) {
    throw new Error(`${asset.name} does not match the release Core contract`)
  }
}

const webPath = await verifyAsset(web.asset, 'Web')
const webComponent = archiveManifest(webPath, 'release-manifest.json')
if (
  webComponent.component !== 'web' ||
  webComponent.version !== web.version ||
  webComponent.api_contract !== web.api_contract ||
  webComponent.core?.min !== web.core?.min ||
  (webComponent.core?.max || null) !== (web.core?.max || null)
) {
  throw new Error(`${web.asset.name} does not match the release Web contract`)
}
if (compareVersions(core.version, web.core.min) < 0) {
  throw new Error(`Core ${core.version} is older than Web minimum ${web.core.min}`)
}
if (web.core.max && compareVersions(core.version, web.core.max) > 0) {
  throw new Error(`Core ${core.version} is newer than Web maximum ${web.core.max}`)
}

process.stdout.write(
  `${JSON.stringify({
    release_version: manifest.release_version,
    git_sha: manifest.git_sha,
    core_version: core.version,
    web_version: web.version,
    web_asset: web.asset.name
  })}\n`
)
