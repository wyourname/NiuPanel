import { readFileSync } from 'node:fs'

const [indexPath, expectedChannel] = process.argv.slice(2)
if (!indexPath || !['preview', 'stable'].includes(expectedChannel)) {
  console.error('Usage: node scripts/verify-update-channel-index.mjs <index-path> <preview|stable>')
  process.exit(1)
}

const index = JSON.parse(readFileSync(indexPath, 'utf8'))
if (index.schema_version !== 2 || index.channel !== expectedChannel || !index.release) {
  throw new Error('Invalid Panel update index schema or channel')
}
if (!Number.isFinite(Date.parse(index.updated_at)) || new Date(index.updated_at).toISOString() !== index.updated_at) {
  throw new Error('Panel update index timestamp must use canonical ISO-8601')
}

const release = index.release
const parseSemver = (value, label, allowPrerelease = true) => {
  const raw = String(value)
  const separator = raw.indexOf('-')
  const base = separator === -1 ? raw : raw.slice(0, separator)
  const prerelease = separator === -1 ? undefined : raw.slice(separator + 1)
  const numbers = base.split('.')
  if (
    numbers.length !== 3 ||
    numbers.some((part) => !/^(0|[1-9]\d*)$/.test(part)) ||
    (!allowPrerelease && prerelease !== undefined) ||
    (prerelease !== undefined && (
      !prerelease ||
      prerelease.split('.').some((part) =>
        !/^[0-9A-Za-z-]+$/.test(part) || (/^\d+$/.test(part) && !/^(0|[1-9]\d*)$/.test(part))
      )
    ))
  ) {
    throw new Error(`${label} is not valid semantic version: ${value}`)
  }
  return numbers.map(Number)
}
parseSemver(release.version, 'Panel version')
if (release.tag !== `v${release.version}`) {
  throw new Error(`Invalid Panel release identity: ${release.tag || release.version}`)
}
if (expectedChannel === 'stable' && release.version.includes('-')) {
  throw new Error('Stable channel cannot point to a prerelease Panel version')
}
if (!String(release.release_url || '').startsWith('https://')) {
  throw new Error('Panel release URL must use HTTPS')
}
if (release.release_url !== `https://${new URL(release.release_url).host}${new URL(release.release_url).pathname}` ||
    !release.release_url.endsWith(`/releases/tag/${release.tag}`) ||
    !Number.isSafeInteger(release.launcher_protocol) || release.launcher_protocol < 1 ||
    typeof release.notes !== 'string') {
  throw new Error('Panel release metadata is invalid')
}

const core = release.core
const web = release.web
if (!core || !web) throw new Error('Panel release must contain Core and Web descriptors')
if (!/^Core-v\d+\.\d+\.\d+$/.test(core.tag || '') || core.tag !== `Core-v${core.version}`) {
  throw new Error(`Invalid Core release identity: ${core.tag || core.version}`)
}
if (!/^web-v\d+\.\d+\.\d+$/.test(web.tag || '') || web.tag !== `web-v${web.version}`) {
  throw new Error(`Invalid Web release identity: ${web.tag || web.version}`)
}
if (release.launcher_protocol !== core.launcher_protocol) {
  throw new Error('Panel and Core launcher protocols do not match')
}
if (core.api_contract !== web.api_contract) {
  throw new Error('Core and Web API contracts do not match')
}
for (const [component, label] of [[core, 'Core'], [web, 'Web']]) {
  if (!String(component.release_url || '').startsWith('https://') ||
      !component.release_url.endsWith(`/releases/tag/${component.tag}`) ||
      typeof component.notes !== 'string') {
    throw new Error(`${label} release metadata is invalid`)
  }
}
for (const [value, label] of [
  [core.launcher_protocol, 'Core launcher protocol'],
  [core.api_contract, 'Core API contract'],
  [core.schema_epoch, 'Core schema epoch'],
  [core.schema_revision, 'Core schema revision'],
  [web.api_contract, 'Web API contract']
]) {
  if (!Number.isSafeInteger(value) || value < 1) throw new Error(`${label} is invalid`)
}

const parsePlainVersion = (value, label) => {
  return parseSemver(value, label, false)
}
const compareVersions = (left, right) => {
  const a = parsePlainVersion(left, 'Version')
  const b = parsePlainVersion(right, 'Version')
  for (let index = 0; index < a.length; index += 1) {
    if (a[index] !== b[index]) return a[index] - b[index]
  }
  return 0
}
if (compareVersions(core.version, '0.8.0') < 0) {
  throw new Error(`Core ${core.version} is below the minimum update baseline 0.8.0`)
}
parsePlainVersion(web.version, 'Web version')
if (compareVersions(core.version, web.core?.min) < 0) {
  throw new Error(`Core ${core.version} is older than Web minimum ${web.core?.min}`)
}
if (web.core?.max && compareVersions(core.version, web.core.max) > 0) {
  throw new Error(`Core ${core.version} is newer than Web maximum ${web.core.max}`)
}

const validateAsset = (asset, label) => {
  if (
    !asset?.name ||
    !String(asset.url || '').startsWith('https://') ||
    !/^[a-f0-9]{64}$/i.test(asset.sha256 || '') ||
    !Number.isSafeInteger(asset.size) ||
    asset.size <= 0
  ) {
    throw new Error(`${label} asset descriptor is invalid`)
  }
}
for (const [architecture, target] of [
  ['x86_64', 'x86_64-unknown-linux-musl'],
  ['aarch64', 'aarch64-unknown-linux-musl'],
  ['armv7', 'armv7-unknown-linux-musleabihf']
]) {
  const asset = core.assets?.[architecture]
  validateAsset(asset, `Core ${architecture}`)
  if (asset.name !== `niupanel_linux_${architecture}.tar.gz`) {
    throw new Error(`Core ${architecture} asset has an unexpected name: ${asset.name}`)
  }
  if (asset.target !== target) throw new Error(`Core ${architecture} target is invalid`)
  if (asset.url !== core.release_url.replace('/releases/tag/', '/releases/download/') + `/${asset.name}`) {
    throw new Error(`Core ${architecture} asset URL does not match its immutable release`)
  }
}
validateAsset(web.asset, 'Web')
if (web.asset.name !== `niupanel_web_${web.version}.tar.gz`) {
  throw new Error(`Web asset has an unexpected name: ${web.asset.name}`)
}
if (web.asset.url !== web.release_url.replace('/releases/tag/', '/releases/download/') + `/${web.asset.name}`) {
  throw new Error('Web asset URL does not match its immutable release')
}

console.log(
  `Verified ${expectedChannel} Panel ${release.version} (Core ${core.version}, Web ${web.version})`
)
