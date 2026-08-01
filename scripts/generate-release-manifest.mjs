import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  createReadStream,
  existsSync,
  readdirSync,
  statSync,
  writeFileSync
} from 'node:fs'
import { join } from 'node:path'

const [artifactsRoot, releaseVersionInput, gitSha, ...tail] = process.argv.slice(2)

let outputPath
let docker
if (tail.length === 1) {
  ;[outputPath] = tail
} else if (tail.length === 4) {
  const [dockerEnvironmentVersionInput, dockerImage, dockerDigest, dockerOutputPath] = tail
  docker = {
    environmentVersion: dockerEnvironmentVersionInput?.replace(/^v/, ''),
    image: dockerImage,
    digest: dockerDigest
  }
  outputPath = dockerOutputPath
}

if (
  !artifactsRoot ||
  !releaseVersionInput ||
  !gitSha ||
  !outputPath ||
  (docker && (!docker.environmentVersion || !docker.image || !docker.digest))
) {
  console.error(
    'Usage: node scripts/generate-release-manifest.mjs <artifacts-root> <release-version> <git-sha> <output> [or <docker-environment-version> <docker-image> <docker-digest> <output>]'
  )
  process.exit(1)
}

const releaseVersion = releaseVersionInput.replace(/^v/, '')
if (!/^\d+\.\d+\.\d+$/.test(releaseVersion)) {
  throw new Error(
    `Release version must be a plain numeric version such as 0.8.1, got ${releaseVersionInput}`
  )
}
const sha256 = (path) =>
  new Promise((resolve, reject) => {
    const hash = createHash('sha256')
    const stream = createReadStream(path)
    stream.on('error', reject)
    stream.on('data', (chunk) => hash.update(chunk))
    stream.on('end', () => resolve(hash.digest('hex')))
  })

const findArtifact = (artifact, name) => {
  const candidates = [join(artifactsRoot, name), join(artifactsRoot, artifact, name)]
  return candidates.find((candidate) => existsSync(candidate))
}

const archiveEntries = (archivePath) =>
  execFileSync('tar', ['-tzf', archivePath], {
    encoding: 'utf8'
  })
    .split('\n')
    .map((entry) => entry.trim())
    .filter(Boolean)

