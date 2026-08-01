import { readFileSync, writeFileSync } from 'node:fs'

const [indexPath, channel, component, componentPath] = process.argv.slice(2)

if (!indexPath || !channel || !component || !componentPath) {
  console.error(
    'Usage: node scripts/update-channel-index.mjs <index-path> <preview|stable> <core|web> <component-json>'
  )
  process.exit(1)
}

if (!['preview', 'stable'].includes(channel)) {
  throw new Error(`Unsupported update channel: ${channel}`)
}
if (!['core', 'web'].includes(component)) {
  throw new Error(`Unsupported component: ${component}`)
}

const componentValue = JSON.parse(readFileSync(componentPath, 'utf8'))
let current = {}
try {
  current = JSON.parse(readFileSync(indexPath, 'utf8'))
} catch (error) {
  if (error?.code !== 'ENOENT') throw error
}

const next = {
  ...current,
  schema_version: 1,
  channel,
  updated_at: new Date().toISOString(),
  [component]: componentValue
}

if (!next.core || !next.web) {
  throw new Error(
    `Cannot create ${channel}.json until both Core and Web components are available`
  )
}

if (next.channel !== channel || next.schema_version !== 1) {
  throw new Error('Existing update index has an incompatible schema or channel')
}

const versionParts = (value) => {
  const match = String(value).match(/^(\d+)\.(\d+)\.(\d+)$/)
  if (!match) throw new Error(`Expected plain numeric version, got ${value}`)
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

const core = next.core
const web = next.web
if (!/^core-v\d+\.\d+\.\d+$/.test(core.tag || '')) {
  throw new Error(`Invalid Core tag: ${core.tag}`)
}
if (!/^web-v\d+\.\d+\.\d+$/.test(web.tag || '')) {
  throw new Error(`Invalid Web tag: ${web.tag}`)
}
if (core.tag !== `core-v${core.version}` || web.tag !== `web-v${web.version}`) {
  throw new Error('Component tag and component version do not match')
}
if (compareVersions(core.version, '0.8.0') < 0) {
  throw new Error(`Core ${core.version} is below the minimum supported update baseline 0.8.0`)
}
if (core.api_contract !== web.api_contract) {
  throw new Error('Core and Web API contracts do not match')
}
if (compareVersions(core.version, web.core?.min) < 0) {
  throw new Error(`Core ${core.version} is older than Web minimum ${web.core?.min}`)
}
if (web.core?.max && compareVersions(core.version, web.core.max) > 0) {
  throw new Error(`Core ${core.version} is newer than Web maximum ${web.core.max}`)
}
for (const architecture of ['x86_64', 'aarch64', 'armv7']) {
  const asset = core.assets?.[architecture]
  if (!asset?.name || !asset?.url || !asset?.sha256 || !asset?.size || !asset?.target) {
    throw new Error(`Core is missing a valid ${architecture} asset`)
  }
}
if (!web.asset?.name || !web.asset?.url || !web.asset?.sha256 || !web.asset?.size) {
  throw new Error('Web is missing a valid release asset')
}

writeFileSync(indexPath, `${JSON.stringify(next, null, 2)}\n`)
