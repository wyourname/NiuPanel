import { readFileSync } from 'node:fs'

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8')
const versionSource = read('niupanel-common/src/version.rs')
const migrationSource = read('migration/src/lib.rs')
const coreCargo = read('niupanel/Cargo.toml')
const launcherCargo = read('niupanel-launcher/Cargo.toml')
const webPackage = JSON.parse(read('niupanelweb/package.json'))

const constant = (name) => {
  const value = versionSource.match(new RegExp(`pub const ${name}: u32 = (\\d+);`))?.[1]
  if (!value) throw new Error(`Missing ${name} in niupanel-common/src/version.rs`)
  return Number.parseInt(value, 10)
}
const packageVersion = (source, name) => {
  const value = source.match(/^version\s*=\s*"([^"]+)"/m)?.[1]
  if (!value) throw new Error(`Missing package version for ${name}`)
  return value
}

const migrationCount = [...migrationSource.matchAll(/Box::new\(m::/g)].length
const schemaRevision = constant('SCHEMA_REVISION')
if (migrationCount !== schemaRevision) {
  throw new Error(
    `SCHEMA_REVISION=${schemaRevision}, but migration/src/lib.rs registers ${migrationCount} migrations`
  )
}

const coreVersion = packageVersion(coreCargo, 'niupanel')
const launcherVersion = packageVersion(launcherCargo, 'niupanel-launcher')
const releaseTag = process.argv[2]
const stableVersionPattern = /^\d+\.\d+\.\d+$/
if (!stableVersionPattern.test(coreVersion)) {
  throw new Error(
    `Core package version must use a plain numeric version (for example 0.8.1), got ${coreVersion}`
  )
}
if (releaseTag) {
  const coreTag = releaseTag.match(/^core-v(\d+\.\d+\.\d+)$/)
  const webTag = releaseTag.match(/^web-v(\d+\.\d+\.\d+)$/)
  if (coreTag) {
    if (coreTag[1] !== coreVersion) {
      throw new Error(
        `Core tag ${releaseTag} must match the Core package version ${coreVersion}`
      )
    }
  } else if (webTag) {
    if (webTag[1] !== webPackage.version) {
      throw new Error(
        `Web tag ${releaseTag} must match the Web package version ${webPackage.version}`
      )
    }
  } else {
    throw new Error(
      `Release tags must use core-v<major>.<minor>.<patch> or web-v<major>.<minor>.<patch>, got ${releaseTag}`
    )
  }
}
if (constant('RELEASE_PROTOCOL_VERSION') < 1 || constant('API_CONTRACT_VERSION') < 1) {
  throw new Error('Release protocol and API contract versions must be positive')
}

console.log(
  `Version contract verified: ${releaseTag || 'local build'}, Core ${coreVersion}, Launcher ${launcherVersion}, release protocol ${constant('RELEASE_PROTOCOL_VERSION')}, Web ${webPackage.version}, schema ${constant('SCHEMA_EPOCH')}.${schemaRevision}, API ${constant('API_CONTRACT_VERSION')}`
)
