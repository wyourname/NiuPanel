import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { createReadStream, readdirSync, statSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const [kind, artifactsRoot, repository, tag, outputPath] = process.argv.slice(2)
if (!['core', 'web'].includes(kind) || !artifactsRoot || !repository || !tag || !outputPath) {
  console.error(
    'Usage: node scripts/generate-update-component.mjs <core|web> <artifacts-root> <owner/repo> <tag> <output>'
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

const archiveManifest = (path, filename) => {
  const entry = execFileSync('tar', ['-tzf', path], { encoding: 'utf8' })
    .split('\n')
    .map((candidate) => candidate.trim())
    .find((candidate) => {
      const normalized = candidate.replace(/^\.\//, '')
      return normalized === filename || normalized.endsWith(`/${filename}`)
    })
  if (!entry) throw new Error(`${path} does not contain ${filename}`)
  return JSON.parse(
    execFileSync('tar', ['-xOzf', path, entry], { encoding: 'utf8' })
  )
}

const releaseUrl = `https://github.com/${repository}/releases/tag/${tag}`
const downloadUrl = (name) => `https://github.com/${repository}/releases/download/${tag}/${name}`

if (kind === 'core') {
  const targets = [
    ['x86_64', 'niupanel_linux_x86_64.tar.gz', 'x86_64-unknown-linux-musl'],
    ['aarch64', 'niupanel_linux_aarch64.tar.gz', 'aarch64-unknown-linux-musl'],
    ['armv7', 'niupanel_linux_armv7.tar.gz', 'armv7-unknown-linux-musleabihf']
  ]
  const assets = {}
  let canonical
  for (const [architecture, name, target] of targets) {
    const path = join(artifactsRoot, name)
    const manifest = archiveManifest(path, 'core-release.json')
    if (manifest.component !== 'core' || manifest.target !== target) {
      throw new Error(`${name} has an invalid Core manifest`)
    }
    const contract = JSON.stringify({
      version: manifest.version,
      launcher_protocol: manifest.launcher_protocol,
      api_contract: manifest.api_contract,
      schema_epoch: manifest.schema_epoch,
      schema_revision: manifest.schema_revision
    })
    if (canonical && canonical.contract !== contract) {
      throw new Error(`${name} does not match the other Core archives`)
    }
    canonical ??= { manifest, contract }
    assets[architecture] = {
      name,
      url: downloadUrl(name),
      target,
      sha256: await hashFile(path),
      size: statSync(path).size
    }
  }
  if (tag !== `Core-v${canonical.manifest.version}`) {
    throw new Error(`Core tag ${tag} does not match ${canonical.manifest.version}`)
  }
  writeFileSync(
    outputPath,
    `${JSON.stringify(
      {
        version: canonical.manifest.version,
        tag,
        release_url: releaseUrl,
        notes: '',
        launcher_protocol: canonical.manifest.launcher_protocol,
        api_contract: canonical.manifest.api_contract,
        schema_epoch: canonical.manifest.schema_epoch,
        schema_revision: canonical.manifest.schema_revision,
        assets
      },
      null,
      2
    )}\n`
  )
} else {
  const archives = readdirSync(artifactsRoot).filter((name) =>
    /^niupanel_web_.+\.tar\.gz$/.test(name)
  )
  if (archives.length !== 1) {
    throw new Error(`Expected exactly one Web archive, found ${archives.length}`)
  }
  const [name] = archives
  const path = join(artifactsRoot, name)
  const manifest = archiveManifest(path, 'release-manifest.json')
  if (manifest.component !== 'web' || name !== `niupanel_web_${manifest.version}.tar.gz`) {
    throw new Error(`${name} has an invalid Web manifest`)
  }
  if (tag !== `web-v${manifest.version}`) {
    throw new Error(`Web tag ${tag} does not match ${manifest.version}`)
  }
  writeFileSync(
    outputPath,
    `${JSON.stringify(
      {
        version: manifest.version,
        tag,
        release_url: releaseUrl,
        notes: '',
        api_contract: manifest.api_contract,
        core: manifest.core,
        asset: {
          name,
          url: downloadUrl(name),
          sha256: await hashFile(path),
          size: statSync(path).size
        }
      },
      null,
      2
    )}\n`
  )
}
