import { readFileSync } from 'node:fs'

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8')
const versionSource = read('niupanel-common/src/version.rs')
const migrationSource = read('migration/src/lib.rs')
const coreCargo = read('niupanel/Cargo.toml')
const launcherCargo = read('niupanel-launcher/Cargo.toml')

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
if (coreVersion !== launcherVersion) {
  throw new Error(`Core version ${coreVersion} and launcher version ${launcherVersion} must match for the 0.8 release protocol`)
}
if (constant('RELEASE_PROTOCOL_VERSION') < 1 || constant('API_CONTRACT_VERSION') < 1) {
  throw new Error('Release protocol and API contract versions must be positive')
}

console.log(
  `Version contract verified: Core ${coreVersion}, schema ${constant('SCHEMA_EPOCH')}.${schemaRevision}, API ${constant('API_CONTRACT_VERSION')}`
)