const archiveManifest = (archivePath, filename) => {
  const entries = archiveEntries(archivePath)
  const entry = entries.find((candidate) => {
    const normalized = candidate.replace(/^\.\//, '')
    return normalized === filename || normalized.endsWith(`/${filename}`)
  })
  if (!entry) {
    throw new Error(`${archivePath} does not contain ${filename}`)
  }
  const content = execFileSync('tar', ['-xOzf', archivePath, entry], {
    encoding: 'utf8'
  })
  return JSON.parse(content)
}

const normalizedArchiveEntries = (archivePath) =>
  archiveEntries(archivePath).map((entry) => entry.replace(/^\.\//, ''))

const assertCoreArchiveBoundary = (archivePath) => {
  const entries = normalizedArchiveEntries(archivePath)
  if (entries.some((entry) => entry === 'web' || entry.startsWith('web/'))) {
    throw new Error(
      `${archivePath} embeds a Web release; Core and Web must be published as separate update packages`
    )
  }
  if (entries.some((entry) => entry === 'release-manifest.json')) {
    throw new Error(`${archivePath} contains a Web release manifest`)
  }
}

const assertWebArchiveBoundary = (archivePath) => {
  const entries = normalizedArchiveEntries(archivePath)
  const forbidden = new Set(['niupanel', 'niupanel-launcher', 'core-release.json'])
  if (
    entries.some(
      (entry) =>
        forbidden.has(entry) || entry === 'tools' || entry.startsWith('tools/')
    )
  ) {
    throw new Error(
      `${archivePath} embeds Core files; Core and Web must be published as separate update packages`
    )
  }
}

const versionParts = (version) => {
  const match = version.match(/^(\d+)\.(\d+)\.(\d+)(?:[-+].*)?$/)
  if (!match) throw new Error(`Invalid semantic version: ${version}`)
  return match.slice(1).map(Number)
}

const compareVersions = (left, right) => {
  const leftParts = versionParts(left)
  const rightParts = versionParts(right)
  for (let index = 0; index < leftParts.length; index += 1) {
    if (leftParts[index] !== rightParts[index]) {
      return leftParts[index] - rightParts[index]
    }
  }
  return 0
}

const coreTargets = [
  {
    arch: 'x86_64',
    artifact: 'niupanel_linux_x86_64',
    target: 'x86_64-unknown-linux-musl'
  },
  {
    arch: 'aarch64',
    artifact: 'niupanel_linux_aarch64',
    target: 'aarch64-unknown-linux-musl'
  },
  {
    arch: 'armv7',
    artifact: 'niupanel_linux_armv7',
    target: 'armv7-unknown-linux-musleabihf'
  }
]

const coreAssets = {}
let canonicalCore
const requireAllCoreAssets = process.env.NIUPANEL_REQUIRE_ALL_CORE_ASSETS === '1'
for (const target of coreTargets) {
  const name = `${target.artifact}.tar.gz`
  const path = findArtifact(target.artifact, name)
  if (!path) {
    if (requireAllCoreAssets) throw new Error(`Missing release asset ${name}`)
    continue
  }
  assertCoreArchiveBoundary(path)
  const manifest = archiveManifest(path, 'core-release.json')
  if (manifest.component !== 'core') {
    throw new Error(`${name} has an invalid Core component`)
  }
  if (manifest.version !== releaseVersion) {
    throw new Error(
      `${name} contains Core ${manifest.version}, expected ${releaseVersion}`
    )
  }
  const architecturePrefix = target.target.split('-')[0]
  if (
    (requireAllCoreAssets && manifest.target !== target.target) ||
    (!requireAllCoreAssets && !manifest.target.startsWith(`${architecturePrefix}-`))
  ) {
    throw new Error(
      `${name} targets ${manifest.target}, expected ${target.target}`
    )
  }
  const contract = JSON.stringify({
    version: manifest.version,
    launcher_protocol: manifest.launcher_protocol,
    api_contract: manifest.api_contract,
    schema_epoch: manifest.schema_epoch,
    schema_revision: manifest.schema_revision
  })
  if (canonicalCore && contract !== canonicalCore.contract) {
    throw new Error(`${name} does not match the other Core manifests`)
  }
  canonicalCore ??= { manifest, contract }
  coreAssets[target.arch] = {
    name,
    target: manifest.target,
    sha256: await sha256(path),
    size: statSync(path).size
  }
}
if (!canonicalCore) {
  throw new Error('No Core release assets were found')
}

const nestedWebDirectory = join(artifactsRoot, 'niupanel_web')
const webDirectory = existsSync(nestedWebDirectory) ? nestedWebDirectory : artifactsRoot
const webArchives = readdirSync(webDirectory).filter((name) =>
  /^niupanel_web_.+\.tar\.gz$/.test(name)
)
if (webArchives.length !== 1) {
  throw new Error(
    `Expected exactly one Web release archive in ${webDirectory}, found ${webArchives.length}`
  )
}
const [webArchiveName] = webArchives
const webArchivePath = join(webDirectory, webArchiveName)
assertWebArchiveBoundary(webArchivePath)
const webManifest = archiveManifest(webArchivePath, 'release-manifest.json')
if (webManifest.component !== 'web') {
  throw new Error(`${webArchiveName} has an invalid Web component`)
}
if (webArchiveName !== `niupanel_web_${webManifest.version}.tar.gz`) {
  throw new Error(
    `${webArchiveName} does not match Web version ${webManifest.version}`
  )
}
if (webManifest.api_contract !== canonicalCore.manifest.api_contract) {
  throw new Error(
    `API contract mismatch: Core ${canonicalCore.manifest.api_contract}, Web ${webManifest.api_contract}`
  )
}
if (compareVersions(releaseVersion, webManifest.core.min) < 0) {
  throw new Error(
    `Core ${releaseVersion} is older than Web minimum ${webManifest.core.min}`
  )
}
if (
  webManifest.core.max &&
  compareVersions(releaseVersion, webManifest.core.max) > 0
) {
  throw new Error(
    `Core ${releaseVersion} is newer than Web maximum ${webManifest.core.max}`
  )
}
if (docker) {
  if (!/^sha256:[a-f0-9]{64}$/.test(docker.digest)) {
    throw new Error(`Invalid Docker digest: ${docker.digest}`)
  }
  if (!docker.image.endsWith(`:${docker.environmentVersion}`)) {
    throw new Error(
      `Docker image ${docker.image} must use the environment tag ${docker.environmentVersion}`
    )
  }
}

const sourceDateEpoch = Number.parseInt(process.env.SOURCE_DATE_EPOCH || '', 10)
const generatedAt = Number.isFinite(sourceDateEpoch)
  ? new Date(sourceDateEpoch * 1000).toISOString()
  : new Date().toISOString()
const core = canonicalCore.manifest
const release = {
  schema_version: 2,
  release_version: releaseVersion,
  git_sha: gitSha,
  generated_at: generatedAt,
  api_contract: core.api_contract,
  components: {
    core: {
      version: core.version,
      launcher_protocol: core.launcher_protocol,
      api_contract: core.api_contract,
      schema_epoch: core.schema_epoch,
      schema_revision: core.schema_revision,
      assets: coreAssets
    },
    web: {
      version: webManifest.version,
      api_contract: webManifest.api_contract,
      core: webManifest.core,
      asset: {
        name: webArchiveName,
        sha256: await sha256(webArchivePath),
        size: statSync(webArchivePath).size
      }
    }
  }
}

if (docker) {
  release.components.docker = {
    environment_version: docker.environmentVersion,
    bundled_core_version: releaseVersion,
    image: docker.image,
    digest: docker.digest
  }
}

writeFileSync(outputPath, `${JSON.stringify(release, null, 2)}\n`)
console.log(
  `Release manifest generated: ${releaseVersion} (Core ${core.version}, Web ${webManifest.version})`
)
